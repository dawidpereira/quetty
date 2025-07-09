use crate::components::common::{ComponentId, Msg};
use server::service_bus_manager::azure_management_client::{
    AzureResourceCache, ServiceBusNamespace,
};
use std::sync::mpsc::Sender;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    NamespacePicker,
    QueuePicker,
    MessagePicker,
    MessageDetails,
    Loading,
    HelpScreen,
    ThemePicker,
    ConfigScreen,
    PasswordPopup,
    AzureDiscovery,
}

/// Manages application state transitions and UI state
pub struct StateManager {
    pub app_state: AppState,
    pub active_component: ComponentId,
    pub quit: bool,
    pub redraw: bool,
    pub selected_namespace: Option<String>,
    pub loading_message: Option<(String, AppState)>,
    pub loading_cancel_button: Option<String>, // operation_id for cancel button
    pub previous_state: Option<AppState>,
    pub pending_confirmation_action: Option<Box<Msg>>,
    pub is_editing_message: bool,
    pub tx_to_main: Sender<Msg>,
    pub current_page_size: Option<u32>, // Dynamic page size that can be changed during runtime
    pub is_authenticating: bool,        // Track if authentication is in progress
    pub last_device_code_copy: Option<std::time::Instant>, // Track last copy time to prevent spam

    // Azure discovery state
    pub azure_cache: AzureResourceCache,
    pub selected_subscription: Option<String>,
    pub selected_resource_group: Option<String>,
    pub discovered_namespaces: Vec<ServiceBusNamespace>,
    pub discovered_connection_string: Option<String>,
}

impl StateManager {
    /// Create a new StateManager
    pub fn new(tx_to_main: Sender<Msg>) -> Self {
        // Get cache configuration from app config
        let config = crate::config::get_config_or_panic();
        let cache_ttl = Duration::from_secs(config.azure_resource_cache_ttl_seconds());
        let max_entries = config.azure_resource_cache_max_entries();

        Self {
            app_state: AppState::Loading,
            active_component: ComponentId::LoadingIndicator,
            quit: false,
            redraw: true,
            selected_namespace: None,
            loading_message: None,
            loading_cancel_button: None,
            previous_state: None,
            pending_confirmation_action: None,
            is_editing_message: false,
            tx_to_main,
            current_page_size: None,
            is_authenticating: false,
            last_device_code_copy: None,
            azure_cache: AzureResourceCache::with_config(cache_ttl, max_entries),
            selected_subscription: None,
            selected_resource_group: None,
            discovered_namespaces: Vec::new(),
            discovered_connection_string: None,
        }
    }

    /// Change application state with validation
    pub fn set_app_state(&mut self, new_state: AppState) {
        // Validate state transition
        if !self.is_valid_transition(&self.app_state, &new_state) {
            log::warn!(
                "Invalid state transition attempted: {:?} -> {:?}",
                self.app_state,
                new_state
            );
            return;
        }

        log::debug!("State transition: {:?} -> {:?}", self.app_state, new_state);
        self.app_state = new_state;
        self.redraw = true;
    }

    /// Validate if a state transition is allowed
    fn is_valid_transition(&self, from: &AppState, to: &AppState) -> bool {
        match (from, to) {
            // Loading state can transition to any state
            (AppState::Loading, _) => true,

            // Any state can transition to Loading or HelpScreen
            (_, AppState::Loading) | (_, AppState::HelpScreen) => true,

            // Namespace picker transitions
            (AppState::NamespacePicker, AppState::QueuePicker) => self.selected_namespace.is_some(),
            (AppState::NamespacePicker, AppState::AzureDiscovery) => true,

            // Queue picker transitions
            (AppState::QueuePicker, AppState::MessagePicker) => true,
            (AppState::QueuePicker, AppState::NamespacePicker) => true,

            // Message picker transitions
            (AppState::MessagePicker, AppState::MessageDetails) => true,
            (AppState::MessagePicker, AppState::QueuePicker) => true,

            // Message details transitions
            (AppState::MessageDetails, AppState::MessagePicker) => true,

            // Help screen can go back to any state
            (AppState::HelpScreen, _) => true,

            // Theme picker transitions
            (AppState::ThemePicker, _) => true,
            (_, AppState::ThemePicker) => true,

            // Config screen transitions
            (AppState::ConfigScreen, _) => true,
            (_, AppState::ConfigScreen) => true,

            // Password popup transitions
            (AppState::PasswordPopup, _) => true,
            (_, AppState::PasswordPopup) => true,

            // Azure discovery transitions
            (AppState::AzureDiscovery, AppState::NamespacePicker) => {
                // Can only go to namespace picker if we have a connection string
                self.discovered_connection_string.is_some()
            }

            // Same state transitions are always allowed (no-op)
            _ if from == to => true,

            // All other transitions are invalid
            _ => {
                log::debug!("Transition {from:?} -> {to:?} not allowed");
                false
            }
        }
    }

    /// Set the active component
    pub fn set_active_component(&mut self, component: ComponentId) {
        self.active_component = component;
        self.redraw = true;
    }

    /// Take and return the pending confirmation action
    pub fn take_pending_confirmation(&mut self) -> Option<Box<Msg>> {
        self.pending_confirmation_action.take()
    }

    /// Set message editing mode
    pub fn set_editing_message(&mut self, editing: bool) {
        self.is_editing_message = editing;
    }

