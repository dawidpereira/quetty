use crate::app::model::Model;
use crate::components::common::{AzureDiscoveryMsg, Msg};
use crate::error::AppError;
use quetty_server::service_bus_manager::ServiceBusManager;
use std::sync::Arc;
use tuirealm::terminal::TerminalAdapter;

pub struct ServiceBusHandler;

impl ServiceBusHandler {
    /// Handle Service Bus manager creation
    pub fn handle_created<T>(model: &mut Model<T>) -> Option<Msg>
    where
        T: TerminalAdapter,
    {
        log::info!("Creating Service Bus manager with discovered connection string");

        // Get the discovered connection string
        let connection_string = match &model.state_manager.discovered_connection_string {
            Some(cs) => cs.clone(),
            None => {
                log::error!("No connection string found after discovery");
                return Some(Msg::AzureDiscovery(AzureDiscoveryMsg::DiscoveryError(
                    "No connection string found".to_string(),
                )));
            }
        };

        // Get configuration
        let config = crate::config::get_config_or_panic();
        let mut azure_ad_config = config.azure_ad().clone();

        // Update the azure_ad_config with discovered values
        if let (Some(subscription_id), Some(resource_group), Some(namespace)) = (
            &model.state_manager.selected_subscription,
            &model.state_manager.selected_resource_group,
            &model.state_manager.selected_namespace,
        ) {
            log::info!(
                "Updating Azure AD config with discovered values - subscription: {subscription_id}, resource_group: {resource_group}, namespace: {namespace}"
            );
            azure_ad_config.subscription_id = Some(subscription_id.clone());
            azure_ad_config.resource_group = Some(resource_group.clone());
            azure_ad_config.namespace = Some(namespace.clone());
        } else {
            log::warn!("Missing discovered values for Azure AD config update");
        }

        let statistics_config =
            quetty_server::service_bus_manager::azure_management_client::StatisticsConfig::new(
                config.queue_stats_display_enabled(),
                config.queue_stats_cache_ttl_seconds(),
                config.queue_stats_use_management_api(),
            );
        let batch_config = config.batch().clone();

        // Create the Service Bus client - we'll handle this asynchronously
        let tx = model.tx_to_main().clone();
        let task_manager = model.task_manager.clone();
        let http_client = model.http_client.clone();

        task_manager.execute("Initializing Service Bus client...", async move {
            // Create the Service Bus client
            let client = azservicebus::ServiceBusClient::new_from_connection_string(
                &connection_string,
                azservicebus::ServiceBusClientOptions::default(),
            )
            .await
            .map_err(|e| {
                AppError::ServiceBus(format!("Failed to create Service Bus client: {e}"))
            })?;

            // Create a new service bus manager
            let new_manager = Arc::new(tokio::sync::Mutex::new(ServiceBusManager::new(
                Arc::new(tokio::sync::Mutex::new(client)),
                http_client,
                azure_ad_config,
                statistics_config,
                batch_config,
                connection_string,
            )));

            // Send the manager back to the main thread
            let _ = tx.send(Msg::AzureDiscovery(
                AzureDiscoveryMsg::ServiceBusManagerReady(new_manager.clone()),
            ));

            Ok(())
        });

        None
    }

    /// Handle Service Bus manager ready state
    pub fn handle_ready<T>(
        model: &mut Model<T>,
        manager: Arc<tokio::sync::Mutex<ServiceBusManager>>,
    ) -> Option<Msg>
    where
        T: TerminalAdapter,
    {
        log::info!("Storing Service Bus manager and updating queue manager");

        // Store the manager in the model
        model.service_bus_manager = Some(manager.clone());

        // Update the queue manager with the new service bus manager
        model.queue_manager.set_service_bus_manager(manager);

        // Discovery is complete
        Some(Msg::AzureDiscovery(AzureDiscoveryMsg::DiscoveryComplete))
    }
}
