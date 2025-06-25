use super::Model;
use crate::components::common::{MessageActivityMsg, Msg, PopupActivityMsg};
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn handle_update(&mut self, msg: Option<Msg>) -> Option<Msg> {
        if let Some(msg) = msg {
            // Set redraw
            self.set_redraw(true);

            // Process the message and handle any resulting errors
            let result = match msg {
                Msg::AppClose => {
                    self.shutdown(); // Properly shutdown and terminate
                    None
                }

                Msg::MessageActivity(MessageActivityMsg::EditingModeStarted) => {
                    self.set_editing_message(true);
                    if let Err(e) = self.update_global_key_watcher_editing_state() {
                        self.error_reporter.report_key_watcher_error(e);
                    }
                    None
                }
                Msg::MessageActivity(MessageActivityMsg::EditingModeStopped) => {
                    self.set_editing_message(false);
                    if let Err(e) = self.update_global_key_watcher_editing_state() {
                        self.error_reporter.report_key_watcher_error(e);
                    }
                    None
                }
                Msg::MessageActivity(msg) => self.update_messages(msg),
                Msg::QueueActivity(msg) => self.update_queue(msg),
                Msg::NamespaceActivity(msg) => self.update_namespace(msg),
                Msg::ThemeActivity(msg) => self.update_theme(msg),
                Msg::LoadingActivity(msg) => self.update_loading(msg),
                Msg::PopupActivity(msg) => self.update_popup(msg),
                Msg::Error(e) => {
                    log::error!("Error received: {}", e);
                    self.update_popup(PopupActivityMsg::ShowError(e))
                }
                Msg::ClipboardError(error_msg) => {
                    self.error_reporter
                        .report_clipboard_error("copy_to_clipboard", &error_msg);
                    None
                }
                Msg::ToggleHelpScreen => self.update_help(),
                Msg::ToggleThemePicker => {
                    if let Err(e) = self.mount_theme_picker() {
                        self.error_reporter
                            .report_mount_error("ThemePicker", "mount", e);
                        None
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(Msg::Error(e)) = result {
                log::error!("Error from message processing: {}", e);
                if let Err(err) = self.mount_error_popup(&e) {
                    self.error_reporter
                        .report_mount_error("ErrorPopup", "mount", err);
                    // Since we can't show the error popup, report the original error through ErrorReporter
                    self.error_reporter
                        .report_simple(e, "MessageProcessing", "handle_update");
                }
                None
            } else {
                result
            }
        } else {
            None
        }
    }
}
