use crate::app::model::{AppState, Model};
use crate::components::common::{AzureDiscoveryMsg, ComponentId, Msg};
use crate::components::namespace_picker::NamespacePicker;
use crate::error::AppError;
use server::service_bus_manager::azure_management_client::{
    AzureManagementClient, ServiceBusNamespace,
};
use tuirealm::terminal::TerminalAdapter;
use tuirealm::{Sub, SubClause, SubEventClause};

pub struct NamespaceHandler;

impl NamespaceHandler {
    /// Handle Service Bus namespace discovery process
    pub fn discover<T>(model: &mut Model<T>, subscription_id: String) -> Option<Msg>
    where
        T: TerminalAdapter,
    {
        // Check cache first for performance optimization
        if let Some(cached_namespaces) = model
            .state_manager
            .azure_cache
            .get_cached_namespaces(&subscription_id)
        {
            log::info!(
                "Using cached namespaces for subscription {} ({} found)",
                subscription_id,
                cached_namespaces.len()
            );
            return Some(Msg::AzureDiscovery(
                AzureDiscoveryMsg::NamespacesDiscovered(cached_namespaces),
            ));
        }

        let tx = model.tx_to_main().clone();
        let auth_service = model.auth_service.clone()?;
        let http_client = model.http_client.clone();

        model.task_manager.execute("Processing...", async move {
            log::info!("Fetching namespaces from Azure Management API (not cached)");

            let token = auth_service
                .get_management_token()
                .await
                .map_err(|e| AppError::Auth(e.to_string()))?;

            let client = AzureManagementClient::new(http_client);
            let namespaces = client
                .list_service_bus_namespaces(&token, &subscription_id)
                .await
                .map_err(|e| AppError::ServiceBus(e.to_string()))?;

            let _ = tx.send(Msg::AzureDiscovery(
                AzureDiscoveryMsg::NamespacesDiscovered(namespaces),
            ));

            Ok(())
        });

        // Use atomic loading state update
        model.state_manager.set_loading_state(
            "Discovering Service Bus namespaces...".to_string(),
            AppState::AzureDiscovery,
        );
        let _ = model.mount_loading_indicator("Discovering Service Bus namespaces...");
        None
    }

    /// Handle the result of namespace discovery
    pub fn handle_discovered<T>(
        model: &mut Model<T>,
        namespaces: Vec<ServiceBusNamespace>,
    ) -> Option<Msg>
    where
        T: TerminalAdapter,
    {
        // Unmount loading indicator
        if let Err(e) = model.app.umount(&ComponentId::LoadingIndicator) {
            log::warn!("Failed to unmount loading indicator: {e}");
        }

        if namespaces.is_empty() {
            return Some(Msg::AzureDiscovery(AzureDiscoveryMsg::DiscoveryError(
                "No Service Bus namespaces found. Please create a namespace first.".to_string(),
            )));
        }

        // Cache namespaces
        if let Some(subscription_id) = &model.state_manager.selected_subscription {
            model
                .state_manager
                .azure_cache
                .cache_namespaces(subscription_id.clone(), namespaces.clone());
        }

        // Convert to namespace names for the picker
        let namespace_names: Vec<String> = namespaces.iter().map(|ns| ns.name.clone()).collect();

        // Update existing namespace picker with discovered namespaces
        if model.app.mounted(&ComponentId::NamespacePicker) {
            if let Err(e) = model.app.umount(&ComponentId::NamespacePicker) {
                log::warn!("Failed to unmount existing namespace picker: {e}");
            }
        }

        match model.app.mount(
            ComponentId::NamespacePicker,
            Box::new(NamespacePicker::new(Some(namespace_names))),
            vec![Sub::new(SubEventClause::Any, SubClause::Always)],
        ) {
            Ok(_) => {
                // Keep in AzureDiscovery state for namespace selection
                model.state_manager.app_state = AppState::AzureDiscovery;

                // Activate the namespace picker with proper error handling
                if let Err(e) = model.app.active(&ComponentId::NamespacePicker) {
                    log::error!("Failed to activate namespace picker: {e}");
                    return Some(Msg::AzureDiscovery(AzureDiscoveryMsg::DiscoveryError(
                        format!("Failed to activate namespace picker: {e}"),
                    )));
                }

                model.state_manager.set_redraw(true);

                // Force an immediate redraw to ensure the picker is visible
                if let Err(e) = model.view() {
                    log::error!("Failed to force redraw after mounting namespace picker: {e:?}");
                }

                // Store the full namespace objects for later use
                model
                    .state_manager
                    .update_discovered_resources(namespaces, None);
                None
            }
            Err(e) => Some(Msg::AzureDiscovery(AzureDiscoveryMsg::DiscoveryError(
                format!("Failed to show namespace picker: {e}"),
            ))),
        }
    }
}
