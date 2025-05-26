use crate::app::model::{AppState, Model};
use crate::components::common::{MessageActivityMsg, Msg};
use server::consumer::Consumer;
use server::model::MessageModel;
use server::producer::{Producer, ServiceBusClientProducerExt};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tuirealm::terminal::TerminalAdapter;

/// Dedicated type for managing message pagination state
#[derive(Debug, Clone, Default)]
pub struct MessagePaginationState {
    pub current_page: usize,
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub total_pages_loaded: usize,
    pub last_loaded_sequence: Option<i64>,
    pub all_loaded_messages: Vec<MessageModel>,
}

impl MessagePaginationState {
    /// Check if a specific page is already loaded
    pub fn is_page_loaded(&self, page: usize) -> bool {
        page < self.total_pages_loaded
    }

    /// Calculate page bounds for the current page
    pub fn calculate_page_bounds(&self, page_size: usize) -> (usize, usize) {
        let start_idx = self.current_page * page_size;
        let end_idx = std::cmp::min(start_idx + page_size, self.all_loaded_messages.len());
        (start_idx, end_idx)
    }

    /// Reset pagination state to initial values
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Update pagination state based on current page and total loaded pages
    pub fn update(&mut self, page_size: usize) {
        self.has_previous_page = self.current_page > 0;
        self.has_next_page = self.calculate_has_next_page(page_size);
    }

    /// Calculate if there's a next page available
    fn calculate_has_next_page(&self, page_size: usize) -> bool {
        let next_page_exists = self.current_page + 1 < self.total_pages_loaded;
        let might_have_more_to_load =
            self.total_pages_loaded > 0 && self.all_loaded_messages.len() % page_size == 0;

        next_page_exists || might_have_more_to_load
    }

    /// Move to the next page
    pub fn advance_to_next_page(&mut self) {
        self.current_page += 1;
    }

    /// Move to the previous page
    pub fn go_to_previous_page(&mut self) {
        if self.current_page > 0 {
            self.current_page -= 1;
        }
    }

    /// Set the current page directly
    pub fn set_current_page(&mut self, page: usize) {
        self.current_page = page;
    }

    /// Increment total pages loaded and update last sequence
    pub fn add_loaded_page(&mut self, new_messages: Vec<MessageModel>) {
        self.all_loaded_messages.extend(new_messages);
        self.total_pages_loaded += 1;

        if let Some(last_msg) = self.all_loaded_messages.last() {
            self.last_loaded_sequence = Some(last_msg.sequence);
        }
    }

    /// Get current page messages
    pub fn get_current_page_messages(&self, page_size: usize) -> Vec<MessageModel> {
        let (start_idx, end_idx) = self.calculate_page_bounds(page_size);

        if start_idx < self.all_loaded_messages.len() {
            self.all_loaded_messages[start_idx..end_idx].to_vec()
        } else {
            Vec::new()
        }
    }

    /// Remove a message by ID and sequence from the loaded messages and adjust pagination
    pub fn remove_message_by_id_and_sequence(
        &mut self,
        message_id: &str,
        message_sequence: i64,
        page_size: usize,
    ) -> bool {
        // Find the message index in all loaded messages by both ID and sequence
        if let Some(global_index) = self
            .all_loaded_messages
            .iter()
            .position(|msg| msg.id == message_id && msg.sequence == message_sequence)
        {
            // Remove the message
            self.all_loaded_messages.remove(global_index);

            // Adjust pagination based on the removal
            self.adjust_pagination_after_removal(global_index, page_size);

            true
        } else {
            false
        }
    }

