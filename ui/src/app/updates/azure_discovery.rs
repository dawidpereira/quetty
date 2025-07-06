use crate::app::model::{AppState, Model};
use crate::components::common::{
    AzureDiscoveryMsg, ComponentId, Msg, ResourceGroupSelectionMsg, SubscriptionSelectionMsg,
};
use crate::components::namespace_picker::NamespacePicker;
use crate::components::resource_group_picker::ResourceGroupPicker;
use crate::components::subscription_picker::SubscriptionPicker;
use crate::error::AppError;
use server::service_bus_manager::ServiceBusManager;
use server::service_bus_manager::azure_management_client::{
    AzureManagementClient, ResourceGroup, ServiceBusNamespace, Subscription,
};
use std::sync::Arc;
use tuirealm::terminal::TerminalAdapter;
use tuirealm::{Sub, SubClause, SubEventClause};

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn handle_azure_discovery(&mut self, msg: AzureDiscoveryMsg) -> Option<Msg> {
        match msg {
            AzureDiscoveryMsg::StartDiscovery => {
                log::info!("Starting Azure resource discovery");
                self.start_azure_discovery()
            }
            AzureDiscoveryMsg::DiscoveringSubscriptions => {
                log::info!("Discovering Azure subscriptions");
                self.discover_subscriptions()
            }
            AzureDiscoveryMsg::SubscriptionsDiscovered(subscriptions) => {
                let count = subscriptions.len();
                log::info!("Discovered {count} subscriptions");
                self.handle_subscriptions_discovered(subscriptions)
            }
            AzureDiscoveryMsg::DiscoveringResourceGroups(subscription_id) => {
                log::info!("Discovering resource groups for subscription: {subscription_id}");
                self.discover_resource_groups(subscription_id)
            }
            AzureDiscoveryMsg::ResourceGroupsDiscovered(groups) => {
                let count = groups.len();
                log::info!("Discovered {count} resource groups");
                self.handle_resource_groups_discovered(groups)
            }
            AzureDiscoveryMsg::DiscoveringNamespaces(subscription_id) => {
                log::info!(
                    "Discovering Service Bus namespaces for subscription: {subscription_id}"
                );
                self.discover_namespaces(subscription_id)
            }
            AzureDiscoveryMsg::NamespacesDiscovered(namespaces) => {
                let count = namespaces.len();
                log::info!("Discovered {count} Service Bus namespaces");
                self.handle_namespaces_discovered(namespaces)
            }
            AzureDiscoveryMsg::FetchingConnectionString {
                subscription_id,
                resource_group,
                namespace,
            } => {
                log::info!("Fetching connection string for namespace: {namespace}");
                self.fetch_connection_string(subscription_id, resource_group, namespace)
            }
            AzureDiscoveryMsg::ConnectionStringFetched(connection_string) => {
                log::info!("Successfully fetched connection string");
                self.handle_connection_string_fetched(connection_string)
            }
            AzureDiscoveryMsg::ServiceBusManagerCreated => {
                log::info!("Service Bus manager created with discovered connection string");
                self.handle_service_bus_manager_created()
            }
            AzureDiscoveryMsg::ServiceBusManagerReady(manager) => {
                log::info!("Service Bus manager ready, storing it");
                self.handle_service_bus_manager_ready(manager)
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

    pub fn handle_subscription_selection(&mut self, msg: SubscriptionSelectionMsg) -> Option<Msg> {
        match msg {
            SubscriptionSelectionMsg::SubscriptionSelected(subscription_id) => {
                log::info!("Subscription selected: {subscription_id}");
                self.state_manager.selected_subscription = Some(subscription_id.clone());
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

    pub fn handle_resource_group_selection(
        &mut self,
        msg: ResourceGroupSelectionMsg,
    ) -> Option<Msg> {
        match msg {
            ResourceGroupSelectionMsg::ResourceGroupSelected(resource_group) => {
                log::info!("Resource group selected: {resource_group}");
                self.state_manager.selected_resource_group = Some(resource_group);
                if let Err(e) = self.app.umount(&ComponentId::ResourceGroupPicker) {
                    log::error!("Failed to unmount resource group picker: {e}");
                }

                if let Some(subscription_id) = &self.state_manager.selected_subscription {
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

    fn start_azure_discovery(&mut self) -> Option<Msg> {
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

    fn discover_subscriptions(&mut self) -> Option<Msg> {
        // Check cache first for performance optimization
        if let Some(cached_subscriptions) =
            self.state_manager.azure_cache.get_cached_subscriptions()
        {
            log::info!(
                "Using cached subscriptions ({} found)",
                cached_subscriptions.len()
            );
            return Some(Msg::AzureDiscovery(
                AzureDiscoveryMsg::SubscriptionsDiscovered(cached_subscriptions.clone()),
            ));
        }

        let tx = self.tx_to_main().clone();
        let auth_service = self.auth_service.clone()?;
        let http_client = self.http_client.clone();

        self.task_manager.execute("Processing...", async move {
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
        let _ = self.mount_loading_indicator("Discovering Azure subscriptions...");
        None
    }

    fn handle_subscriptions_discovered(&mut self, subscriptions: Vec<Subscription>) -> Option<Msg> {
        // Unmount loading indicator
        if let Err(e) = self.app.umount(&ComponentId::LoadingIndicator) {
            log::warn!("Failed to unmount loading indicator: {e}");
        }

        if subscriptions.is_empty() {
            return Some(Msg::AzureDiscovery(AzureDiscoveryMsg::DiscoveryError(
                "No Azure subscriptions found. Please check your permissions.".to_string(),
            )));
        }

        // Cache subscriptions
        self.state_manager
            .azure_cache
            .cache_subscriptions(subscriptions.clone());

        // Mount subscription picker
        match self.app.mount(
            ComponentId::SubscriptionPicker,
            Box::new(SubscriptionPicker::new(Some(subscriptions))),
            vec![],
        ) {
            Ok(_) => {
                log::info!("Subscription picker mounted successfully");
                // Set the app state to prevent reverting to NamespacePicker
                self.state_manager.app_state = AppState::AzureDiscovery;
                if let Err(e) = self.app.active(&ComponentId::SubscriptionPicker) {
                    log::error!("Failed to activate subscription picker: {e}");
                }
                self.state_manager.set_redraw(true);
                log::debug!("Redraw flag set after mounting subscription picker");

                // Force an immediate redraw to ensure the picker is visible
                if let Err(e) = self.view() {
                    log::error!("Failed to force redraw after mounting subscription picker: {e:?}");
                }

                None
            }
            Err(e) => Some(Msg::AzureDiscovery(AzureDiscoveryMsg::DiscoveryError(
                format!("Failed to show subscription picker: {e}"),
            ))),
        }
    }

    fn discover_resource_groups(&mut self, subscription_id: String) -> Option<Msg> {
        // Check cache first for performance optimization
        if let Some(cached_groups) = self
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
                AzureDiscoveryMsg::ResourceGroupsDiscovered(cached_groups.clone()),
            ));
        }

        let tx = self.tx_to_main().clone();
        let auth_service = self.auth_service.clone()?;
        let http_client = self.http_client.clone();

        self.task_manager.execute("Processing...", async move {
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

        let _ = self.mount_loading_indicator("Discovering resource groups...");
        None
    }

    fn handle_resource_groups_discovered(&mut self, groups: Vec<ResourceGroup>) -> Option<Msg> {
        // Unmount loading indicator
        if let Err(e) = self.app.umount(&ComponentId::LoadingIndicator) {
            log::warn!("Failed to unmount loading indicator: {e}");
        }

        if groups.is_empty() {
            return Some(Msg::AzureDiscovery(AzureDiscoveryMsg::DiscoveryError(
                "No resource groups found in this subscription.".to_string(),
            )));
        }

        // Cache resource groups
        if let Some(subscription_id) = &self.state_manager.selected_subscription {
            self.state_manager
                .azure_cache
                .cache_resource_groups(subscription_id.clone(), groups.clone());
        }

        // Mount resource group picker
        match self.app.mount(
            ComponentId::ResourceGroupPicker,
            Box::new(ResourceGroupPicker::new(Some(groups))),
            vec![],
        ) {
            Ok(_) => {
                log::info!("Resource group picker mounted successfully");
                // Set the app state to prevent reverting to NamespacePicker
                self.state_manager.app_state = AppState::AzureDiscovery;
                if let Err(e) = self.app.active(&ComponentId::ResourceGroupPicker) {
                    log::error!("Failed to activate resource group picker: {e}");
                }
                self.state_manager.set_redraw(true);
                log::debug!("Redraw flag set after mounting resource group picker");

                // Force an immediate redraw to ensure the picker is visible
                if let Err(e) = self.view() {
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

    fn discover_namespaces(&mut self, subscription_id: String) -> Option<Msg> {
        // Check cache first for performance optimization
        if let Some(cached_namespaces) = self
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
                AzureDiscoveryMsg::NamespacesDiscovered(cached_namespaces.clone()),
            ));
        }

        let tx = self.tx_to_main().clone();
        let auth_service = self.auth_service.clone()?;
        let http_client = self.http_client.clone();

        self.task_manager.execute("Processing...", async move {
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

        let _ = self.mount_loading_indicator("Discovering Service Bus namespaces...");
        None
    }

    fn handle_namespaces_discovered(
        &mut self,
        namespaces: Vec<ServiceBusNamespace>,
    ) -> Option<Msg> {
        // Unmount loading indicator
        if let Err(e) = self.app.umount(&ComponentId::LoadingIndicator) {
            log::warn!("Failed to unmount loading indicator: {e}");
        }

        if namespaces.is_empty() {
            return Some(Msg::AzureDiscovery(AzureDiscoveryMsg::DiscoveryError(
                "No Service Bus namespaces found. Please create a namespace first.".to_string(),
            )));
        }

        // Cache namespaces
        if let Some(subscription_id) = &self.state_manager.selected_subscription {
            self.state_manager
                .azure_cache
                .cache_namespaces(subscription_id.clone(), namespaces.clone());
        }

        // Convert to namespace names for the picker
        let namespace_names: Vec<String> = namespaces.iter().map(|ns| ns.name.clone()).collect();

        // Update existing namespace picker with discovered namespaces
        if self.app.mounted(&ComponentId::NamespacePicker) {
            if let Err(e) = self.app.umount(&ComponentId::NamespacePicker) {
                log::warn!("Failed to unmount existing namespace picker: {e}");
            }
        }

        match self.app.mount(
            ComponentId::NamespacePicker,
            Box::new(NamespacePicker::new(Some(namespace_names))),
            vec![Sub::new(SubEventClause::Any, SubClause::Always)],
        ) {
            Ok(_) => {
                // Keep in AzureDiscovery state for namespace selection
                self.state_manager.app_state = AppState::AzureDiscovery;

                // Activate the namespace picker with proper error handling
                if let Err(e) = self.app.active(&ComponentId::NamespacePicker) {
                    log::error!("Failed to activate namespace picker: {e}");
                    return Some(Msg::AzureDiscovery(AzureDiscoveryMsg::DiscoveryError(
                        format!("Failed to activate namespace picker: {e}"),
                    )));
                }

                self.state_manager.set_redraw(true);

                // Force an immediate redraw to ensure the picker is visible
                if let Err(e) = self.view() {
                    log::error!("Failed to force redraw after mounting namespace picker: {e:?}");
                }

                // Store the full namespace objects for later use
                self.state_manager.discovered_namespaces = namespaces;
                None
            }
            Err(e) => Some(Msg::AzureDiscovery(AzureDiscoveryMsg::DiscoveryError(
                format!("Failed to show namespace picker: {e}"),
            ))),
        }
    }

    fn fetch_connection_string(
        &mut self,
        subscription_id: String,
        resource_group: String,
        namespace: String,
    ) -> Option<Msg> {
        let tx = self.tx_to_main().clone();
        let auth_service = self.auth_service.clone()?;
        let http_client = self.http_client.clone();

        self.task_manager.execute("Processing...", async move {
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

        let _ = self.mount_loading_indicator("Fetching connection string...");
        None
    }

    fn handle_connection_string_fetched(&mut self, connection_string: String) -> Option<Msg> {
        // Unmount loading indicator
        if let Err(e) = self.app.umount(&ComponentId::LoadingIndicator) {
            log::warn!("Failed to unmount loading indicator: {e}");
        }

        // Store the connection string in state
        self.state_manager.discovered_connection_string = Some(connection_string.clone());

        // Since we have the connection string, we can now create the Service Bus manager
        // We'll do this synchronously since we're already in the main thread
        Some(Msg::AzureDiscovery(
            AzureDiscoveryMsg::ServiceBusManagerCreated,
        ))
    }

    fn handle_service_bus_manager_created(&mut self) -> Option<Msg> {
        log::info!("Creating Service Bus manager with discovered connection string");

        // Get the discovered connection string
        let connection_string = match &self.state_manager.discovered_connection_string {
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
            &self.state_manager.selected_subscription,
            &self.state_manager.selected_resource_group,
            &self.state_manager.selected_namespace,
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
            server::service_bus_manager::azure_management_client::StatisticsConfig::new(
                config.queue_stats_display_enabled(),
                config.queue_stats_cache_ttl_seconds(),
                config.queue_stats_use_management_api(),
            );
        let batch_config = config.batch().clone();

        // Create the Service Bus client - we'll handle this asynchronously
        let tx = self.tx_to_main().clone();
        let task_manager = self.task_manager.clone();
        let http_client = self.http_client.clone();

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

    fn handle_service_bus_manager_ready(
        &mut self,
        manager: Arc<tokio::sync::Mutex<ServiceBusManager>>,
    ) -> Option<Msg> {
        log::info!("Storing Service Bus manager and updating queue manager");

        // Store the manager in the model
        self.service_bus_manager = Some(manager.clone());

        // Update the queue manager with the new service bus manager
        self.queue_manager.set_service_bus_manager(manager);

        // Discovery is complete
        Some(Msg::AzureDiscovery(AzureDiscoveryMsg::DiscoveryComplete))
    }

    fn finalize_discovery(&mut self) -> Option<Msg> {
        log::info!("Finalizing Azure discovery");

        // Clear authentication flag since discovery is complete
        self.state_manager.is_authenticating = false;

        // If we have a discovered connection string and namespace, we can proceed
        if self.state_manager.discovered_connection_string.is_some() {
            // We already have the namespace from discovery, so we can skip loading namespaces
            // The namespace was already set when the user selected it

            // Unmount any discovery-related pickers
            let _ = self.app.umount(&ComponentId::SubscriptionPicker);
            let _ = self.app.umount(&ComponentId::ResourceGroupPicker);
            let _ = self.app.umount(&ComponentId::NamespacePicker);

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
