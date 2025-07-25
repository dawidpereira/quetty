use crate::app::model::{AppState, Model};
use crate::components::common::{Msg, QueueActivityMsg};
use crate::constants::env_vars::*;
use crate::error::AppError;
use quetty_server::service_bus_manager::{ServiceBusCommand, ServiceBusResponse};
use std::env;
use std::sync::Mutex;
use tuirealm::terminal::TerminalAdapter;

/// Thread-safe environment variable management
/// This provides a safe wrapper around env::set_var to prevent data races
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Safe wrapper for setting environment variables
/// This prevents data races by using a mutex lock and handles lock poisoning
fn safe_set_env_var(key: &str, value: &str) -> crate::error::AppResult<()> {
    let _lock = ENV_LOCK.lock().map_err(|e| {
        crate::error::AppError::State(format!("Environment variable lock poisoned: {e}"))
    })?;
    unsafe {
        env::set_var(key, value);
    }
    Ok(())
}

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Helper method to save queue name for connection string auth
    /// This handles both environment variable and .env file persistence
    fn save_queue_name_for_connection_string_auth(&mut self, queue_name: &str) {
        let config = crate::config::get_config_or_panic();
        if config.azure_ad().auth_method == crate::utils::auth::AUTH_METHOD_CONNECTION_STRING {
            log::info!("Saving queue name '{queue_name}' for connection string auth");

            // Save to environment variable
            if let Err(e) = safe_set_env_var(SERVICEBUS_QUEUE_NAME, queue_name) {
                log::error!("Failed to save queue name to environment: {e}");
                return;
            }

            // Also persist to .env file immediately
            let config_data = crate::components::common::ConfigUpdateData {
                auth_method: crate::utils::auth::AUTH_METHOD_CONNECTION_STRING.to_string(),
                tenant_id: None,
                client_id: None,
                client_secret: None,
                subscription_id: None,
                resource_group: None,
                namespace: None,
                connection_string: None,
                master_password: None,
                queue_name: Some(queue_name.to_string()),
            };

            if let Err(e) = self.write_env_file(&config_data) {
                log::error!("Failed to persist queue name to .env file: {e}");
            } else {
                log::info!("Queue name persisted to .env file");
            }
        }
    }
    /// Handle queue-related messages from the UI
    ///
    /// Processes queue selection, switching, unselection, and exit operations.
    /// This includes initializing consumers, managing queue state transitions,
    /// handling discovery mode, and resource cleanup.
    ///
    /// # Arguments
    /// * `msg` - The queue activity message to process
    ///
    /// # Returns
    /// * `Some(Msg)` - Next UI action to take
    /// * `None` - No further action needed
    pub fn update_queue(&mut self, msg: QueueActivityMsg) -> Option<Msg> {
        match msg {
            QueueActivityMsg::QueueSelected(queue) => {
                log::info!("Queue selected: '{queue}' - initializing queue and loading statistics");
                self.queue_state_mut().set_selected_queue(queue);
                self.new_consumer_for_queue();

                // Load stats for the newly selected queue
                self.load_stats_for_current_queue();

                None
            }
            QueueActivityMsg::QueueSelectedFromManualEntry(queue) => {
                // First exit editing mode, then select the queue
                self.set_editing_message(false);
                if let Err(e) = self.update_global_key_watcher_editing_state() {
                    self.error_reporter.report_key_watcher_error(e);
                }

                // Save queue name for connection string auth only
                self.save_queue_name_for_connection_string_auth(&queue);

                // Now select the queue
                self.queue_state_mut().set_selected_queue(queue);
                self.new_consumer_for_queue();

                // Load stats for the newly selected queue
                self.load_stats_for_current_queue();

                None
            }
            QueueActivityMsg::QueuesLoaded(queues) => {
                // Stop loading indicator when queues are loaded
                if let Some(_msg) =
                    self.update_loading(crate::components::common::LoadingActivityMsg::Stop)
                {
                    log::debug!("Loading stopped after queues loaded");
                }

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
                let Some(service_bus_manager) = self.service_bus_manager.clone() else {
                    log::warn!("Service bus manager not initialized");
                    return None;
                };
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

                // Check if we're in discovery mode
                if let (
                    Some(subscription_id),
                    Some(resource_group),
                    Some(namespace),
                    Some(auth_service),
                ) = (
                    &self.state_manager.selected_subscription,
                    &self.state_manager.selected_resource_group,
                    &self.state_manager.selected_namespace,
                    &self.auth_service,
                ) {
                    log::info!("In discovery mode - loading queues with discovered values");
                    self.queue_manager.load_queues_with_discovery(
                        subscription_id.clone(),
                        resource_group.clone(),
                        namespace.clone(),
                        auth_service.clone(),
                        self.http_client.clone(),
                    );
                } else {
                    log::info!("Using regular queue loading");
                    self.load_queues();
                }
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
                let Some(service_bus_manager) = self.service_bus_manager.clone() else {
                    log::warn!("Service bus manager not initialized");
                    return None;
                };
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

                        let queues = quetty_server::service_bus_manager::ServiceBusManager::list_queues_azure_ad(
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
