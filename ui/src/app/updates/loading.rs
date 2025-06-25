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
                let previous_state = self.state_manager.app_state.clone();

                // Store loading message and previous state
                self.state_manager.loading_message = Some((message.clone(), previous_state));

                // Mount loading indicator with proper subscriptions
                if let Err(e) = self.mount_loading_indicator(&message) {
                    log::error!("Failed to mount loading indicator: {}", e);
                }

                self.set_app_state(AppState::Loading);
                self.set_redraw(true);
                None
            }
            LoadingActivityMsg::Stop => {
                log::debug!("Stopping loading");

                // Only revert to previous state if we're still in Loading state
                // This prevents overriding state changes that happened during loading
                if self.state_manager.app_state == AppState::Loading {
                    if let Some((_, previous_state)) = self.state_manager.loading_message.take() {
                        if previous_state != AppState::Loading {
                            self.set_app_state(previous_state);
                        } else {
                            // If previous state was also loading, go to NamespacePicker
                            self.set_app_state(AppState::NamespacePicker);
                        }
                    }
                } else {
                    // App state has changed during loading, keep the current state
                    self.state_manager.loading_message.take();
                    log::debug!(
                        "Loading stopped but app state has changed to {:?}, keeping current state",
                        self.state_manager.app_state
                    );
                }

                // Unmount loading indicator
                if self.app.mounted(&ComponentId::LoadingIndicator) {
                    if let Err(e) = self.app.umount(&ComponentId::LoadingIndicator) {
                        log::error!("Failed to unmount loading indicator: {}", e);
                    } else {
                        log::debug!("Loading indicator unmounted successfully");
                    }
                }

                self.set_redraw(true);
                None
            }
        }
    }
}
