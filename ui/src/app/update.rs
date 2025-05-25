use crate::app::model::{AppState, Model};
use crate::components::common::{
    ComponentId, LoadingActivityMsg, MessageActivityMsg, Msg, NamespaceActivityMsg,
    QueueActivityMsg,
};
use crate::components::help_screen::HelpScreen;

use std::sync::Arc;
use tokio::sync::Mutex;
use tuirealm::State;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn update_loading(&mut self, msg: LoadingActivityMsg) -> Option<Msg> {
        match msg {
            LoadingActivityMsg::Start(message) => {
                log::debug!("Starting loading: {}", message);

                // Store current state to return to later
                let previous_state = self.app_state.clone();

                // Store loading message and previous state
                self.loading_message = Some((message.clone(), previous_state));

                // Mount loading indicator with proper subscriptions
                if let Err(e) = self.mount_loading_indicator(&message) {
                    log::error!("Failed to mount loading indicator: {}", e);
                }

                self.app_state = AppState::Loading;
                self.redraw = true;
                None
            }
            LoadingActivityMsg::Update(message) => {
                log::debug!("Updating loading message: {}", message);

                // Update loading message, keep previous state
                if let Some((_, previous_state)) = &self.loading_message {
                    self.loading_message = Some((message.clone(), previous_state.clone()));
                } else {
                    // If no previous message, store current state
                    self.loading_message = Some((message.clone(), self.app_state.clone()));
                }

                // Mount loading indicator with proper subscriptions
                if let Err(e) = self.mount_loading_indicator(&message) {
                    log::error!("Failed to mount loading indicator: {}", e);
                }

                self.redraw = true;
                None
            }
            LoadingActivityMsg::Stop => {
                log::debug!("Stopping loading");

                // Return to previous state if we have one
                if let Some((_, previous_state)) = self.loading_message.take() {
                    if previous_state != AppState::Loading {
                        self.app_state = previous_state;
                    } else {
                        // If previous state was also loading, go to NamespacePicker
                        self.app_state = AppState::NamespacePicker;
                    }
                }

                // Unmount loading indicator
                if self.app.mounted(&ComponentId::LoadingIndicator) {
                    if let Err(e) = self.app.umount(&ComponentId::LoadingIndicator) {
                        log::error!("Failed to unmount loading indicator: {}", e);
                    } else {
                        log::debug!("Loading indicator unmounted successfully");
                    }
                }

                self.redraw = true;
                None
            }
        }
    }

    pub fn update_messages(&mut self, msg: MessageActivityMsg) -> Option<Msg> {
        match msg {
            MessageActivityMsg::EditMessage(index) => {
                if let Err(e) = self.remount_message_details(index) {
                    return Some(Msg::Error(e));
                }
                self.app_state = AppState::MessageDetails;
                Some(Msg::ForceRedraw)
            }
            MessageActivityMsg::CancelEditMessage => {
                self.app_state = AppState::MessagePicker;
                None
            }
            MessageActivityMsg::MessagesLoaded(messages) => {
                self.messages = Some(messages);
                if let Err(e) = self.remount_messages() {
                    return Some(Msg::Error(e));
                }
                if let Err(e) = self.remount_message_details(0) {
                    return Some(Msg::Error(e));
                }
                self.app_state = AppState::MessagePicker;
                None
            }
            MessageActivityMsg::ConsumerCreated(consumer) => {
                self.consumer = Some(Arc::new(Mutex::new(consumer)));
                if let Err(e) = self.load_messages() {
                    return Some(Msg::Error(e));
                }
                None
            }
            MessageActivityMsg::PreviewMessageDetails(index) => {
                if let Err(e) = self.remount_message_details(index) {
                    return Some(Msg::Error(e));
                }
                None
            }
        }
    }

    pub fn update_queue(&mut self, msg: QueueActivityMsg) -> Option<Msg> {
        match msg {
            QueueActivityMsg::QueueSelected(queue) => {
                self.pending_queue = Some(queue);
                if let Err(e) = self.new_consumer_for_queue() {
                    return Some(Msg::Error(e));
                }
                None
            }
            QueueActivityMsg::QueuesLoaded(queues) => {
                if let Err(e) = self.remount_queue_picker(Some(queues)) {
                    return Some(Msg::Error(e));
                }
                self.app_state = AppState::QueuePicker;
                None
            }
            QueueActivityMsg::QueueUnselected => {
                self.app_state = AppState::QueuePicker;
                None
            }
        }
    }

    pub fn update_namespace(&mut self, msg: NamespaceActivityMsg) -> Option<Msg> {
        match msg {
            NamespaceActivityMsg::NamespacesLoaded(namespace) => {
                if let Err(e) = self.remount_namespace_picker(Some(namespace)) {
                    return Some(Msg::Error(e));
                }
                self.app_state = AppState::NamespacePicker;
                None
            }
            NamespaceActivityMsg::NamespaceSelected => {
                // Store the currently selected namespace from the namespace picker component
                if let Ok(State::One(tuirealm::StateValue::String(namespace))) =
                    self.app.state(&ComponentId::NamespacePicker)
                {
                    log::info!("Selected namespace: {}", namespace);
                    self.selected_namespace = Some(namespace);
                }

                if let Err(e) = self.load_queues() {
                    return Some(Msg::Error(e));
                }
                None
            }
            NamespaceActivityMsg::NamespaceUnselected => {
                // Clear selected namespace
                self.selected_namespace = None;

                if let Err(e) = self.load_namespaces() {
                    return Some(Msg::Error(e));
                }
                None
            }
        }
    }

    pub fn update_help(&mut self) -> Option<Msg> {
        // Toggle between help screen and previous state
        if self.app_state == AppState::HelpScreen {
            // If we're already showing help screen, go back to previous state
            if let Some(prev_state) = self.previous_state.take() {
                self.app_state = prev_state;

                // Unmount the help screen
                if let Err(e) = self.app.umount(&ComponentId::HelpScreen) {
                    log::error!("Failed to unmount help screen: {}", e);
                }

                // Return to appropriate component based on state
                match self.app_state {
                    AppState::NamespacePicker => {
                        if let Err(e) = self.app.active(&ComponentId::NamespacePicker) {
                            log::error!("Failed to activate namespace picker: {}", e);
                        }
                    }
                    AppState::QueuePicker => {
                        if let Err(e) = self.app.active(&ComponentId::QueuePicker) {
                            log::error!("Failed to activate queue picker: {}", e);
                        }
                    }
                    AppState::MessagePicker => {
                        if let Err(e) = self.app.active(&ComponentId::Messages) {
                            log::error!("Failed to activate messages: {}", e);
                        }
                    }
                    AppState::MessageDetails => {
                        if let Err(e) = self.app.active(&ComponentId::MessageDetails) {
                            log::error!("Failed to activate message details: {}", e);
                        }
                    }
                    _ => {}
                }
            } else {
                // If we don't have a previous state, default to NamespacePicker
                self.app_state = AppState::NamespacePicker;

                // Unmount the help screen
                if let Err(e) = self.app.umount(&ComponentId::HelpScreen) {
                    log::error!("Failed to unmount help screen: {}", e);
                }

                if let Err(e) = self.app.active(&ComponentId::NamespacePicker) {
                    log::error!("Failed to activate namespace picker: {}", e);
                }
            }
        } else {
            // Save current state before showing help screen
            self.previous_state = Some(self.app_state.clone());

            // Show help screen
            self.app_state = AppState::HelpScreen;

            // Mount help screen component if not already mounted
            if !self.app.mounted(&ComponentId::HelpScreen) {
                if let Err(e) = self.app.mount(
                    ComponentId::HelpScreen,
                    Box::new(HelpScreen::new()),
                    Vec::default(),
                ) {
                    log::error!("Failed to mount help screen: {}", e);
                }
            }

            // Activate the help screen
            if let Err(e) = self.app.active(&ComponentId::HelpScreen) {
                log::error!("Failed to activate help screen: {}", e);
            }
        }

        self.redraw = true;
        None
    }
}
