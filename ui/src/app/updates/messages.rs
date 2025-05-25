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
        self.messages = Some(messages);
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
        self.consumer = Some(Arc::new(Mutex::new(consumer)));
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
        let is_initial_load = self.message_pagination.all_loaded_messages.is_empty();

        // Add new messages to pagination state
        self.message_pagination.add_loaded_page(new_messages);

        // If this is not the initial load, advance to the new page
        if !is_initial_load {
            self.message_pagination.advance_to_next_page();
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
        if !self.message_pagination.all_loaded_messages.is_empty() {
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
        self.message_pagination.has_next_page = has_next;
        self.message_pagination.has_previous_page = has_previous;
        self.message_pagination.current_page = current_page;
        self.message_pagination.total_pages_loaded = total_pages_loaded;
        None
    }

    // Pagination request handlers
    fn handle_next_page_request(&mut self) -> Option<Msg> {
        if self.message_pagination.has_next_page {
            if let Err(e) = self.handle_next_page() {
                return Some(Msg::Error(e));
            }
        }
        None
    }

    fn handle_previous_page_request(&mut self) -> Option<Msg> {
        if self.message_pagination.has_previous_page {
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
            self.message_pagination.current_page,
            self.message_pagination.total_pages_loaded
        );

        let next_page = self.message_pagination.current_page + 1;

        if self.message_pagination.is_page_loaded(next_page) {
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
            self.message_pagination.current_page
        );

        if self.message_pagination.current_page > 0 {
            self.message_pagination.go_to_previous_page();
            self.update_pagination_state();
            self.send_page_changed_message();
        }

        Ok(())
    }

    // Pagination utility methods
    fn reset_pagination_state(&mut self) {
        self.message_pagination.reset();
    }

    fn switch_to_loaded_page(&mut self, page: usize) {
        log::debug!("Page {} already loaded, switching view", page);
        self.message_pagination.set_current_page(page);
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
        self.message_pagination.update(page_size);

        log::debug!(
            "Updated pagination state: current={}, total_loaded={}, has_prev={}, has_next={}",
            self.message_pagination.current_page,
            self.message_pagination.total_pages_loaded,
            self.message_pagination.has_previous_page,
            self.message_pagination.has_next_page
        );
    }

    fn update_current_page_view(&mut self) -> crate::error::AppResult<()> {
        let page_size = crate::config::CONFIG.max_messages() as usize;
        let current_page_messages = self.message_pagination.get_current_page_messages(page_size);
        let (start_idx, end_idx) = self.message_pagination.calculate_page_bounds(page_size);

        log::debug!(
            "Updating view for page {}: showing messages {}-{} of {}",
            self.message_pagination.current_page,
            start_idx,
            end_idx,
            self.message_pagination.all_loaded_messages.len()
        );

        self.messages = Some(current_page_messages);
        self.update_pagination_state();
        self.send_pagination_state_update()?;
        self.remount_messages()?;

        Ok(())
    }

    fn send_pagination_state_update(&self) -> crate::error::AppResult<()> {
        self.tx_to_main
            .send(crate::components::common::Msg::MessageActivity(
                crate::components::common::MessageActivityMsg::PaginationStateUpdated {
                    has_next: self.message_pagination.has_next_page,
                    has_previous: self.message_pagination.has_previous_page,
                    current_page: self.message_pagination.current_page,
                    total_pages_loaded: self.message_pagination.total_pages_loaded,
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
            self.message_pagination.last_loaded_sequence
        );

        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        self.send_loading_start_message(&tx_to_main);

        let consumer = self.get_consumer()?;
        let tx_to_main_err = tx_to_main.clone();
        let from_sequence = self
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
        self.consumer.clone().ok_or_else(|| {
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
}
