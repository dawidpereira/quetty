mod connection_handler;
mod namespace_handler;
mod resource_group_handler;
mod service_bus_handler;
mod subscription_handler;

pub use connection_handler::ConnectionHandler;
pub use namespace_handler::NamespaceHandler;
pub use resource_group_handler::ResourceGroupHandler;
pub use service_bus_handler::ServiceBusHandler;
pub use subscription_handler::SubscriptionHandler;

use crate::app::model::Model;
use crate::components::common::{AzureDiscoveryMsg, Msg};
use crate::error::AppError;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Handle subscription selection messages
    pub fn handle_subscription_selection(
        &mut self,
        msg: crate::components::common::SubscriptionSelectionMsg,
    ) -> Option<Msg> {
        use crate::components::common::{ComponentId, SubscriptionSelectionMsg};

        match msg {
            SubscriptionSelectionMsg::SubscriptionSelected(subscription_id) => {
                log::info!("Subscription selected: {subscription_id}");
                // Use atomic update to ensure state consistency
                self.state_manager.update_azure_selection(
                    Some(subscription_id.clone()),
                    None,
                    None,
                );
                if let Err(e) = self.app.umount(&ComponentId::SubscriptionPicker) {
                    log::error!("Failed to unmount subscription picker: {e}");
                }
                Some(Msg::AzureDiscovery(
                    AzureDiscoveryMsg::DiscoveringResourceGroups(subscription_id),
                ))
            }
            SubscriptionSelectionMsg::CancelSelection => {
                log::info!("Subscription selection cancelled - exiting application");

                // Set quit flag first to prevent any further event processing
                self.set_quit(true);

                // Unmount subscription picker
                if let Err(e) = self.app.umount(&ComponentId::SubscriptionPicker) {
                    log::error!("Failed to unmount subscription picker: {e}");
                }

                // Return AppClose to trigger immediate shutdown
                Some(Msg::AppClose)
            }
            SubscriptionSelectionMsg::SelectionChanged => {
                self.state_manager.set_redraw(true);
                None
            }
        }
    }

    /// Handle resource group selection messages
    pub fn handle_resource_group_selection(
        &mut self,
        msg: crate::components::common::ResourceGroupSelectionMsg,
    ) -> Option<Msg> {
        use crate::components::common::{ComponentId, ResourceGroupSelectionMsg};

        match msg {
            ResourceGroupSelectionMsg::ResourceGroupSelected(resource_group) => {
                log::info!("Resource group selected: {resource_group}");
                // Use atomic update to ensure state consistency
                let subscription = self.state_manager.selected_subscription.clone();
                if let Some(sub_id) = subscription.clone() {
                    self.state_manager.update_azure_selection(
                        Some(sub_id.clone()),
                        Some(resource_group),
                        None,
                    );
                }
                if let Err(e) = self.app.umount(&ComponentId::ResourceGroupPicker) {
                    log::error!("Failed to unmount resource group picker: {e}");
                }

                if let Some(subscription_id) = &subscription {
                    Some(Msg::AzureDiscovery(
                        AzureDiscoveryMsg::DiscoveringNamespaces(subscription_id.clone()),
                    ))
                } else {
                    Some(Msg::AzureDiscovery(AzureDiscoveryMsg::DiscoveryError(
                        "No subscription selected".to_string(),
                    )))
                }
            }
            ResourceGroupSelectionMsg::CancelSelection => {
                log::info!("Resource group selection cancelled");
                if let Err(e) = self.app.umount(&ComponentId::ResourceGroupPicker) {
                    log::error!("Failed to unmount resource group picker: {e}");
                }
                // Go back to subscription selection
                Some(Msg::AzureDiscovery(
                    AzureDiscoveryMsg::DiscoveringSubscriptions,
                ))
            }
            ResourceGroupSelectionMsg::SelectionChanged => {
                self.state_manager.set_redraw(true);
                None
            }
        }
    }

    /// Main Azure discovery handler - now delegates to focused handlers
    pub fn handle_azure_discovery(&mut self, msg: AzureDiscoveryMsg) -> Option<Msg> {
        match msg {
            AzureDiscoveryMsg::StartDiscovery => {
                log::info!("Starting Azure resource discovery");
                self.start_azure_discovery()
            }
            AzureDiscoveryMsg::DiscoveringSubscriptions => {
                log::info!("Discovering Azure subscriptions");
                SubscriptionHandler::discover(self)
            }
            AzureDiscoveryMsg::SubscriptionsDiscovered(subscriptions) => {
                let count = subscriptions.len();
                log::info!("Discovered {count} subscriptions");
                SubscriptionHandler::handle_discovered(self, subscriptions)
            }
            AzureDiscoveryMsg::DiscoveringResourceGroups(subscription_id) => {
                log::info!("Discovering resource groups for subscription: {subscription_id}");
                ResourceGroupHandler::discover(self, subscription_id)
            }
            AzureDiscoveryMsg::ResourceGroupsDiscovered(groups) => {
                let count = groups.len();
                log::info!("Discovered {count} resource groups");
                ResourceGroupHandler::handle_discovered(self, groups)
            }
            AzureDiscoveryMsg::DiscoveringNamespaces(subscription_id) => {
                log::info!(
                    "Discovering Service Bus namespaces for subscription: {subscription_id}"
                );
                NamespaceHandler::discover(self, subscription_id)
            }
            AzureDiscoveryMsg::NamespacesDiscovered(namespaces) => {
                let count = namespaces.len();
                log::info!("Discovered {count} Service Bus namespaces");
                NamespaceHandler::handle_discovered(self, namespaces)
            }
            AzureDiscoveryMsg::FetchingConnectionString {
                subscription_id,
                resource_group,
                namespace,
            } => {
                log::info!("Fetching connection string for namespace: {namespace}");
                ConnectionHandler::fetch(self, subscription_id, resource_group, namespace)
            }
            AzureDiscoveryMsg::ConnectionStringFetched(connection_string) => {
                log::info!("Successfully fetched connection string");
                ConnectionHandler::handle_fetched(self, connection_string)
            }
            AzureDiscoveryMsg::ServiceBusManagerCreated => {
                log::info!("Service Bus manager created with discovered connection string");
                ServiceBusHandler::handle_created(self)
            }
            AzureDiscoveryMsg::ServiceBusManagerReady(manager) => {
                log::info!("Service Bus manager ready, storing it");
                ServiceBusHandler::handle_ready(self, manager)
            }
            AzureDiscoveryMsg::DiscoveryError(error) => {
                log::error!("Azure discovery error: {error}");
                self.mount_error_popup(&AppError::ServiceBus(error))
                    .ok()
                    .map(|_| Msg::ForceRedraw)
            }
            AzureDiscoveryMsg::DiscoveryComplete => {
                log::info!("Azure resource discovery completed");
                self.finalize_discovery()
            }
        }
    }

    /// Start the Azure discovery process - checks configuration and determines next steps
    fn start_azure_discovery(&mut self) -> Option<Msg> {
        // Reset any previous discovery state
        self.state_manager.reset_azure_discovery_state();

        // Check if we already have a connection string configured
        let config = crate::config::get_config_or_panic();
        if config.servicebus().connection_string().is_some() {
            log::info!("Connection string already configured, skipping discovery");
            return Some(Msg::AzureDiscovery(AzureDiscoveryMsg::DiscoveryComplete));
        }

        let azure_ad_config = config.azure_ad();

        // Check if we have all required Azure AD config to skip discovery
        if let (Ok(subscription_id), Ok(resource_group), Ok(namespace)) = (
            azure_ad_config.subscription_id(),
            azure_ad_config.resource_group(),
            azure_ad_config.namespace(),
        ) {
            log::info!(
                "Azure AD config complete, skipping discovery and fetching connection string directly"
            );
            return Some(Msg::AzureDiscovery(
                AzureDiscoveryMsg::FetchingConnectionString {
                    subscription_id: subscription_id.to_string(),
                    resource_group: resource_group.to_string(),
                    namespace: namespace.to_string(),
                },
            ));
        }

        // Start with subscription discovery
        log::info!("Azure AD config incomplete, starting discovery process");
        Some(Msg::AzureDiscovery(
            AzureDiscoveryMsg::DiscoveringSubscriptions,
        ))
    }

    /// Finalize the discovery process and prepare for queue operations
    fn finalize_discovery(&mut self) -> Option<Msg> {
        log::info!("Finalizing Azure discovery");

        // Clear authentication flag since discovery is complete
        self.state_manager.set_authentication_state(false);

        // If we have a discovered connection string and namespace, we can proceed
        if self.state_manager.discovered_connection_string.is_some() {
            // Unmount any discovery-related pickers
            let _ = self
                .app
                .umount(&crate::components::common::ComponentId::SubscriptionPicker);
            let _ = self
                .app
                .umount(&crate::components::common::ComponentId::ResourceGroupPicker);
            let _ = self
                .app
                .umount(&crate::components::common::ComponentId::NamespacePicker);

            // Discovery complete - ready for queue operations
            log::info!("Discovery complete - ready for queue operations");

            // Now load queues for the selected namespace
            if let (
                Some(subscription_id),
                Some(resource_group),
                Some(namespace),
                Some(auth_service),
            ) = (
                &self.state_manager.selected_subscription,
                &self.state_manager.selected_resource_group,
                &self.state_manager.selected_namespace,
                &self.auth_service,
            ) {
                log::info!("Loading queues for discovered namespace: {namespace}");
                self.queue_manager.load_queues_with_discovery(
                    subscription_id.clone(),
                    resource_group.clone(),
                    namespace.clone(),
                    auth_service.clone(),
                    self.http_client.clone(),
                );
            }
        }

        None
    }
}
