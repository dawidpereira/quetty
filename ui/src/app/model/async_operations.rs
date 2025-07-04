use super::Model;
use crate::components::common::Msg;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Load namespaces using QueueManager
    pub fn load_namespaces(&self) {
        self.queue_manager.load_namespaces();
    }

    /// Load queues using QueueManager
    pub fn load_queues(&self) {
        self.queue_manager.load_queues();
    }

    /// Create new consumer for the selected queue using QueueManager
    pub fn new_consumer_for_queue(&mut self) {
        if let Some(queue) = self.queue_manager.queue_state.pending_queue.clone() {
            self.queue_manager.switch_to_queue(queue);
        }
    }

    /// Force reload messages - useful after bulk operations that modify the queue
    pub fn handle_force_reload_messages(&mut self) -> Option<Msg> {
        log::info!("Force reloading messages after bulk operation with backfill");
        let current_page_size = crate::config::get_current_page_size();
        log::info!("Force reload will use current page size: {current_page_size} messages");

        // Reset pagination state but preserve the knowledge of current page size
        self.reset_pagination_state();
        self.queue_state_mut().messages = None;
        self.queue_state_mut().bulk_selection.clear_all();

        if let Err(e) = self
            .app
            .active(&crate::components::common::ComponentId::Messages)
        {
            log::warn!("Failed to activate messages component during force reload: {e}");
        }

        // Start a reload that will fill the current page size, adding backfill if needed
        self.start_page_reload(current_page_size);

        // Force a full UI redraw to clear stale state
        self.set_redraw(true);
        Some(crate::components::common::Msg::ForceRedraw)
    }

    /// Start a reload that ensures we have `target_page_size` messages in memory for the first page.
    fn start_page_reload(&self, target_page_size: u32) {
        let service_bus_manager = self.service_bus_manager.clone();
        let tx_to_main = self.state_manager.tx_to_main.clone();

        self.task_manager
            .execute("Reloading messages...", async move {
                log::info!("Starting page reload with target page size: {target_page_size}");

                let result = Self::execute_reload_task(
                    tx_to_main.clone(),
                    service_bus_manager,
                    target_page_size,
                )
                .await;

                // Always send a message to clear loading state, even on error
                if let Err(e) = &result {
                    log::error!("Error in reload task: {e}");
                    // Send empty message list to clear loading state
                    let _ = tx_to_main.send(crate::components::common::Msg::MessageActivity(
                        crate::components::common::MessageActivityMsg::MessagesLoaded(Vec::new()),
                    ));
                }
                result
            });
    }

    /// Fetches batches until we have at least `target_page_size` messages or run out.
    async fn execute_reload_task(
        tx_to_main: std::sync::mpsc::Sender<crate::components::common::Msg>,
        service_bus_manager: std::sync::Arc<
            tokio::sync::Mutex<server::service_bus_manager::ServiceBusManager>,
        >,
        target_page_size: u32,
    ) -> Result<(), crate::error::AppError> {
        let mut all_loaded_messages = Vec::new();
        let mut current_sequence: Option<i64> = None;
        let mut total_attempts = 0;
        let max_attempts = 5; // Prevent infinite loops
        let target_size = target_page_size as usize;

        log::info!("Page reload targeting {target_size} messages");

        // Progressive loading loop to handle sequence gaps
        while all_loaded_messages.len() < target_size && total_attempts < max_attempts {
            total_attempts += 1;
            let messages_needed = target_size - all_loaded_messages.len();

            // Always request only as many messages as still needed (up to a single page).
            let load_count = messages_needed;

            log::debug!(
                "Page reload attempt {total_attempts}: need {messages_needed} more messages, loading {load_count} (from_sequence: {current_sequence:?})"
            );

            let batch_messages =
                Self::fetch_batch(&service_bus_manager, current_sequence, load_count as u32)
                    .await?;

            if batch_messages.is_empty() {
                log::info!(
                    "Page reload reached end of queue with {} messages (target was {})",
                    all_loaded_messages.len(),
                    target_size
                );
                break;
            }

            // Update sequence for next iteration
            if let Some(last_msg) = batch_messages.last() {
                current_sequence = Some(last_msg.sequence + 1);
            }

            all_loaded_messages.extend(batch_messages);

            log::debug!(
                "Page reload progress: {} messages loaded (target: {})",
                all_loaded_messages.len(),
                target_size
            );

            // If we've reached our target or exceeded it, we can stop
            if all_loaded_messages.len() >= target_size {
                log::info!(
                    "Page reload reached target: {} messages loaded (target: {})",
                    all_loaded_messages.len(),
                    target_size
                );
                break;
            }
        }

        // Take only the target amount of messages if we loaded more
        if all_loaded_messages.len() > target_size {
            all_loaded_messages.truncate(target_size);
            log::debug!(
                "Page reload truncated to {target_size} messages to match target page size"
            );
        }

        log::info!(
            "Page reload completed: loaded {} messages (target: {}, attempts: {})",
            all_loaded_messages.len(),
            target_size,
            total_attempts
        );

        // Send the loaded messages using the standard message format
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::MessageActivity(
            crate::components::common::MessageActivityMsg::MessagesLoaded(all_loaded_messages),
        )) {
            log::error!("Failed to send page reload messages: {e}");
            return Err(crate::error::AppError::Component(e.to_string()));
        }

        Ok(())
    }

    /// Helper: fetch a single batch of messages
    async fn fetch_batch(
        service_bus_manager: &std::sync::Arc<
            tokio::sync::Mutex<server::service_bus_manager::ServiceBusManager>,
        >,
        from_sequence: Option<i64>,
        max_count: u32,
    ) -> Result<Vec<server::model::MessageModel>, crate::error::AppError> {
        let command = server::service_bus_manager::ServiceBusCommand::PeekMessages {
            max_count,
            from_sequence,
        };

        match service_bus_manager
            .lock()
            .await
            .execute_command(command)
            .await
        {
            server::service_bus_manager::ServiceBusResponse::MessagesReceived { messages } => {
                log::debug!("Page reload received {} messages in batch", messages.len());
                Ok(messages)
            }
            server::service_bus_manager::ServiceBusResponse::Error { error } => {
                Err(crate::error::AppError::ServiceBus(error.to_string()))
            }
            _ => Err(crate::error::AppError::ServiceBus(
                "Unexpected response for page reload peek messages".to_string(),
            )),
        }
    }
}
