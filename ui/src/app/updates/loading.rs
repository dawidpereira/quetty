use crate::app::model::{AppState, Model};
use crate::components::common::{ComponentId, LoadingActivityMsg, Msg};
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
}

