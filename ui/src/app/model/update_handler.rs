use super::Model;
use crate::components::common::{MessageActivityMsg, Msg, PopupActivityMsg, QueueActivityMsg};
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Central message handler for the application
    ///
    /// Routes incoming messages to appropriate handlers and manages application state.
    /// This is the main event processing hub that coordinates between UI components,
    /// authentication, configuration, queue management, and other subsystems.
    ///
    /// # Arguments
    /// * `msg` - Optional message to process (None means no operation)
    ///
    /// # Returns
    /// * `Some(Msg)` - Cascading message to process next
    /// * `None` - No further action needed
    pub fn handle_update(&mut self, msg: Option<Msg>) -> Option<Msg> {
        if let Some(msg) = msg {
            // Log all messages during startup
            if self.state_manager.is_authenticating {
                log::debug!("Update during auth: {msg:?}");
            }

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
                Msg::QueueActivity(QueueActivityMsg::ExitQueueConfirmation) => {
                    Some(Msg::PopupActivity(PopupActivityMsg::ShowConfirmation {
                        title: "Exit Queue".to_string(),
                        message: "Are you sure you want to exit the current queue and return to queue selection?".to_string(),
                        on_confirm: Box::new(Msg::QueueActivity(QueueActivityMsg::ExitQueueConfirmed)),
                    }))
                }
                Msg::QueueActivity(msg) => self.update_queue(msg),
                Msg::NamespaceActivity(msg) => self.update_namespace(msg),
                Msg::ThemeActivity(msg) => self.update_theme(msg),
                Msg::LoadingActivity(msg) => self.update_loading(msg),
                Msg::PopupActivity(msg) => self.update_popup(msg),
                Msg::Error(e) => {
                    log::error!("Error received: {e}");
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
                Msg::ToggleConfigScreen => {
                    if let Err(e) = self.mount_config_screen() {
                        self.error_reporter
                            .report_mount_error("ConfigScreen", "mount", e);
                        None
                    } else {
                        None
                    }
                }
                Msg::TogglePasswordPopup => {
                    if let Err(e) = self.mount_password_popup(None) {
                        self.error_reporter
                            .report_mount_error("PasswordPopup", "mount", e);
                        None
                    } else {
                        None
                    }
                }
                Msg::ConfigActivity(msg) => {
                    match self.update_config(msg) {
                        Ok(result) => result,
                        Err(e) => {
                            self.error_reporter
                                .report_mount_error("ConfigScreen", "update", e);
                            None
                        }
                    }
                }
                Msg::SetEditingMode(editing) => {
                    self.set_editing_message(editing);
                    if let Err(e) = self.update_global_key_watcher_editing_state() {
                        self.error_reporter.report_key_watcher_error(e);
                    }
                    None
                }
                Msg::AuthActivity(msg) => {
                    match self.update_auth(msg) {
                        Ok(next_msg) => next_msg,
                        Err(e) => {
                            self.error_reporter.report_error(e);
                            None
                        }
                    }
                }
                Msg::SubscriptionSelection(msg) => self.handle_subscription_selection(msg),
                Msg::ResourceGroupSelection(msg) => self.handle_resource_group_selection(msg),
                Msg::AzureDiscovery(msg) => self.handle_azure_discovery(msg),
                Msg::SetServiceBusManager(manager) => {
                    log::info!("Setting Service Bus manager in queue manager and model");

                    // Clear statistics cache when authentication method changes
                    self.queue_manager.queue_state.stats_manager.clear_all_cache();

                    self.queue_manager.set_service_bus_manager(manager.clone());
                    self.service_bus_manager = Some(manager);
                    None
                }
                Msg::ShowError(error_msg) => {
                    self.update_popup(PopupActivityMsg::ShowError(crate::error::AppError::Component(error_msg)))
                }
                Msg::ShowSuccess(success_msg) => {
                    self.update_popup(PopupActivityMsg::ShowSuccess(success_msg))
                }
                Msg::Tick => {
                    // Tick is used to refresh components that need periodic updates
                    // Currently only used for the auth popup timer
                    None
                }

                Msg::ForceRedraw => {
                    // Force a screen redraw
                    self.set_redraw(true);
                    None
                }
            };

            if let Some(Msg::Error(e)) = result {
                log::error!("Error from message processing: {e}");
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
