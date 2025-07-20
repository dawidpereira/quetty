use crate::app::model::{AppState, Model};
use crate::components::common::{AzureDiscoveryMsg, ComponentId, Msg};
use crate::error::AppError;
use quetty_server::service_bus_manager::azure_management_client::AzureManagementClient;
use tuirealm::terminal::TerminalAdapter;

pub struct ConnectionHandler;

impl ConnectionHandler {
    /// Handle connection string fetching process
    pub fn fetch<T>(
        model: &mut Model<T>,
        subscription_id: String,
        resource_group: String,
        namespace: String,
    ) -> Option<Msg>
    where
        T: TerminalAdapter,
    {
        let tx = model.tx_to_main().clone();
        let auth_service = model.auth_service.clone()?;
        let http_client = model.http_client.clone();

        model.task_manager.execute("Processing...", async move {
            let token = auth_service
                .get_management_token()
                .await
                .map_err(|e| AppError::Auth(e.to_string()))?;

            let client = AzureManagementClient::new(http_client);
            let connection_string = client
                .get_namespace_connection_string(
                    &token,
                    &subscription_id,
                    &resource_group,
                    &namespace,
                )
                .await
                .map_err(|e| AppError::ServiceBus(e.to_string()))?;

            let _ = tx.send(Msg::AzureDiscovery(
                AzureDiscoveryMsg::ConnectionStringFetched(connection_string),
            ));

            Ok(())
        });

        // Use atomic loading state update
        model.state_manager.set_loading_state(
            "Fetching connection string...".to_string(),
            AppState::AzureDiscovery,
        );
        let _ = model.mount_loading_indicator("Fetching connection string...");
        None
    }

    /// Handle the result of connection string fetching
    pub fn handle_fetched<T>(model: &mut Model<T>, connection_string: String) -> Option<Msg>
    where
        T: TerminalAdapter,
    {
        // Unmount loading indicator
        if let Err(e) = model.app.umount(&ComponentId::LoadingIndicator) {
            log::warn!("Failed to unmount loading indicator: {e}");
        }

        // Store the connection string in state
        let namespaces = model.state_manager.discovered_namespaces.clone();
        model
            .state_manager
            .update_discovered_resources(namespaces, Some(connection_string.clone()));

        // Persist discovered configuration to disk for future use
        if let (Some(subscription_id), Some(resource_group), Some(namespace)) = (
            &model.state_manager.selected_subscription,
            &model.state_manager.selected_resource_group,
            &model.state_manager.selected_namespace,
        ) {
            log::info!("Persisting discovered Azure configuration to disk");

            // Get current auth method instead of hardcoding device_code
            let config = crate::config::get_config_or_panic();
            let current_auth_method = config.azure_ad().auth_method.clone();

            let config_data = crate::components::common::ConfigUpdateData {
                auth_method: current_auth_method,
                tenant_id: None,
                client_id: None,
                client_secret: None,
                subscription_id: Some(subscription_id.clone()),
                resource_group: Some(resource_group.clone()),
                namespace: Some(namespace.clone()),
                connection_string: Some(connection_string.clone()),
                master_password: None, // Not needed for Azure AD auth methods
                queue_name: None,      // Not needed for Azure AD auth methods
            };

            // Save to .env file
            if let Err(e) = model.write_env_file(&config_data) {
                log::error!("Failed to persist discovered configuration: {e}");
            } else {
                log::info!("Successfully persisted discovered configuration");
            }
        }

        // Since we have the connection string, we can now create the Service Bus manager
        Some(Msg::AzureDiscovery(
            AzureDiscoveryMsg::ServiceBusManagerCreated,
        ))
    }
}
