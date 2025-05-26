use crate::app::model::{AppState, Model};
use crate::components::common::{MessageActivityMsg, Msg};
use server::consumer::Consumer;
use server::model::MessageModel;
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
            MessageActivityMsg::ReloadMessages => self.handle_reload_messages(),
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
        log::debug!("Acquiring consumer lock");
        let mut consumer = consumer.lock().await;
        log::debug!("Peeking messages with sequence: {:?}", from_sequence);

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
            log::debug!("No new messages available");
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
                    Self::handle_dlq_success(&tx_to_main, &message_id);
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
        log::debug!("Acquiring consumer lock for DLQ operation");
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
        const MAX_ATTEMPTS: usize = 10;
        let mut target_message = None;
        let mut other_messages = Vec::new();

        while attempts < MAX_ATTEMPTS && target_message.is_none() {
            let received_messages = consumer.receive_messages(5).await.map_err(|e| {
                log::error!("Failed to receive messages: {}", e);
                crate::error::AppError::ServiceBus(e.to_string())
            })?;

            if received_messages.is_empty() {
                log::warn!("No more messages available to receive");
                break;
            }

            for msg in received_messages {
                if let Some(msg_id) = msg.message_id() {
                    if msg_id == message_id {
                        log::info!("Found target message with ID {}", message_id);
                        target_message = Some(msg);
                        break;
                    }
                }
                other_messages.push(msg);
            }

            attempts += 1;
        }

        // Abandon all the other messages we received but don't want to dead letter
        Self::abandon_other_messages(consumer, other_messages).await;

        // Return the target message or error
        target_message.ok_or_else(|| {
            log::error!(
                "Could not find message with ID {} after {} attempts",
                message_id,
                attempts
            );
            crate::error::AppError::ServiceBus(format!(
                "Could not find message with ID {} in received messages",
                message_id
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
    fn handle_dlq_success(tx_to_main: &Sender<crate::components::common::Msg>, message_id: &str) {
        log::info!(
            "DLQ operation completed successfully for message {}",
            message_id
        );

        // Stop loading indicator
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Stop,
        )) {
            log::error!("Failed to send loading stop message: {}", e);
        }

        // Trigger a message reload to refresh the message list
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::MessageActivity(
            crate::components::common::MessageActivityMsg::ReloadMessages,
        )) {
            log::error!("Failed to send message reload message: {}", e);
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

    fn handle_reload_messages(&mut self) -> Option<Msg> {
        // Reset pagination and reload messages from the current queue
        self.reset_pagination_state();
        if let Err(e) = self.load_messages() {
            return Some(Msg::Error(e));
        }
        None
    }
}
