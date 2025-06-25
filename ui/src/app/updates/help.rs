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
        if self.state_manager.app_state == AppState::HelpScreen {
            // If we're already showing help screen, go back to previous state
            if let Some(prev_state) = self.state_manager.previous_state.take() {
                self.set_app_state(prev_state);

                // Unmount the help screen
                if let Err(e) = self.app.umount(&ComponentId::HelpScreen) {
                    self.error_reporter
                        .report_mount_error("HelpScreen", "unmount", &e);
                }

                // Return to appropriate component based on state
                match self.state_manager.app_state {
                    AppState::NamespacePicker => {
                        if let Err(e) = self.app.active(&ComponentId::NamespacePicker) {
                            self.error_reporter
                                .report_activation_error("NamespacePicker", &e);
                        }
                    }
                    AppState::QueuePicker => {
                        if let Err(e) = self.app.active(&ComponentId::QueuePicker) {
                            self.error_reporter
                                .report_activation_error("QueuePicker", &e);
                        }
                    }
                    AppState::MessagePicker => {
                        if let Err(e) = self.app.active(&ComponentId::Messages) {
                            self.error_reporter.report_activation_error("Messages", &e);
                        }
                    }
                    AppState::MessageDetails => {
                        if let Err(e) = self.app.active(&ComponentId::MessageDetails) {
                            self.error_reporter
                                .report_activation_error("MessageDetails", &e);
                        }
                    }
                    AppState::ThemePicker => {
                        if let Err(e) = self.app.active(&ComponentId::ThemePicker) {
                            self.error_reporter
                                .report_activation_error("ThemePicker", &e);
                        }
                    }
                    _ => {}
                }
            } else {
                // If we don't have a previous state, default to NamespacePicker
                self.set_app_state(AppState::NamespacePicker);

                // Unmount the help screen
                if let Err(e) = self.app.umount(&ComponentId::HelpScreen) {
                    self.error_reporter
                        .report_mount_error("HelpScreen", "unmount", &e);
                }

                if let Err(e) = self.app.active(&ComponentId::NamespacePicker) {
                    self.error_reporter
                        .report_activation_error("NamespacePicker", &e);
                }
            }
        } else {
            // Save current state before showing help screen
            self.state_manager.previous_state = Some(self.state_manager.app_state.clone());

            // Show help screen
            self.set_app_state(AppState::HelpScreen);

            // Mount help screen component if not already mounted
            if !self.app.mounted(&ComponentId::HelpScreen) {
                if let Err(e) = self.app.mount(
                    ComponentId::HelpScreen,
                    Box::new(HelpScreen::new()),
                    Vec::default(),
                ) {
                    self.error_reporter
                        .report_mount_error("HelpScreen", "mount", &e);
                }
            }

            // Activate the help screen
            if let Err(e) = self.app.active(&ComponentId::HelpScreen) {
                self.error_reporter
                    .report_activation_error("HelpScreen", &e);
            }
        }

        self.set_redraw(true);
        None
    }
}