    /// Adjust pagination state after a message is removed
    fn adjust_pagination_after_removal(&mut self, removed_global_index: usize, page_size: usize) {
        let current_page_start = self.current_page * page_size;

        // If the removed message was before the current page, no adjustment needed
        if removed_global_index < current_page_start {
            return;
        }

        // If the removed message was on the current page or after
        if removed_global_index >= current_page_start {
            // Check if current page now has fewer messages than expected
            let messages_on_current_page = self.get_current_page_messages(page_size).len();

            // If current page is empty and we're not on page 0, go back one page
            if messages_on_current_page == 0 && self.current_page > 0 {
                self.current_page -= 1;
            }
        }

        // Update pagination flags
        self.update(page_size);
    }
}

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn update_messages(&mut self, msg: MessageActivityMsg) -> Option<Msg> {
        match msg {
            MessageActivityMsg::EditMessage(index) => self.handle_edit_message(index),
            MessageActivityMsg::CancelEditMessage => self.handle_cancel_edit_message(),
            MessageActivityMsg::MessagesLoaded(messages) => self.handle_messages_loaded(messages),
            MessageActivityMsg::ConsumerCreated(consumer) => self.handle_consumer_created(consumer),
            MessageActivityMsg::QueueNameUpdated(queue_name) => {
                self.handle_queue_name_updated(queue_name)
            }
            MessageActivityMsg::PreviewMessageDetails(index) => {
                self.handle_preview_message_details(index)
            }
            MessageActivityMsg::NextPage => self.handle_next_page_request(),
            MessageActivityMsg::PreviousPage => self.handle_previous_page_request(),
            MessageActivityMsg::NewMessagesLoaded(new_messages) => {
                self.handle_new_messages_loaded(new_messages)
            }
            MessageActivityMsg::PageChanged => self.handle_page_changed(),
            MessageActivityMsg::PaginationStateUpdated {
                has_next,
                has_previous,
                current_page,
                total_pages_loaded,
            } => self.handle_pagination_state_updated(
                has_next,
                has_previous,
                current_page,
                total_pages_loaded,
            ),
            MessageActivityMsg::SendMessageToDLQ(index) => self.handle_send_message_to_dlq(index),
            MessageActivityMsg::ResendMessageFromDLQ(index) => {
                self.handle_resend_message_from_dlq(index)
            }
            MessageActivityMsg::RemoveMessageFromState(message_id, message_sequence) => {
                self.handle_remove_message_from_state(message_id, message_sequence)
            }
        }
    }

    // Message state management methods
    fn handle_edit_message(&mut self, index: usize) -> Option<Msg> {
        if let Err(e) = self.remount_message_details(index) {
            return Some(Msg::Error(e));
        }
        self.app_state = AppState::MessageDetails;
        Some(Msg::ForceRedraw)
    }

    fn handle_cancel_edit_message(&mut self) -> Option<Msg> {
        self.app_state = AppState::MessagePicker;
        None
    }

    fn handle_messages_loaded(&mut self, messages: Vec<MessageModel>) -> Option<Msg> {
        self.queue_state.messages = Some(messages);
        if let Err(e) = self.remount_messages() {
            return Some(Msg::Error(e));
        }
        if let Err(e) = self.remount_message_details(0) {
            return Some(Msg::Error(e));
        }
        self.app_state = AppState::MessagePicker;
        None
    }

    fn handle_consumer_created(&mut self, consumer: Consumer) -> Option<Msg> {
        self.queue_state.consumer = Some(Arc::new(Mutex::new(consumer)));

        // Update current_queue_name to match the queue that was actually used to create the consumer
        if let Some(pending_queue) = &self.queue_state.pending_queue {
            self.queue_state.current_queue_name = Some(pending_queue.clone());
        }

        self.reset_pagination_state();
        if let Err(e) = self.load_messages() {
            return Some(Msg::Error(e));
        }
        None
    }

    fn handle_preview_message_details(&mut self, index: usize) -> Option<Msg> {
        if let Err(e) = self.remount_message_details(index) {
            return Some(Msg::Error(e));
        }
        None
    }

    fn handle_new_messages_loaded(&mut self, new_messages: Vec<MessageModel>) -> Option<Msg> {
        let is_initial_load = self
            .queue_state
            .message_pagination
            .all_loaded_messages
            .is_empty();

        // Add new messages to pagination state
        self.queue_state
            .message_pagination
            .add_loaded_page(new_messages);

        // If this is not the initial load, advance to the new page
        if !is_initial_load {
            self.queue_state.message_pagination.advance_to_next_page();
        }

        // Update the current page view
        if let Err(e) = self.update_current_page_view() {
            return Some(Msg::Error(e));
        }

        // Ensure we're in the right state and have message details
        if self.app_state != AppState::MessagePicker {
            self.app_state = AppState::MessagePicker;
        }

        // Initialize message details if we have messages
        if !self
            .queue_state
            .message_pagination
            .all_loaded_messages
            .is_empty()
        {
            if let Err(e) = self.remount_message_details(0) {
                return Some(Msg::Error(e));
            }
        }

        None
    }

    fn handle_page_changed(&mut self) -> Option<Msg> {
        if let Err(e) = self.update_current_page_view() {
            return Some(Msg::Error(e));
        }
        None
    }

    fn handle_pagination_state_updated(
        &mut self,
        has_next: bool,
        has_previous: bool,
        current_page: usize,
        total_pages_loaded: usize,
    ) -> Option<Msg> {
        self.queue_state.message_pagination.has_next_page = has_next;
        self.queue_state.message_pagination.has_previous_page = has_previous;
        self.queue_state.message_pagination.current_page = current_page;
        self.queue_state.message_pagination.total_pages_loaded = total_pages_loaded;
        None
    }

    // Pagination request handlers
    fn handle_next_page_request(&mut self) -> Option<Msg> {
        if self.queue_state.message_pagination.has_next_page {
            if let Err(e) = self.handle_next_page() {
                return Some(Msg::Error(e));
            }
        }
        None
    }

    fn handle_previous_page_request(&mut self) -> Option<Msg> {
        if self.queue_state.message_pagination.has_previous_page {
            if let Err(e) = self.handle_previous_page() {
                return Some(Msg::Error(e));
            }
        }
        None
    }

    // Pagination navigation methods
    pub fn handle_next_page(&mut self) -> crate::error::AppResult<()> {
        log::debug!(
            "Handle next page - current: {}, total_loaded: {}",
            self.queue_state.message_pagination.current_page,
            self.queue_state.message_pagination.total_pages_loaded
        );

        let next_page = self.queue_state.message_pagination.current_page + 1;

        if self
            .queue_state
            .message_pagination
            .is_page_loaded(next_page)
        {
            self.switch_to_loaded_page(next_page);
        } else {
            log::debug!("Loading new page {} from API", next_page);
            self.load_new_messages_from_api()?;
        }

        Ok(())
    }

    pub fn handle_previous_page(&mut self) -> crate::error::AppResult<()> {
        log::debug!(
            "Handle previous page - current: {}",
            self.queue_state.message_pagination.current_page
        );

        if self.queue_state.message_pagination.current_page > 0 {
            self.queue_state.message_pagination.go_to_previous_page();
            self.update_pagination_state();
            self.send_page_changed_message();
        }

        Ok(())
    }

    // Pagination utility methods
    fn reset_pagination_state(&mut self) {
        self.queue_state.message_pagination.reset();
    }

    fn switch_to_loaded_page(&mut self, page: usize) {
        log::debug!("Page {} already loaded, switching view", page);
        self.queue_state.message_pagination.set_current_page(page);
        self.update_pagination_state();
        self.send_page_changed_message();
    }

    fn send_page_changed_message(&self) {
        if let Err(e) = self
            .tx_to_main
            .send(crate::components::common::Msg::MessageActivity(
                crate::components::common::MessageActivityMsg::PageChanged,
            ))
        {
            log::error!("Failed to send page changed message: {}", e);
        }
    }

    fn update_pagination_state(&mut self) {
        let page_size = crate::config::CONFIG.max_messages() as usize;
        self.queue_state.message_pagination.update(page_size);

        log::debug!(
            "Updated pagination state: current={}, total_loaded={}, has_prev={}, has_next={}",
            self.queue_state.message_pagination.current_page,
            self.queue_state.message_pagination.total_pages_loaded,
            self.queue_state.message_pagination.has_previous_page,
            self.queue_state.message_pagination.has_next_page
        );
    }

    fn update_current_page_view(&mut self) -> crate::error::AppResult<()> {
        let page_size = crate::config::CONFIG.max_messages() as usize;
        let current_page_messages = self
            .queue_state
            .message_pagination
            .get_current_page_messages(page_size);
        let (start_idx, end_idx) = self
            .queue_state
            .message_pagination
            .calculate_page_bounds(page_size);

        log::debug!(
            "Updating view for page {}: showing messages {}-{} of {}",
            self.queue_state.message_pagination.current_page,
            start_idx,
            end_idx,
            self.queue_state
                .message_pagination
                .all_loaded_messages
                .len()
        );

        self.queue_state.messages = Some(current_page_messages);
        self.update_pagination_state();
        self.send_pagination_state_update()?;
        self.remount_messages()?;

        Ok(())
    }

    fn send_pagination_state_update(&self) -> crate::error::AppResult<()> {
        self.tx_to_main
            .send(crate::components::common::Msg::MessageActivity(
                crate::components::common::MessageActivityMsg::PaginationStateUpdated {
                    has_next: self.queue_state.message_pagination.has_next_page,
                    has_previous: self.queue_state.message_pagination.has_previous_page,
                    current_page: self.queue_state.message_pagination.current_page,
                    total_pages_loaded: self.queue_state.message_pagination.total_pages_loaded,
                },
            ))
            .map_err(|e| {
                log::error!("Failed to send pagination state update: {}", e);
                crate::error::AppError::Component(e.to_string())
            })
    }

    fn load_new_messages_from_api(&mut self) -> crate::error::AppResult<()> {
        log::debug!(
            "Loading new messages from API, last_sequence: {:?}",
            self.queue_state.message_pagination.last_loaded_sequence
        );

        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        self.send_loading_start_message(&tx_to_main);

        let consumer = self.get_consumer()?;
        let tx_to_main_err = tx_to_main.clone();
        let from_sequence = self
            .queue_state
            .message_pagination
            .last_loaded_sequence
            .map(|seq| seq + 1);

        taskpool.execute(async move {
            Self::execute_message_loading_task(tx_to_main, tx_to_main_err, consumer, from_sequence)
                .await;
        });

        Ok(())
    }

    fn send_loading_start_message(&self, tx_to_main: &Sender<crate::components::common::Msg>) {
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Start(
                "Loading more messages...".to_string(),
            ),
        )) {
            log::error!("Failed to send loading start message: {}", e);
        }
    }

    fn get_consumer(&self) -> crate::error::AppResult<Arc<Mutex<Consumer>>> {
        self.queue_state.consumer.clone().ok_or_else(|| {
            log::error!("No consumer available");
            crate::error::AppError::State("No consumer available".to_string())
        })
    }

    async fn execute_message_loading_task(
        tx_to_main: Sender<crate::components::common::Msg>,
        tx_to_main_err: Sender<crate::components::common::Msg>,
        consumer: Arc<Mutex<Consumer>>,
        from_sequence: Option<i64>,
    ) {
        let result =
            Self::load_messages_from_consumer(tx_to_main.clone(), consumer, from_sequence).await;

        if let Err(e) = result {
            Self::handle_loading_error(tx_to_main, tx_to_main_err, e);
        }
    }

    async fn load_messages_from_consumer(
        tx_to_main: Sender<crate::components::common::Msg>,
        consumer: Arc<Mutex<Consumer>>,
        from_sequence: Option<i64>,
    ) -> Result<(), crate::error::AppError> {
        let mut consumer = consumer.lock().await;

        let messages = consumer
            .peek_messages(crate::config::CONFIG.max_messages(), from_sequence)
            .await
            .map_err(|e| {
                log::error!("Failed to peek messages: {}", e);
                crate::error::AppError::ServiceBus(e.to_string())
            })?;

        log::info!("Loaded {} new messages from API", messages.len());

        Self::send_loading_stop_message(&tx_to_main);
        Self::send_loaded_messages(&tx_to_main, messages)?;

        Ok(())
    }

    fn send_loading_stop_message(tx_to_main: &Sender<crate::components::common::Msg>) {
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Stop,
        )) {
            log::error!("Failed to send loading stop message: {}", e);
        }
    }

    fn send_loaded_messages(
        tx_to_main: &Sender<crate::components::common::Msg>,
        messages: Vec<MessageModel>,
    ) -> Result<(), crate::error::AppError> {
        if !messages.is_empty() {
            tx_to_main
                .send(crate::components::common::Msg::MessageActivity(
                    crate::components::common::MessageActivityMsg::NewMessagesLoaded(messages),
                ))
                .map_err(|e| {
                    log::error!("Failed to send new messages loaded message: {}", e);
                    crate::error::AppError::Component(e.to_string())
                })?;
        } else {
            Self::send_page_changed_fallback(tx_to_main)?;
        }
        Ok(())
    }

    fn send_page_changed_fallback(
        tx_to_main: &Sender<crate::components::common::Msg>,
    ) -> Result<(), crate::error::AppError> {
        tx_to_main
            .send(crate::components::common::Msg::MessageActivity(
                crate::components::common::MessageActivityMsg::PageChanged,
            ))
            .map_err(|e| {
                log::error!("Failed to send page changed message: {}", e);
                crate::error::AppError::Component(e.to_string())
            })
    }

    fn handle_loading_error(
        tx_to_main: Sender<crate::components::common::Msg>,
        tx_to_main_err: Sender<crate::components::common::Msg>,
        error: crate::error::AppError,
    ) {
        log::error!("Error in message loading task: {}", error);

        Self::send_loading_stop_message(&tx_to_main);
        let _ = tx_to_main_err.send(crate::components::common::Msg::Error(error));
    }

    fn handle_send_message_to_dlq(&mut self, index: usize) -> Option<Msg> {
        // ⚠️ WARNING: DLQ message sending is in development and not recommended for production use

        // Validate the request
        let message = match self.validate_dlq_request(index) {
            Ok(msg) => msg,
            Err(error_msg) => return Some(error_msg),
        };

        // Get required resources
        let consumer = match self.get_consumer_for_dlq() {
            Ok(consumer) => consumer,
            Err(error_msg) => return Some(error_msg),
        };

        // Start the DLQ operation
        self.start_dlq_operation(message, consumer);
        None
    }

    /// Validates that the DLQ request is valid and returns the target message
    fn validate_dlq_request(&self, index: usize) -> Result<server::model::MessageModel, Msg> {
        // Get the message at the specified index
        let message = if let Some(messages) = &self.queue_state.messages {
            if let Some(msg) = messages.get(index) {
                msg.clone()
            } else {
                log::error!("Message index {} out of bounds", index);
                return Err(Msg::Error(crate::error::AppError::State(
                    "Message index out of bounds".to_string(),
                )));
            }
        } else {
            log::error!("No messages available");
            return Err(Msg::Error(crate::error::AppError::State(
                "No messages available".to_string(),
            )));
        };

        // Only allow sending to DLQ from main queue (not from DLQ itself)
        if self.queue_state.current_queue_type != crate::components::common::QueueType::Main {
            log::warn!("Cannot send message to DLQ from dead letter queue");
            return Err(Msg::Error(crate::error::AppError::State(
                "Cannot send message to DLQ from dead letter queue".to_string(),
            )));
        }

        Ok(message)
    }

    /// Gets the consumer for DLQ operations
    fn get_consumer_for_dlq(&self) -> Result<Arc<Mutex<Consumer>>, Msg> {
        match self.queue_state.consumer.clone() {
            Some(consumer) => Ok(consumer),
            None => {
                log::error!("No consumer available");
                Err(Msg::Error(crate::error::AppError::State(
                    "No consumer available".to_string(),
                )))
            }
        }
    }

    /// Starts the DLQ operation in a background task
    fn start_dlq_operation(
        &self,
        message: server::model::MessageModel,
        consumer: Arc<Mutex<Consumer>>,
    ) {
        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        // Show loading indicator
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Start(
                "Sending message to dead letter queue...".to_string(),
            ),
        )) {
            log::error!("Failed to send loading start message: {}", e);
        }

        let tx_to_main_err = tx_to_main.clone();
        let message_id = message.id.clone();
        let message_sequence = message.sequence;

        taskpool.execute(async move {
            let result =
                Self::execute_dlq_operation(consumer, message_id.clone(), message_sequence).await;

            match result {
                Ok(()) => {
                    Self::handle_dlq_success(&tx_to_main, &message_id, message_sequence);
                }
                Err(e) => {
                    Self::handle_dlq_error(&tx_to_main, &tx_to_main_err, e);
                }
            }
        });
    }

    /// Executes the DLQ operation: find and dead letter the target message
    async fn execute_dlq_operation(
        consumer: Arc<Mutex<Consumer>>,
        message_id: String,
        message_sequence: i64,
    ) -> Result<(), crate::error::AppError> {
        let mut consumer = consumer.lock().await;

        // Find the target message
        let target_msg =
            Self::find_target_message(&mut consumer, &message_id, message_sequence).await?;

        // Send the message to dead letter queue
        log::info!("Sending message {} to dead letter queue", message_id);
        consumer
            .dead_letter_message(
                &target_msg,
                Some("Manual dead letter".to_string()),
                Some("Message manually sent to DLQ via Ctrl+D".to_string()),
            )
            .await
            .map_err(|e| {
                log::error!("Failed to dead letter message: {}", e);
                crate::error::AppError::ServiceBus(e.to_string())
            })?;

        log::info!(
            "Successfully sent message {} to dead letter queue",
            message_id
        );

        Ok(())
    }

    /// Finds the target message by receiving messages and searching by ID
    async fn find_target_message(
        consumer: &mut Consumer,
        message_id: &str,
        message_sequence: i64,
    ) -> Result<azservicebus::ServiceBusReceivedMessage, crate::error::AppError> {
        log::debug!(
            "Looking for message with ID {} and sequence {}",
            message_id,
            message_sequence
        );

        let mut attempts = 0;
        let dlq_config = crate::config::CONFIG.dlq();
        let max_attempts = dlq_config.max_attempts();
        let receive_timeout_secs = dlq_config
            .receive_timeout_secs()
            .min(dlq_config.receive_timeout_cap_secs());
        let mut target_message = None;
        let mut other_messages = Vec::new();

        while attempts < max_attempts && target_message.is_none() {
            log::debug!("Attempt {} to find target message", attempts + 1);

            // Add timeout to prevent hanging indefinitely
            let received_messages = match tokio::time::timeout(
                std::time::Duration::from_secs(receive_timeout_secs),
                consumer.receive_messages(5),
            )
            .await
            {
                Ok(Ok(messages)) => messages,
                Ok(Err(e)) => {
                    log::error!("Failed to receive messages: {}", e);
                    return Err(crate::error::AppError::ServiceBus(e.to_string()));
                }
                Err(_) => {
                    log::error!(
                        "Timeout while receiving messages after {} seconds",
                        receive_timeout_secs
                    );
                    return Err(crate::error::AppError::ServiceBus(format!(
                        "Timeout while receiving messages after {} seconds",
                        receive_timeout_secs
                    )));
                }
            };

            if received_messages.is_empty() {
                log::warn!(
                    "No more messages available to receive on attempt {}",
                    attempts + 1
                );
                // If no messages are available, wait a bit before retrying
                tokio::time::sleep(std::time::Duration::from_millis(
                    dlq_config.retry_delay_ms(),
                ))
                .await;
                attempts += 1;
                continue;
            }

            log::debug!(
                "Received {} messages on attempt {}",
                received_messages.len(),
                attempts + 1
            );

            for msg in received_messages {
                if let Some(msg_id) = msg.message_id() {
                    log::debug!(
                        "Checking message ID: {} (looking for: {}), sequence: {} (looking for: {})",
                        msg_id,
                        message_id,
                        msg.sequence_number(),
                        message_sequence
                    );

                    if msg_id == message_id && msg.sequence_number() == message_sequence {
                        log::info!(
                            "Found target message with ID {} and sequence {}",
                            message_id,
                            message_sequence
                        );
                        target_message = Some(msg);
                        break;
                    }
                } else {
                    log::debug!("Message has no ID, sequence: {}", msg.sequence_number());
                }
                other_messages.push(msg);
            }

            attempts += 1;
        }

        // Abandon all the other messages we received but don't want to dead letter
        if !other_messages.is_empty() {
            log::debug!("Abandoning {} other messages", other_messages.len());
            Self::abandon_other_messages(consumer, other_messages).await;
        }

        // Return the target message or error
        target_message.ok_or_else(|| {
            log::error!(
                "Could not find message with ID {} and sequence {} after {} attempts",
                message_id,
                message_sequence,
                attempts
            );
            crate::error::AppError::ServiceBus(format!(
                "Could not find message with ID {} and sequence {} in received messages after {} attempts",
                message_id, message_sequence, attempts
            ))
        })
    }

    /// Abandons messages that were received but are not the target
    async fn abandon_other_messages(
        consumer: &mut Consumer,
        other_messages: Vec<azservicebus::ServiceBusReceivedMessage>,
    ) {
        for msg in other_messages {
            if let Err(e) = consumer.abandon_message(&msg).await {
                let msg_id = msg
                    .message_id()
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                log::warn!("Failed to abandon message {}: {}", msg_id, e);
            }
        }
    }

    /// Handles successful DLQ operation
    fn handle_dlq_success(
        tx_to_main: &Sender<crate::components::common::Msg>,
        message_id: &str,
        message_sequence: i64,
    ) {
        log::info!(
            "DLQ operation completed successfully for message {} (sequence {})",
            message_id,
            message_sequence
        );

        // Stop loading indicator
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Stop,
        )) {
            log::error!("Failed to send loading stop message: {}", e);
        }

        // Remove the message from local state instead of reloading from server
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::MessageActivity(
            crate::components::common::MessageActivityMsg::RemoveMessageFromState(
                message_id.to_string(),
                message_sequence,
            ),
        )) {
            log::error!("Failed to send remove message from state message: {}", e);
        }
    }

    /// Handles DLQ operation errors
    fn handle_dlq_error(
        tx_to_main: &Sender<crate::components::common::Msg>,
        tx_to_main_err: &Sender<crate::components::common::Msg>,
        error: crate::error::AppError,
    ) {
        log::error!("Error in DLQ operation: {}", error);

        // Stop loading indicator
        if let Err(err) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Stop,
        )) {
            log::error!("Failed to send loading stop message: {}", err);
        }

        // Send error message
        let _ = tx_to_main_err.send(crate::components::common::Msg::Error(error));
    }

    fn handle_resend_message_from_dlq(&mut self, index: usize) -> Option<Msg> {
        // ⚠️ WARNING: DLQ message resending is in development and not recommended for production use

        // Validate the request
        let message = match self.validate_resend_request(index) {
            Ok(msg) => msg,
            Err(error_msg) => return Some(error_msg),
        };

        // Get required resources
        let consumer = match self.get_consumer_for_dlq() {
            Ok(consumer) => consumer,
            Err(error_msg) => return Some(error_msg),
        };

        // Start the resend operation
        self.start_resend_operation(message, consumer);
        None
    }

    /// Validates that the resend request is valid and returns the target message
    fn validate_resend_request(&self, index: usize) -> Result<server::model::MessageModel, Msg> {
        // Get the message at the specified index
        let message = if let Some(messages) = &self.queue_state.messages {
            if let Some(msg) = messages.get(index) {
                msg.clone()
            } else {
                log::error!("Message index {} out of bounds", index);
                return Err(Msg::Error(crate::error::AppError::State(
                    "Message index out of bounds".to_string(),
                )));
            }
        } else {
            log::error!("No messages available");
            return Err(Msg::Error(crate::error::AppError::State(
                "No messages available".to_string(),
            )));
        };

        // Only allow resending from DLQ (not from main queue)
        if self.queue_state.current_queue_type != crate::components::common::QueueType::DeadLetter {
            log::warn!("Cannot resend message from main queue - only from dead letter queue");
            return Err(Msg::Error(crate::error::AppError::State(
                "Cannot resend message from main queue - only from dead letter queue".to_string(),
            )));
        }

        Ok(message)
    }

    /// Starts the resend operation in a background task
    fn start_resend_operation(
        &self,
        message: server::model::MessageModel,
        consumer: Arc<Mutex<Consumer>>,
    ) -> Option<Msg> {
        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        // Show loading indicator
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Start(
                "Resending message from dead letter queue...".to_string(),
            ),
        )) {
            log::error!("Failed to send loading start message: {}", e);
        }

        let tx_to_main_err = tx_to_main.clone();
        let message_id = message.id.clone();
        let message_sequence = message.sequence;

        // Get the main queue name and service bus client for resending
        let main_queue_name = match self.get_main_queue_name_from_current_dlq() {
            Ok(name) => name,
            Err(e) => {
                log::error!("Failed to get main queue name: {}", e);
                return Some(Msg::Error(e));
            }
        };
        let service_bus_client = self.service_bus_client.clone();

        log::info!(
            "Starting resend operation for message {} (sequence {}) from DLQ to queue {}",
            message_id,
            message_sequence,
            main_queue_name
        );

        let task = async move {
            log::debug!("Executing resend operation in background task");

            // Add overall timeout to the entire resend operation
            let dlq_config = crate::config::CONFIG.dlq();
            let overall_timeout_secs = (dlq_config.receive_timeout_secs()
                + dlq_config.send_timeout_secs())
            .min(dlq_config.overall_timeout_cap_secs());
            log::debug!(
                "Using overall timeout of {} seconds for resend operation",
                overall_timeout_secs
            );

            let result = tokio::time::timeout(
                std::time::Duration::from_secs(overall_timeout_secs),
                Self::execute_resend_operation(
                    consumer,
                    message_id.clone(),
                    message_sequence,
                    main_queue_name,
                    service_bus_client,
                ),
            )
            .await;

            match result {
                Ok(Ok(())) => {
                    log::info!(
                        "Resend operation completed successfully for message {}",
                        message_id
                    );
                    Self::handle_resend_success(&tx_to_main, &message_id, message_sequence);
                }
                Ok(Err(e)) => {
                    log::error!("Failed to resend message {}: {}", message_id, e);
                    Self::handle_resend_error(&tx_to_main, &tx_to_main_err, e);
                }
                Err(_) => {
                    log::error!(
                        "Overall timeout for resend operation after {} seconds",
                        overall_timeout_secs
                    );
                    let timeout_error = crate::error::AppError::ServiceBus(format!(
                        "Resend operation timed out after {} seconds",
                        overall_timeout_secs
                    ));
                    Self::handle_resend_error(&tx_to_main, &tx_to_main_err, timeout_error);
                }
            }
        };

        taskpool.execute(task);

        None
    }

    /// Executes the resend operation: receive message from DLQ, send to main queue, complete DLQ message
    async fn execute_resend_operation(
        consumer: Arc<Mutex<Consumer>>,
        message_id: String,
        message_sequence: i64,
        main_queue_name: String,
        service_bus_client: Arc<
            Mutex<azservicebus::ServiceBusClient<azservicebus::core::BasicRetryPolicy>>,
        >,
    ) -> Result<(), crate::error::AppError> {
        log::debug!("Acquiring consumer lock for resend operation");
        let mut consumer = consumer.lock().await;

        // Find the target message in DLQ
        log::debug!("Searching for target message in DLQ");
        let target_msg =
            Self::find_target_message(&mut consumer, &message_id, message_sequence).await?;

        // Get the message body and properties for resending
        log::debug!("Extracting message body for resending");
        let message_body = target_msg.body().map_err(|e| {
            log::error!("Failed to get message body: {}", e);
            crate::error::AppError::ServiceBus(e.to_string())
        })?;

        // Create a new message with the same content using Producer helper
        log::debug!("Creating new message with {} bytes", message_body.len());
        let new_message = Producer::create_message(message_body.to_vec());

        // Send the message to the main queue
        log::info!(
            "Resending message {} from DLQ to main queue {}",
            message_id,
            main_queue_name
        );
        Self::send_message_to_main_queue(&main_queue_name, new_message, service_bus_client).await?;

        // Complete the original DLQ message to remove it from DLQ
        log::info!(
            "Completing DLQ message {} to remove it from dead letter queue",
            message_id
        );
        consumer.complete_message(&target_msg).await.map_err(|e| {
            log::error!("Failed to complete DLQ message: {}", e);
            crate::error::AppError::ServiceBus(e.to_string())
        })?;

        log::info!(
            "Successfully resent message {} from dead letter queue to main queue",
            message_id
        );

        Ok(())
    }

    /// Get the main queue name from the current DLQ queue name
    fn get_main_queue_name_from_current_dlq(&self) -> Result<String, crate::error::AppError> {
        if let Some(current_queue_name) = &self.queue_state.current_queue_name {
            if self.queue_state.current_queue_type
                == crate::components::common::QueueType::DeadLetter
            {
                // Remove the /$deadletterqueue suffix to get the main queue name
                if let Some(main_name) = current_queue_name.strip_suffix("/$deadletterqueue") {
                    Ok(main_name.to_string())
                } else {
                    log::error!(
                        "Current queue name '{}' doesn't have expected DLQ suffix '/$deadletterqueue'",
                        current_queue_name
                    );
                    Err(crate::error::AppError::State(
                        "Current queue name doesn't have expected DLQ suffix".to_string(),
                    ))
                }
            } else {
                log::error!(
                    "Cannot resend from main queue - current queue type is {:?}",
                    self.queue_state.current_queue_type
                );
                Err(crate::error::AppError::State(
                    "Cannot resend from main queue - only from dead letter queue".to_string(),
                ))
            }
        } else {
            log::error!("No current queue name available");
            Err(crate::error::AppError::State(
                "No current queue name available".to_string(),
            ))
        }
    }

    /// Send a message to the main queue using Producer
    async fn send_message_to_main_queue(
        queue_name: &str,
        message: azservicebus::ServiceBusMessage,
        service_bus_client: Arc<
            Mutex<azservicebus::ServiceBusClient<azservicebus::core::BasicRetryPolicy>>,
        >,
    ) -> Result<(), crate::error::AppError> {
        // Use configurable timeout with cap to avoid hanging - Azure Service Bus might have internal timeouts
        let dlq_config = crate::config::CONFIG.dlq();
        let send_timeout_secs = dlq_config
            .send_timeout_secs()
            .min(dlq_config.send_timeout_cap_secs());

        log::debug!(
            "Creating producer for queue: {} (timeout: {}s)",
            queue_name,
            send_timeout_secs
        );

        // Add timeout to the entire send operation
        let send_result =
            tokio::time::timeout(std::time::Duration::from_secs(send_timeout_secs), async {
                log::debug!("Acquiring service bus client lock for sending");
                // Acquire the service bus client lock
                let mut client = service_bus_client.lock().await;

                log::debug!("Creating producer for queue: {}", queue_name);
                // Create a producer for the main queue
                let mut producer = client
                    .create_producer_for_queue(
                        queue_name,
                        azservicebus::ServiceBusSenderOptions::default(),
                    )
                    .await
                    .map_err(|e| {
                        log::error!("Failed to create producer for queue {}: {}", queue_name, e);
                        crate::error::AppError::ServiceBus(e.to_string())
                    })?;

                log::debug!("Sending message to queue: {}", queue_name);

                // Send the message using the producer
                producer.send_message(message).await.map_err(|e| {
                    log::error!("Failed to send message to queue {}: {}", queue_name, e);
                    crate::error::AppError::ServiceBus(e.to_string())
                })?;

                log::debug!("Message sent successfully, disposing producer");

                // Dispose the producer
                producer.dispose().await.map_err(|e| {
                    log::warn!("Failed to dispose producer for queue {}: {}", queue_name, e);
                    crate::error::AppError::ServiceBus(e.to_string())
                })?;

                log::debug!("Producer disposed successfully");
                Ok::<(), crate::error::AppError>(())
            })
            .await;

        match send_result {
            Ok(Ok(())) => {
                log::info!("Successfully sent message to queue: {}", queue_name);
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => {
                log::error!(
                    "Timeout while sending message to queue {} after {} seconds",
                    queue_name,
                    send_timeout_secs
                );
                Err(crate::error::AppError::ServiceBus(format!(
                    "Timeout while sending message to queue {} after {} seconds",
                    queue_name, send_timeout_secs
                )))
            }
        }
    }

    /// Handles successful resend operation
    fn handle_resend_success(
        tx_to_main: &Sender<crate::components::common::Msg>,
        message_id: &str,
        message_sequence: i64,
    ) {
        log::info!(
            "Resend operation completed successfully for message {} (sequence {})",
            message_id,
            message_sequence
        );

        // Stop loading indicator
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Stop,
        )) {
            log::error!("Failed to send loading stop message: {}", e);
        }

        // Remove the message from local state since it's been resent
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::MessageActivity(
            crate::components::common::MessageActivityMsg::RemoveMessageFromState(
                message_id.to_string(),
                message_sequence,
            ),
        )) {
            log::error!("Failed to send remove message from state message: {}", e);
        }
    }

    /// Handles resend operation errors
    fn handle_resend_error(
        tx_to_main: &Sender<crate::components::common::Msg>,
        tx_to_main_err: &Sender<crate::components::common::Msg>,
        error: crate::error::AppError,
    ) {
        log::error!("Error in resend operation: {}", error);

        // Stop loading indicator
        if let Err(err) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Stop,
        )) {
            log::error!("Failed to send loading stop message: {}", err);
        }

        // Send error message
        let _ = tx_to_main_err.send(crate::components::common::Msg::Error(error));
    }

    fn handle_remove_message_from_state(
        &mut self,
        message_id: String,
        message_sequence: i64,
    ) -> Option<Msg> {
        let page_size = crate::config::CONFIG.max_messages() as usize;

        // Remove the message from pagination state by both ID and sequence
        let removed = self
            .queue_state
            .message_pagination
            .remove_message_by_id_and_sequence(&message_id, message_sequence, page_size);

        if !removed {
            log::warn!(
                "Message with ID {} and sequence {} not found in local state",
                message_id,
                message_sequence
            );
            return None;
        }

        log::info!(
            "Removed message {} (sequence {}) from local state",
            message_id,
            message_sequence
        );

        // Update the current page view with the new state
        if let Err(e) = self.update_current_page_view() {
            return Some(Msg::Error(e));
        }

        // Update message details if we have messages
        let current_page_messages = self
            .queue_state
            .message_pagination
            .get_current_page_messages(page_size);
        if !current_page_messages.is_empty() {
            if let Err(e) = self.remount_message_details(0) {
                return Some(Msg::Error(e));
            }
        }

        None
    }

    fn handle_queue_name_updated(&mut self, queue_name: String) -> Option<Msg> {
        self.queue_state.current_queue_name = Some(queue_name);
        None
    }
}
