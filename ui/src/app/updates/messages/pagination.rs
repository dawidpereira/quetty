use crate::app::model::{AppState, Model};
use crate::components::common::{ComponentId, MessageActivityMsg, Msg};
use crate::config::CONFIG;
use crate::error::{AppError, AppResult};
use server::model::MessageModel;
use std::sync::Arc;
use std::sync::mpsc::Sender;
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
    pub fn calculate_page_bounds(&self, page_size: u32) -> (usize, usize) {
        let page_size = page_size as usize;
        let start_idx = self.current_page * page_size;
        let end_idx = std::cmp::min(start_idx + page_size, self.all_loaded_messages.len());
        (start_idx, end_idx)
    }

    /// Reset pagination state to initial values
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Update pagination state based on current page and total loaded pages
    pub fn update(&mut self, page_size: u32) {
        self.has_previous_page = self.current_page > 0;
        self.has_next_page = self.calculate_has_next_page(page_size);
    }

    /// Calculate if there's a next page available
    fn calculate_has_next_page(&self, page_size: u32) -> bool {
        let page_size = page_size as usize;
        let next_page_exists = self.current_page + 1 < self.total_pages_loaded;

        // Check if we might have more messages to load from API
        let might_have_more_to_load = self.total_pages_loaded > 0
            && (
                // Normal case: we have a full page, so might be more
                self.all_loaded_messages.len() % page_size == 0 ||
            // After bulk removal: we have fewer messages than expected for loaded pages
            self.all_loaded_messages.len() < self.total_pages_loaded * page_size
            );

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
    pub fn get_current_page_messages(&self, page_size: u32) -> Vec<MessageModel> {
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
    // Pagination request handlers
    pub fn handle_next_page_request(&mut self) -> Option<Msg> {
        if self.queue_state.message_pagination.has_next_page {
            if let Err(e) = self.handle_next_page() {
                return Some(Msg::Error(e));
            }
        }
        None
    }

    pub fn handle_previous_page_request(&mut self) -> Option<Msg> {
        if self.queue_state.message_pagination.has_previous_page {
            if let Err(e) = self.handle_previous_page() {
                return Some(Msg::Error(e));
            }
        }
        None
    }

    // Pagination navigation methods
    pub fn handle_next_page(&mut self) -> AppResult<()> {
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
            self.load_messages_from_api_with_count(CONFIG.max_messages())?;
        }

        Ok(())
    }

    pub fn handle_previous_page(&mut self) -> AppResult<()> {
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

    pub fn handle_page_changed(&mut self) -> Option<Msg> {
        if let Err(e) = self.update_current_page_view() {
            return Some(Msg::Error(e));
        }
        None
    }

    pub fn handle_pagination_state_updated(
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

    // Pagination utility methods
    pub fn reset_pagination_state(&mut self) {
        self.queue_state.message_pagination.reset();
    }

    fn switch_to_loaded_page(&mut self, page: usize) {
        log::debug!("Page {} already loaded, switching view", page);
        self.queue_state.message_pagination.set_current_page(page);

        // Check if this page needs backfilling after switching to it
        let backfill_happened = match self.check_and_backfill_current_page() {
            Ok(happened) => happened,
            Err(e) => {
                log::error!("Failed to check/backfill current page: {}", e);
                false // Continue with normal flow even if backfilling fails
            }
        };

        // Only update view if backfilling didn't happen
        // (backfilling will trigger its own view update when loading completes)
        if !backfill_happened {
            self.update_pagination_state();
            self.send_page_changed_message();
        }
    }

    fn send_page_changed_message(&self) {
        if let Err(e) = self
            .tx_to_main
            .send(Msg::MessageActivity(MessageActivityMsg::PageChanged))
        {
            log::error!("Failed to send page changed message: {}", e);
        }
    }

    fn update_pagination_state(&mut self) {
        let page_size = CONFIG.max_messages();
        self.queue_state.message_pagination.update(page_size);

        log::debug!(
            "Updated pagination state: current={}, total_loaded={}, has_prev={}, has_next={}",
            self.queue_state.message_pagination.current_page,
            self.queue_state.message_pagination.total_pages_loaded,
            self.queue_state.message_pagination.has_previous_page,
            self.queue_state.message_pagination.has_next_page
        );
    }

    pub fn update_current_page_view(&mut self) -> AppResult<()> {
        let page_size = CONFIG.max_messages();
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

        // Check if this is an initial load (when we should focus messages list)
        let is_initial_load = self.queue_state.message_pagination.current_page == 0
            && self.queue_state.message_pagination.total_pages_loaded == 1;

        if is_initial_load {
            // Initial load: remount with focus and set proper app state
            self.remount_messages_with_focus(true)?;
            self.app_state = AppState::MessagePicker;

            // Set focus to messages component
            if let Err(e) = self.app.active(&ComponentId::Messages) {
                log::error!("Failed to activate messages: {}", e);
            }
        } else {
            // Normal page change: check if messages should remain focused
            let should_stay_focused = matches!(self.app_state, AppState::MessagePicker);

            if should_stay_focused {
                // Keep focus when navigating pages (teal border)
                self.remount_messages_with_focus(true)?;
            } else {
                // Reset cursor to position 0 when changing pages (no focus)
                self.remount_messages_with_cursor_control(false)?;
            }
        }

        Ok(())
    }

    fn send_pagination_state_update(&self) -> AppResult<()> {
        self.tx_to_main
            .send(Msg::MessageActivity(
                MessageActivityMsg::PaginationStateUpdated {
                    has_next: self.queue_state.message_pagination.has_next_page,
                    has_previous: self.queue_state.message_pagination.has_previous_page,
                    current_page: self.queue_state.message_pagination.current_page,
                    total_pages_loaded: self.queue_state.message_pagination.total_pages_loaded,
                },
            ))
            .map_err(|e| {
                log::error!("Failed to send pagination state update: {}", e);
                AppError::Component(e.to_string())
            })
    }

    /// Check if current page is under-filled and backfill it if needed
    /// Returns Ok(true) if backfilling happened, Ok(false) if no backfilling was needed
    fn check_and_backfill_current_page(&mut self) -> AppResult<bool> {
        let page_size = CONFIG.max_messages();
        let current_page = self.queue_state.message_pagination.current_page;
        let current_page_messages = self
            .queue_state
            .message_pagination
            .get_current_page_messages(page_size);
        let current_page_size = current_page_messages.len();
        let page_is_under_filled = current_page_size < page_size as usize;

        // Check if this page should be auto-filled
        // We auto-fill if the page is under-filled AND we think there might be more messages available
        let should_auto_fill =
            page_is_under_filled && self.queue_state.message_pagination.has_next_page;

        if should_auto_fill {
            let messages_needed = page_size as usize - current_page_size;
            log::info!(
                "Page {} is under-filled with {} messages (expected {}), auto-loading {} more",
                current_page + 1,
                current_page_size,
                page_size,
                messages_needed
            );

            // Auto-load the missing messages using specialized backfill method
            self.load_messages_for_backfill(messages_needed as u32)?;
            return Ok(true); // Backfilling happened
        } else if page_is_under_filled {
            log::debug!(
                "Page {} has {} messages but no more messages available from API - no auto-loading needed",
                current_page + 1,
                current_page_size
            );
        } else {
            log::debug!(
                "Page {} is properly filled with {} messages",
                current_page + 1,
                current_page_size
            );
        }

        Ok(false) // No backfilling happened
    }

    /// Load messages specifically for backfilling current page (doesn't change pagination state)
    pub fn load_messages_for_backfill(&mut self, message_count: u32) -> AppResult<()> {
        log::debug!(
            "Loading {} messages for backfill, last_sequence: {:?}",
            message_count,
            self.queue_state.message_pagination.last_loaded_sequence
        );

        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        // Send loading start message
        if let Err(e) = tx_to_main.send(Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Start(
                "Loading more messages...".to_string(),
            ),
        )) {
            log::error!("Failed to send loading start message: {}", e);
        }

        let consumer = self.queue_state.consumer.clone().ok_or_else(|| {
            log::error!("No consumer available");
            AppError::State("No consumer available".to_string())
        })?;

        let tx_to_main_err = tx_to_main.clone();
        let from_sequence = self
            .queue_state
            .message_pagination
            .last_loaded_sequence
            .map(|seq| seq + 1);

        taskpool.execute(async move {
            Self::execute_backfill_loading_task(
                tx_to_main,
                tx_to_main_err,
                consumer,
                from_sequence,
                message_count,
            )
            .await;
        });

        Ok(())
    }

    async fn execute_backfill_loading_task(
        tx_to_main: Sender<Msg>,
        tx_to_main_err: Sender<Msg>,
        consumer: Arc<tokio::sync::Mutex<server::consumer::Consumer>>,
        from_sequence: Option<i64>,
        message_count: u32,
    ) {
        let result = Self::load_messages_for_backfill_from_consumer(
            tx_to_main.clone(),
            consumer,
            from_sequence,
            message_count,
        )
        .await;

        if let Err(e) = result {
            log::error!("Error in backfill loading task: {}", e);

            // Send loading stop message
            if let Err(e) = tx_to_main.send(Msg::LoadingActivity(
                crate::components::common::LoadingActivityMsg::Stop,
            )) {
                log::error!("Failed to send loading stop message: {}", e);
            }

            let _ = tx_to_main_err.send(Msg::Error(e));
        }
    }

    async fn load_messages_for_backfill_from_consumer(
        tx_to_main: Sender<Msg>,
        consumer: Arc<tokio::sync::Mutex<server::consumer::Consumer>>,
        from_sequence: Option<i64>,
        message_count: u32,
    ) -> Result<(), AppError> {
        let mut consumer = consumer.lock().await;

        let messages = consumer
            .peek_messages(message_count, from_sequence)
            .await
            .map_err(|e| {
                log::error!("Failed to peek messages for backfill: {}", e);
                AppError::ServiceBus(e.to_string())
            })?;

        log::info!(
            "Loaded {} messages for backfill (requested {})",
            messages.len(),
            message_count
        );

        // Send loading stop message
        if let Err(e) = tx_to_main.send(Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Stop,
        )) {
            log::error!("Failed to send loading stop message: {}", e);
        }

        // Send backfill messages (different from NewMessagesLoaded)
        if !messages.is_empty() {
            if let Err(e) = tx_to_main.send(Msg::MessageActivity(
                crate::components::common::MessageActivityMsg::BackfillMessagesLoaded(messages),
            )) {
                log::error!("Failed to send backfill messages: {}", e);
                return Err(AppError::Component(e.to_string()));
            }
        } else {
            // No messages loaded, trigger page changed to update view
            if let Err(e) = tx_to_main.send(Msg::MessageActivity(
                crate::components::common::MessageActivityMsg::PageChanged,
            )) {
                log::error!("Failed to send page changed message: {}", e);
                return Err(AppError::Component(e.to_string()));
            }
        }

        Ok(())
    }
}
