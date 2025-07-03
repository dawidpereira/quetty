use crate::app::model::{AppState, Model};
use crate::components::common::{ComponentId, LoadingActivityMsg, Msg};
use crate::components::state::ComponentStateMount;
use tuirealm::terminal::TerminalAdapter;
use tuirealm::{Sub, SubClause, SubEventClause};

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
                    self.error_reporter
                        .report_mount_error("LoadingIndicator", "mount", &e);
                }

                self.set_app_state(AppState::Loading);
                self.set_redraw(true);
                None
            }
            LoadingActivityMsg::Stop => {
                log::debug!("Stopping loading");

                // Clear cancel button state
                self.state_manager.loading_cancel_button = None;

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
                        self.error_reporter
                            .report_mount_error("LoadingIndicator", "unmount", &e);
                    } else {
                        log::debug!("Loading indicator unmounted successfully");
                    }
                }

                self.set_redraw(true);
                None
            }
            LoadingActivityMsg::Update(progress_message) => {
                log::debug!("Updating loading progress: {}", progress_message);

                // For progress updates, we'll store the progress and remount the loading indicator
                // This is simpler than trying to access component internals through tuirealm
                if self.app.mounted(&ComponentId::LoadingIndicator) {
                    if let Some((base_message, _)) = &self.state_manager.loading_message {
                        let mut updated_indicator =
                            crate::components::loading_indicator::LoadingIndicator::new(
                                base_message,
                                true,
                            );
                        updated_indicator.update_progress(progress_message);

                        // Preserve cancel button state if it was previously shown
                        if let Some(ref operation_id) = self.state_manager.loading_cancel_button {
                            updated_indicator.show_cancel_button(operation_id.clone());
                        }

                        if let Err(e) = self.app.remount_with_state(
                            ComponentId::LoadingIndicator,
                            updated_indicator,
                            vec![
                                Sub::new(SubEventClause::Tick, SubClause::Always),
                                Sub::new(SubEventClause::Any, SubClause::Always),
                            ],
                        ) {
                            log::error!("Failed to remount loading indicator with progress: {}", e);
                        }
                    }
                }

                self.set_redraw(true);
                None
            }
            LoadingActivityMsg::ShowCancelButton(operation_id) => {
                log::debug!("Showing cancel button for operation: {}", operation_id);

                // Store the cancel button state
                self.state_manager.loading_cancel_button = Some(operation_id.clone());

                // For cancel button, remount with cancel button enabled
                if self.app.mounted(&ComponentId::LoadingIndicator) {
                    if let Some((base_message, _)) = &self.state_manager.loading_message {
                        let mut updated_indicator =
                            crate::components::loading_indicator::LoadingIndicator::new(
                                base_message,
                                true,
                            );
                        updated_indicator.show_cancel_button(operation_id);

                        if let Err(e) = self.app.remount_with_state(
                            ComponentId::LoadingIndicator,
                            updated_indicator,
                            vec![
                                Sub::new(SubEventClause::Tick, SubClause::Always),
                                Sub::new(SubEventClause::Any, SubClause::Always),
                            ],
                        ) {
                            log::error!(
                                "Failed to remount loading indicator with cancel button: {}",
                                e
                            );
                        }
                    }
                }

                self.set_redraw(true);
                None
            }
            LoadingActivityMsg::HideCancelButton => {
                log::debug!("Hiding cancel button");

                // Clear the cancel button state
                self.state_manager.loading_cancel_button = None;

                // For hiding cancel button, remount without cancel button
                if self.app.mounted(&ComponentId::LoadingIndicator) {
                    if let Some((base_message, _)) = &self.state_manager.loading_message {
                        let updated_indicator =
                            crate::components::loading_indicator::LoadingIndicator::new(
                                base_message,
                                true,
                            );

                        if let Err(e) = self.app.remount_with_state(
                            ComponentId::LoadingIndicator,
                            updated_indicator,
                            vec![
                                Sub::new(SubEventClause::Tick, SubClause::Always),
                                Sub::new(SubEventClause::Any, SubClause::Always),
                            ],
                        ) {
                            log::error!(
                                "Failed to remount loading indicator without cancel button: {}",
                                e
                            );
                        }
                    }
                }

                self.set_redraw(true);
                None
            }
            LoadingActivityMsg::Cancel => {
                log::info!("User requested operation cancellation");

                // Get the active operations from task manager and cancel them
                let active_operations = self.task_manager.get_active_operations();
                for operation_id in active_operations {
                    log::info!("Cancelling operation: {}", operation_id);
                    self.task_manager.cancel_operation(&operation_id);

                    // If the user aborted a queue switch, inform the UI so it can roll back
                    if operation_id.starts_with("switch_queue_") {
                        if let Err(e) =
                            self.tx_to_main()
                                .send(crate::components::common::Msg::QueueActivity(
                                crate::components::common::QueueActivityMsg::QueueSwitchCancelled,
                            ))
                        {
                            log::error!("Failed to notify queue switch cancellation: {}", e);
                        }
                    }
                }

                None
            }
        }
    }
}
