use crate::app::model::{AppState, Model};
use crate::components::common::{AzureDiscoveryMsg, ComponentId, Msg};
use crate::components::subscription_picker::SubscriptionPicker;
use crate::error::AppError;
use server::service_bus_manager::azure_management_client::{AzureManagementClient, Subscription};
use tuirealm::terminal::TerminalAdapter;

pub struct SubscriptionHandler;

impl SubscriptionHandler {
    /// Handle subscription discovery process
    pub fn discover<T>(model: &mut Model<T>) -> Option<Msg>
    where
        T: TerminalAdapter,
    {
        // Check cache first for performance optimization
        if let Some(cached_subscriptions) =
            model.state_manager.azure_cache.get_cached_subscriptions()
        {
            log::info!(
                "Using cached subscriptions ({} found)",
                cached_subscriptions.len()
            );
            return Some(Msg::AzureDiscovery(
                AzureDiscoveryMsg::SubscriptionsDiscovered(cached_subscriptions),
            ));
        }

        let tx = model.tx_to_main().clone();
        let auth_service = model.auth_service.clone()?;
        let http_client = model.http_client.clone();

        model.task_manager.execute("Processing...", async move {
            log::info!("Fetching subscriptions from Azure Management API (not cached)");

            // Get Azure AD token with management scope
            let token = auth_service
                .get_management_token()
                .await
                .map_err(|e| AppError::Auth(e.to_string()))?;

            let client = AzureManagementClient::new(http_client);
            let subscriptions = client
                .list_subscriptions(&token)
                .await
                .map_err(|e| AppError::ServiceBus(e.to_string()))?;

            let _ = tx.send(Msg::AzureDiscovery(
                AzureDiscoveryMsg::SubscriptionsDiscovered(subscriptions),
            ));

            Ok(())
        });

        // Show loading indicator
        model.state_manager.set_loading_state(
            "Discovering Azure subscriptions...".to_string(),
            AppState::AzureDiscovery,
        );
        let _ = model.mount_loading_indicator("Discovering Azure subscriptions...");
        None
    }

    /// Handle the result of subscription discovery
    pub fn handle_discovered<T>(
        model: &mut Model<T>,
        subscriptions: Vec<Subscription>,
    ) -> Option<Msg>
    where
        T: TerminalAdapter,
    {
        // Unmount loading indicator
        if let Err(e) = model.app.umount(&ComponentId::LoadingIndicator) {
            log::warn!("Failed to unmount loading indicator: {e}");
        }

        if subscriptions.is_empty() {
            return Some(Msg::AzureDiscovery(AzureDiscoveryMsg::DiscoveryError(
                "No Azure subscriptions found. Please check your permissions.".to_string(),
            )));
        }

        // Cache subscriptions
        model
            .state_manager
            .azure_cache
            .cache_subscriptions(subscriptions.clone());

        // Mount subscription picker
        match model.app.mount(
            ComponentId::SubscriptionPicker,
            Box::new(SubscriptionPicker::new(Some(subscriptions))),
            vec![],
        ) {
            Ok(_) => {
                log::info!("Subscription picker mounted successfully");
                // Set the app state to prevent reverting to NamespacePicker
                model.state_manager.app_state = AppState::AzureDiscovery;
                if let Err(e) = model.app.active(&ComponentId::SubscriptionPicker) {
                    log::error!("Failed to activate subscription picker: {e}");
                }
                model.state_manager.set_redraw(true);
                log::debug!("Redraw flag set after mounting subscription picker");

                // Force an immediate redraw to ensure the picker is visible
                if let Err(e) = model.view() {
                    log::error!("Failed to force redraw after mounting subscription picker: {e:?}");
                }

                None
            }
            Err(e) => Some(Msg::AzureDiscovery(AzureDiscoveryMsg::DiscoveryError(
                format!("Failed to show subscription picker: {e}"),
            ))),
        }
    }
}