use crate::app::model::{AppState, Model};
use crate::components::common::{Msg, QueueActivityMsg};
use crate::error::AppError;
use server::service_bus_manager::{ServiceBusCommand, ServiceBusResponse};
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn update_queue(&mut self, msg: QueueActivityMsg) -> Option<Msg> {
        match msg {
            QueueActivityMsg::QueueSelected(queue) => {
                self.queue_state_mut().set_selected_queue(queue);
                self.new_consumer_for_queue();

                // Load stats for the newly selected queue
                self.load_stats_for_current_queue();

                None
            }
            QueueActivityMsg::QueuesLoaded(queues) => {
                if let Err(e) = self.remount_queue_picker(Some(queues)) {
                    self.error_reporter
                        .report_simple(e, "QueueHandler", "update_queue");
                    return None;
                }
                None
            }
            QueueActivityMsg::QueueUnselected => {
                self.set_app_state(AppState::QueuePicker);
                None
            }
            QueueActivityMsg::ToggleDeadLetterQueue => {
                if self.queue_state_mut().toggle_queue_type().is_some() {
                    log::info!("Toggled queue type, switching consumer");
                    self.new_consumer_for_queue();

                    // Load stats for the toggled queue type
                    self.load_stats_for_current_queue();
                }
                None
            }
            QueueActivityMsg::ExitQueueConfirmed => {
                log::info!("Exiting current queue and returning to queue selection");

                // Check for active operations before proceeding
                let active_ops = self.task_manager.get_active_operations();
                if !active_ops.is_empty() {
                    log::warn!("Cannot exit queue while operations are running: {active_ops:?}");
                    self.error_reporter.report_simple(
                        AppError::State("Cannot exit queue while operations are in progress. Please wait for current operations to complete.".to_string()),
                        "QueueHandler",
                        "exit_queue_blocked"
                    );
                    return None;
                }

                // Clear any pending or current queue selections and cached messages
                let qs = self.queue_state_mut();
                qs.pending_queue = None;
                qs.current_queue_name = None;
                qs.messages = None;
                qs.message_pagination.reset();
                qs.bulk_selection.clear_all();

                // Dispose all backend resources using task manager with progress tracking
                let service_bus_manager = self.service_bus_manager.clone();
                let error_reporter = self.error_reporter.clone();
                let tx_to_main = self.tx_to_main().clone();

                self.task_manager.execute_with_progress(
                    "Disposing resources and exiting queue...",
                    "dispose_resources_and_exit",
                    move |progress| {
                        Box::pin(async move {
                            progress.report_progress("Disposing Service Bus resources...");

                            match service_bus_manager
                                .lock()
                                .await
                                .execute_command(ServiceBusCommand::DisposeAllResources)
                                .await
                            {
                                ServiceBusResponse::AllResourcesDisposed => {
                                    log::info!("All Service Bus resources disposed successfully");
                                    progress.report_progress(
                                        "Resources disposed, returning to namespace selection...",
                                    );

                                    // Send message to complete the exit process
                                    if let Err(e) = tx_to_main.send(Msg::QueueActivity(
                                        QueueActivityMsg::ExitQueueFinalized,
                                    )) {
                                        error_reporter.report_send_error("exit_queue_finalized", e);
                                        return Err(AppError::State(
                                            "Failed to complete queue exit".to_string(),
                                        ));
                                    }

                                    Ok(())
                                }
                                ServiceBusResponse::Error { error } => {
                                    let app_error = AppError::from(error);
                                    error_reporter.report_simple(
                                        app_error.clone(),
                                        "QueueHandler",
                                        "dispose_resources",
                                    );
                                    Err(app_error)
                                }
                                _ => {
                                    let error = AppError::ServiceBus(
                                        "Unexpected response from dispose all resources"
                                            .to_string(),
                                    );
                                    log::warn!("Unexpected response from dispose all resources");
                                    Err(error)
                                }
                            }
                        })
                    },
                );

                None
            }
            QueueActivityMsg::ExitQueueFinalized => {
                log::info!("Finalizing queue exit - returning to queue selection");

                // Load queues to return to queue picker
                self.load_queues();
                None
            }
            QueueActivityMsg::QueueSwitchCancelled => {
                log::info!("Queue switch cancelled by user – reverting to queue picker");

                // Clear any pending or current queue selections and cached messages
                let qs = self.queue_state_mut();
                qs.pending_queue = None;
                qs.current_queue_name = None;
                qs.messages = None;
                qs.message_pagination.reset();

                // Dispose all backend resources first, then reload queue list in sequence
                let service_bus_manager = self.service_bus_manager.clone();
                let task_manager = self.task_manager.clone();
                let tx_to_main = self.tx_to_main().clone();

                self.task_manager.execute("Resetting connection and reloading...", async move {
                    // Reset the entire AMQP connection to clear corrupted session state
                    match service_bus_manager
                        .lock()
                        .await
                        .execute_command(ServiceBusCommand::ResetConnection)
                        .await
                    {
                        ServiceBusResponse::ConnectionReset => {
                            log::info!("Connection reset successfully after cancellation");
                        }
                        ServiceBusResponse::Error { error } => {
                            log::error!("Failed to reset connection after cancellation: {error}");
                            // Continue anyway to reload queues
                        }
                        _ => {
                            log::warn!("Unexpected response from connection reset");
                        }
                    }

                    // Now reload the queue list to ensure a fresh picker state
                    log::info!("Reloading queue list after resource disposal");
                    task_manager.execute("Loading queues...", async move {
                        log::debug!("Requesting queues from Azure AD after cancellation");

                        let queues = server::service_bus_manager::ServiceBusManager::list_queues_azure_ad(
                            crate::config::get_config_or_panic().azure_ad(),
                        )
                        .await
                        .map_err(|e| {
                            log::error!("Failed to list queues after cancellation: {e}");
                            AppError::ServiceBus(e.to_string())
                        })?;

                        log::info!("Loaded {} queues after cancellation", queues.len());

                        // Send loaded queues to update the picker
                        if let Err(e) = tx_to_main.send(Msg::QueueActivity(QueueActivityMsg::QueuesLoaded(queues))) {
                            log::error!("Failed to send queues loaded message after cancellation: {e}");
                            return Err(AppError::Component(e.to_string()));
                        }

                        Ok(())
                    });

                    Ok(())
                });

                self.set_app_state(AppState::QueuePicker);
                None
            }
            QueueActivityMsg::ExitQueueConfirmation => {
                // This message is handled by update_handler to show the confirmation popup
                // No further action needed here
                None
            }
        }
    }

    /// Load statistics for current queue - check cache first, then API if needed
    fn load_stats_for_current_queue(&mut self) {
        let queue_name = self
            .queue_state()
            .current_queue_name
            .clone()
            .unwrap_or_default();
        let base_queue_name = if queue_name.ends_with("/$deadletterqueue") {
            queue_name.trim_end_matches("/$deadletterqueue").to_string()
        } else {
            queue_name
        };

        // Check if we have valid cache
        if self
            .queue_state()
            .stats_manager
            .has_valid_cache(&base_queue_name)
        {
            log::info!("Using cached stats for queue: {base_queue_name}");
            // Cache is valid - stats will be displayed immediately in UI
            return;
        }

        log::info!("No valid cache for queue: {base_queue_name}, loading from API");

        // No valid cache - load from API in background
        if let Err(e) = self.load_queue_statistics_from_api(&base_queue_name) {
            log::error!("Failed to load queue statistics: {e}");
        }
    }
}
