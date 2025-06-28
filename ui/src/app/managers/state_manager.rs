use crate::components::common::{ComponentId, Msg};
use std::sync::mpsc::Sender;

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    NamespacePicker,
    QueuePicker,
    MessagePicker,
    MessageDetails,
    Loading,
    HelpScreen,
    ThemePicker,
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
}

impl StateManager {
    /// Create a new StateManager
    pub fn new(tx_to_main: Sender<Msg>) -> Self {
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
        }
    }

    /// Change application state
    pub fn set_app_state(&mut self, new_state: AppState) {
        log::debug!("State transition: {:?} -> {:?}", self.app_state, new_state);
        self.app_state = new_state;
        self.redraw = true;
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
}
