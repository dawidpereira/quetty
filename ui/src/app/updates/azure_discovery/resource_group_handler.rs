use crate::app::model::{AppState, Model};
use crate::components::common::{AzureDiscoveryMsg, ComponentId, Msg};
use crate::components::resource_group_picker::ResourceGroupPicker;
use crate::error::AppError;
use server::service_bus_manager::azure_management_client::{AzureManagementClient, ResourceGroup};
use tuirealm::terminal::TerminalAdapter;

pub struct ResourceGroupHandler;

impl ResourceGroupHandler {
    /// Handle resource group discovery process
    pub fn discover<T>(model: &mut Model<T>, subscription_id: String) -> Option<Msg>
    where
        T: TerminalAdapter,
    {
        // Check cache first for performance optimization
        if let Some(cached_groups) = model
            .state_manager
            .azure_cache
            .get_cached_resource_groups(&subscription_id)
        {
            log::info!(
                "Using cached resource groups for subscription {} ({} found)",
                subscription_id,
                cached_groups.len()
            );
            return Some(Msg::AzureDiscovery(
                AzureDiscoveryMsg::ResourceGroupsDiscovered(cached_groups),
            ));
        }

        let tx = model.tx_to_main().clone();
        let auth_service = model.auth_service.clone()?;
        let http_client = model.http_client.clone();

        model.task_manager.execute("Processing...", async move {
            log::info!("Fetching resource groups from Azure Management API (not cached)");

            let token = auth_service
                .get_management_token()
                .await
                .map_err(|e| AppError::Auth(e.to_string()))?;

            let client = AzureManagementClient::new(http_client);
            let groups = client
                .list_resource_groups(&token, &subscription_id)
                .await
                .map_err(|e| AppError::ServiceBus(e.to_string()))?;

            let _ = tx.send(Msg::AzureDiscovery(
                AzureDiscoveryMsg::ResourceGroupsDiscovered(groups),
            ));

            Ok(())
        });

        // Use atomic loading state update
        model.state_manager.set_loading_state(
            "Discovering resource groups...".to_string(),
            AppState::AzureDiscovery,
        );
        let _ = model.mount_loading_indicator("Discovering resource groups...");
        None
    }

    /// Handle the result of resource group discovery
    pub fn handle_discovered<T>(model: &mut Model<T>, groups: Vec<ResourceGroup>) -> Option<Msg>
    where
        T: TerminalAdapter,
    {
        // Unmount loading indicator
        if let Err(e) = model.app.umount(&ComponentId::LoadingIndicator) {
            log::warn!("Failed to unmount loading indicator: {e}");
        }

        if groups.is_empty() {
            log::warn!("No resource groups found - this may be due to limited permissions");
            log::info!(
                "Mounting empty resource group picker to allow navigation back to subscription selection"
            );
            // Still mount the picker with empty list to allow proper navigation
            // Users can use ESC to go back to subscription selection
        }

        // Cache resource groups
        if let Some(subscription_id) = &model.state_manager.selected_subscription {
            model
                .state_manager
                .azure_cache
                .cache_resource_groups(subscription_id.clone(), groups.clone());
        }

        // Mount resource group picker
        match model.app.mount(
            ComponentId::ResourceGroupPicker,
            Box::new(ResourceGroupPicker::new(Some(groups))),
            vec![],
        ) {
            Ok(_) => {
                log::info!("Resource group picker mounted successfully");
                // Set the app state to prevent reverting to NamespacePicker
                model.state_manager.app_state = AppState::AzureDiscovery;
                if let Err(e) = model.app.active(&ComponentId::ResourceGroupPicker) {
                    log::error!("Failed to activate resource group picker: {e}");
                }
                model.state_manager.set_redraw(true);
                log::debug!("Redraw flag set after mounting resource group picker");

                // Force an immediate redraw to ensure the picker is visible
                if let Err(e) = model.view() {
                    log::error!(
                        "Failed to force redraw after mounting resource group picker: {e:?}"
                    );
                }

                None
            }
            Err(e) => Some(Msg::AzureDiscovery(AzureDiscoveryMsg::DiscoveryError(
                format!("Failed to show resource group picker: {e}"),
            ))),
        }
    }
}
