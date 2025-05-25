use crate::app::model::{AppState, Model};
use crate::components::common::{ComponentId, Msg};
use crate::components::help_screen::HelpScreen;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
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

