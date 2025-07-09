use crate::app::model::Model;
use crate::components::common::{
    AzureDiscoveryMsg, ComponentId, Msg, NamespaceActivityMsg, QueueActivityMsg,
};
use tuirealm::State;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn update_namespace(&mut self, msg: NamespaceActivityMsg) -> Option<Msg> {
        match msg {
            NamespaceActivityMsg::NamespacesLoaded(namespaces) => {
                log::info!(
                    "Received namespaces loaded event with {} namespaces",
                    namespaces.len()
                );

                if namespaces.is_empty() {
                    log::warn!("No namespaces found - showing empty namespace picker");
                    if let Err(e) = self.remount_namespace_picker(Some(namespaces)) {
                        self.error_reporter.report_simple(
                            e,
                            "NamespaceHandler",
                            "update_namespace",
                        );
                    }
                    return None;
                }

                // If there's only one namespace (e.g., from connection string), automatically select it
                if namespaces.len() == 1 {
                    log::info!(
                        "Only one namespace available, automatically selecting: '{}'",
                        namespaces[0]
                    );
                    self.set_selected_namespace(Some(namespaces[0].clone()));
                    log::info!("Proceeding to handle namespace selection automatically");
                    return self.handle_namespace_selection();
                }

                // Multiple namespaces - show picker
                log::info!("Multiple namespaces found, showing namespace picker");
                if let Err(e) = self.remount_namespace_picker(Some(namespaces)) {
                    self.error_reporter
                        .report_simple(e, "NamespaceHandler", "update_namespace");
                }
                None
            }
            NamespaceActivityMsg::NamespaceSelected => self.handle_namespace_selection(),
            NamespaceActivityMsg::NamespaceCancelled => {
                // In discovery mode, go back to resource group selection
                if self.state_manager.selected_subscription.is_some()
                    && self.state_manager.selected_resource_group.is_some()
                {
                    log::info!("Discovery mode: Going back to resource group selection");
                    // Clear selected namespace
                    self.set_selected_namespace(None);

                    // Change state to AzureDiscovery before unmounting to avoid rendering issues
                    self.set_app_state(crate::app::model::AppState::AzureDiscovery);

                    // Unmount namespace picker
                    if let Err(e) = self.app.umount(&ComponentId::NamespacePicker) {
                        log::error!("Failed to unmount namespace picker: {e}");
                    }
                    // Go back to resource group selection
                    Some(Msg::AzureDiscovery(
                        AzureDiscoveryMsg::DiscoveringResourceGroups(
                            self.state_manager.selected_subscription.clone().unwrap(),
                        ),
                    ))
                } else {
                    // Not in discovery mode, just close
                    log::info!("Not in discovery mode, closing namespace picker");
                    if let Err(e) = self.app.umount(&ComponentId::NamespacePicker) {
                        log::error!("Failed to unmount namespace picker: {e}");
                    }
                    self.set_quit(true);
                    Some(Msg::AppClose)
                }
            }
            NamespaceActivityMsg::NamespaceUnselected => {
                // Clear selected namespace
                self.set_selected_namespace(None);

                // Check if we're in discovery mode
                if self.state_manager.selected_subscription.is_some()
                    && self.state_manager.selected_resource_group.is_some()
                    && !self.state_manager.discovered_namespaces.is_empty()
                {
                    // In discovery mode - go back to namespace selection
                    log::info!("Discovery mode: Going back to namespace selection");
                    let namespaces: Vec<String> = self
                        .state_manager
                        .discovered_namespaces
                        .iter()
                        .map(|ns| ns.name.clone())
                        .collect();
                    return Some(Msg::NamespaceActivity(
                        NamespaceActivityMsg::NamespacesLoaded(namespaces),
                    ));
                } else {
                    // Check if we have configuration-based subscription ID
                    let config = crate::config::get_config_or_panic();
                    let has_subscription_id = config.azure_ad().subscription_id().is_ok();

                    if !has_subscription_id && !self.state_manager.discovered_namespaces.is_empty()
                    {
                        // Still in discovery mode
                        log::info!("Using discovered namespaces for namespace picker");
                        let namespaces: Vec<String> = self
                            .state_manager
                            .discovered_namespaces
                            .iter()
                            .map(|ns| ns.name.clone())
                            .collect();
                        return Some(Msg::NamespaceActivity(
                            NamespaceActivityMsg::NamespacesLoaded(namespaces),
                        ));
                    } else {
                        // Normal mode - load namespaces from Azure
                        self.load_namespaces();
                    }
                }
                None
            }
        }
    }

    /// Handle namespace selection by storing the selected namespace and loading queues
    fn handle_namespace_selection(&mut self) -> Option<Msg> {
        // Try to get namespace from component state first, then fall back to stored state
        let namespace = if let Ok(State::One(tuirealm::StateValue::String(ns))) =
            self.app.state(&ComponentId::NamespacePicker)
        {
            log::info!("Selected namespace from component: {ns}");
            ns
        } else if let Some(stored_namespace) = &self.state_manager.selected_namespace {
            log::info!("Using stored namespace: {stored_namespace}");
            stored_namespace.clone()
        } else {
            log::error!("No namespace available in component state or stored state");
            return None;
        };

        log::info!("Processing namespace selection: '{namespace}'");
        log::info!(
            "Auth method: {}",
            crate::config::get_config_or_panic().azure_ad().auth_method
        );

        // Store the selected namespace first
        self.set_selected_namespace(Some(namespace.clone()));

        // Check if we're in discovery mode and need to fetch connection string
        if self.state_manager.discovered_connection_string.is_none()
            && self.state_manager.selected_subscription.is_some()
            && self.state_manager.selected_resource_group.is_some()
        {
            // Find the full namespace object
            if let Some(_ns) = self
                .state_manager
                .discovered_namespaces
                .iter()
                .find(|n| n.name == namespace)
            {
                let subscription_id = self.state_manager.selected_subscription.clone().unwrap();
                let resource_group = self.state_manager.selected_resource_group.clone().unwrap();

                return Some(Msg::AzureDiscovery(
                    AzureDiscoveryMsg::FetchingConnectionString {
                        subscription_id,
                        resource_group,
                        namespace: namespace.clone(),
                    },
                ));
            }
        }

        // Check if we're using discovered resources (no subscription ID in config)
        let config = crate::config::get_config_or_panic();
        let has_subscription_id = config.azure_ad().has_subscription_id();

        log::info!(
            "Discovery mode check: has_subscription_id={}, discovered_connection_string={:?}",
            has_subscription_id,
            self.state_manager.discovered_connection_string.is_some()
        );

        if !has_subscription_id && self.state_manager.discovered_connection_string.is_some() {
            // We're in discovery mode
            // Note: We don't unmount the namespace picker here anymore to avoid view errors
            // The picker will be unmounted by the queue loading process

            // In discovery mode, we can still list queues using the discovered resources
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
                log::info!("Discovery mode: Loading queues for namespace {namespace}");
                self.queue_manager.load_queues_with_discovery(
                    subscription_id.clone(),
                    resource_group.clone(),
                    namespace.clone(),
                    auth_service.clone(),
                    self.http_client.clone(),
                );
            } else {
                log::warn!("Discovery mode but missing required information to list queues");
                return Some(Msg::QueueActivity(QueueActivityMsg::QueuesLoaded(vec![])));
            }
        } else {
            // Normal mode with subscription ID configured
            // For all authentication methods, proceed to queue discovery
            log::info!("Not in discovery mode - proceeding to load queues");
            self.load_queues();
        }

        None
    }
}