    /// Signal application shutdown
    pub fn shutdown(&mut self) {
        self.quit = true;
    }

    /// Check if application should quit
    pub fn should_quit(&self) -> bool {
        self.quit
    }

    /// Check if redraw is needed
    pub fn needs_redraw(&self) -> bool {
        self.redraw
    }

    /// Set redraw flag
    pub fn set_redraw(&mut self, redraw: bool) {
        self.redraw = redraw;
    }

    /// Mark redraw as complete
    pub fn redraw_complete(&mut self) {
        self.redraw = false;
    }

    /// Get the current page size, falling back to config if not set
    pub fn get_current_page_size(&self) -> u32 {
        self.current_page_size
            .unwrap_or_else(|| crate::config::get_config_or_panic().max_messages())
    }

    // ===== Atomic State Update Methods =====

    /// Atomically update Azure resource selection state
    /// This ensures all related fields are updated together to prevent inconsistent state
    pub fn update_azure_selection(
        &mut self,
        subscription: Option<String>,
        resource_group: Option<String>,
        namespace: Option<String>,
    ) {
        // Validate the selection hierarchy
        if !self.is_valid_azure_selection(&subscription, &resource_group, &namespace) {
            log::warn!(
                "Invalid Azure selection hierarchy: sub={subscription:?}, rg={resource_group:?}, ns={namespace:?}"
            );
            return;
        }

        log::debug!(
            "Atomic Azure selection update: sub={subscription:?}, rg={resource_group:?}, ns={namespace:?}"
        );

        // Clear dependent fields when parent selection changes
        if subscription != self.selected_subscription {
            // Subscription changed, clear resource group and namespace
            self.selected_resource_group = None;
            self.discovered_namespaces.clear();
            self.discovered_connection_string = None;
        }

        if resource_group != self.selected_resource_group {
            // Resource group changed, clear namespace
            self.discovered_namespaces.clear();
            self.discovered_connection_string = None;
        }

        // Update all fields atomically
        self.selected_subscription = subscription;
        self.selected_resource_group = resource_group;
        self.selected_namespace = namespace;
        self.redraw = true;
    }

    /// Validate Azure resource selection hierarchy
    fn is_valid_azure_selection(
        &self,
        subscription: &Option<String>,
        resource_group: &Option<String>,
        namespace: &Option<String>,
    ) -> bool {
        match (subscription, resource_group, namespace) {
            // Valid: Nothing selected
            (None, None, None) => true,

            // Valid: Only subscription selected
            (Some(_), None, None) => true,

            // Valid: Subscription and resource group selected
            (Some(_), Some(_), None) => true,

            // Valid: All selected
            (Some(_), Some(_), Some(_)) => true,

            // Invalid: Resource group without subscription
            (None, Some(_), _) => {
                log::error!("Cannot select resource group without subscription");
                false
            }

            // Invalid: Namespace without subscription or resource group
            (None, None, Some(_)) | (Some(_), None, Some(_)) => {
                log::error!("Cannot select namespace without subscription and resource group");
                false
            }
        }
    }

    /// Atomically update loading state with associated data
    pub fn set_loading_state(&mut self, message: String, return_state: AppState) {
        log::debug!("Setting loading state: {message} -> {return_state:?}");
        self.previous_state = Some(self.app_state.clone());
        self.loading_message = Some((message, return_state));
        self.app_state = AppState::Loading;
        self.active_component = ComponentId::LoadingIndicator;
        self.redraw = true;
    }

    /// Atomically clear loading state and return to specified state
    pub fn clear_loading_state(&mut self, target_state: Option<AppState>) {
        log::debug!("Clearing loading state, target: {target_state:?}");

        if let Some(state) = target_state {
            self.app_state = state;
        } else if let Some((_, return_state)) = &self.loading_message {
            self.app_state = return_state.clone();
        } else if let Some(prev) = &self.previous_state {
            self.app_state = prev.clone();
        }

        self.loading_message = None;
        self.loading_cancel_button = None;
        self.previous_state = None;
        self.redraw = true;
    }

    /// Atomically update authentication state
    pub fn set_authentication_state(&mut self, is_authenticating: bool) {
        log::debug!("Setting authentication state: {is_authenticating}");
        self.is_authenticating = is_authenticating;

        if is_authenticating {
            self.set_loading_state(
                "Authenticating with Azure AD...".to_string(),
                AppState::AzureDiscovery,
            );
        } else {
            self.clear_loading_state(None);
        }
    }

    /// Atomically update discovered Azure resources
    pub fn update_discovered_resources(
        &mut self,
        namespaces: Vec<ServiceBusNamespace>,
        connection_string: Option<String>,
    ) {
        let namespace_count = namespaces.len();
        log::debug!("Updating discovered resources: {namespace_count} namespaces");
        self.discovered_namespaces = namespaces;
        self.discovered_connection_string = connection_string;
        self.redraw = true;
    }

    /// Atomically reset all Azure discovery state
    pub fn reset_azure_discovery_state(&mut self) {
        log::debug!("Resetting all Azure discovery state");
        self.selected_subscription = None;
        self.selected_resource_group = None;
        self.discovered_namespaces.clear();
        self.discovered_connection_string = None;
        self.azure_cache.clear();
        self.redraw = true;
    }
}
