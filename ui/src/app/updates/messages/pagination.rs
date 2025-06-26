use crate::app::model::{AppState, Model};
use crate::components::common::{ComponentId, MessageActivityMsg, Msg};
use crate::config;
use crate::error::{AppError, AppResult};
use server::model::MessageModel;
use server::service_bus_manager::QueueType;

use tuirealm::terminal::TerminalAdapter;

/// Cache for queue statistics to avoid redundant API calls
#[derive(Debug, Clone, PartialEq)]
pub struct QueueStatsCache {
    pub queue_name: String,
    pub queue_type: QueueType,
    pub active_message_count: Option<u64>,
    pub dead_letter_message_count: Option<u64>,
    pub fetched_at: chrono::DateTime<chrono::Utc>,
    pub ttl_seconds: u64, // Default: 60 seconds
}

impl QueueStatsCache {
    pub fn new(
        queue_name: String,
        queue_type: QueueType,
        active_count: Option<u64>,
        dead_letter_count: Option<u64>,
    ) -> Self {
        Self {
            queue_name,
            queue_type,
            active_message_count: active_count,
            dead_letter_message_count: dead_letter_count,
            fetched_at: chrono::Utc::now(),
            ttl_seconds: 60, // 1 minute default TTL
        }
    }

    pub fn is_expired(&self) -> bool {
        let age = chrono::Utc::now().signed_duration_since(self.fetched_at);
        age.num_seconds() as u64 > self.ttl_seconds
    }

    pub fn is_valid_for_queue(&self, name: &str, queue_type: &QueueType) -> bool {
        self.queue_name == name && self.queue_type == *queue_type
    }

    pub fn age_seconds(&self) -> i64 {
        chrono::Utc::now().signed_duration_since(self.fetched_at).num_seconds()
    }
}

/// Dedicated type for managing message pagination state
#[derive(Debug, Clone, Default)]
pub struct MessagePaginationState {
    pub current_page: usize,
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub total_pages_loaded: usize,
    pub last_loaded_sequence: Option<i64>,
    pub all_loaded_messages: Vec<MessageModel>,
    /// True when the last API call returned 0 messages (reached end of queue)
    pub reached_end_of_queue: bool,
    /// Cache for queue statistics
    pub stats_cache: Option<QueueStatsCache>,
    pub page_start_indices: Vec<usize>,
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
        // Initialize with page 0 starting at index 0
        self.page_start_indices = vec![0];
    }

    /// Update pagination state based on current page and total loaded pages
    pub fn update(&mut self, page_size: u32) {
        self.has_previous_page = self.current_page > 0;
        self.has_next_page = self.calculate_has_next_page(page_size);
    }

    /// Calculate if there's a next page available
    fn calculate_has_next_page(&self, page_size: u32) -> bool {
        // If we've confirmed we've reached the end of the queue, no more pages
        if self.reached_end_of_queue {
            return false;
        }

        let _page_size = page_size as usize;
        
        // Check if we have a next page already loaded
        let next_page_exists = self.current_page + 1 < self.total_pages_loaded;
        if next_page_exists {
            return true;
        }

        // If no pages loaded yet, we can try to load
        if self.total_pages_loaded == 0 {
            return true;
        }

        // If we have messages loaded, check if we should try to load more
        // Always allow loading more unless we've explicitly reached the end
        // This handles cases where pages are incomplete due to sequence gaps
        !self.all_loaded_messages.is_empty()
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

    /// Add messages to a new page
    pub fn add_loaded_page(&mut self, new_messages: Vec<MessageModel>) {
        // Only add start index if this is not the first page (which starts at 0)
        if self.total_pages_loaded > 0 {
            self.page_start_indices.push(self.all_loaded_messages.len());
        }

        self.all_loaded_messages.extend(new_messages.clone());
        self.update_last_loaded_sequence(&new_messages);
        self.total_pages_loaded += 1;
    }

    /// Extend the current page with additional messages (used for automatic page filling)
    pub fn extend_current_page(&mut self, additional_messages: Vec<MessageModel>) {
        if additional_messages.is_empty() {
            // Empty result means we've reached the end of the queue
            self.reached_end_of_queue = true;
            return;
        }

        // Add messages to the existing page without incrementing page count
        self.all_loaded_messages.extend(additional_messages.clone());
        self.update_last_loaded_sequence(&additional_messages);
        
        log::debug!(
            "Extended current page with {} messages, total now: {}",
            additional_messages.len(),
            self.all_loaded_messages.len()
        );
        
        // Note: Do NOT increment total_pages_loaded since we're extending existing page
        // Note: Do NOT add to page_start_indices since this isn't a new page
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

    /// Check if stats cache is expired for current queue
    pub fn is_stats_cache_expired(&self, queue_name: &str, queue_type: &QueueType) -> bool {
        match &self.stats_cache {
            Some(cache) => {
                !cache.is_valid_for_queue(queue_name, queue_type) || cache.is_expired()
            }
            None => true, // No cache means expired
        }
    }

    /// Update stats cache
    pub fn update_stats_cache(&mut self, cache: QueueStatsCache) {
        log::debug!(
            "Updated stats cache for queue {} ({:?}): active={:?}, age={}s",
            cache.queue_name,
            cache.queue_type,
            cache.active_message_count,
            cache.age_seconds()
        );
        self.stats_cache = Some(cache);
    }

    /// Get cached stats if valid
    pub fn get_cached_stats(&self) -> Option<&QueueStatsCache> {
        self.stats_cache.as_ref().filter(|cache| !cache.is_expired())
    }

    /// Invalidate stats cache (after bulk operations)
    pub fn invalidate_stats_cache(&mut self) {
        if self.stats_cache.is_some() {
            log::debug!("Invalidated stats cache");
            self.stats_cache = None;
        }
    }

    /// Format pagination status with stats
    pub fn format_pagination_with_stats(&self) -> String {
        let base = format!("Page {}", self.current_page + 1);
        
        match self.get_cached_stats() {
            Some(cache) => {
                if let Some(total) = cache.active_message_count {
                    let age = cache.age_seconds();
                    if age < 60 {
                        format!("{} ({} total msgs)", base, total)
                    } else {
                        format!("{} (~{} total msgs, {}m ago)", base, total, age / 60)
                    }
                } else {
                    base
                }
            }
            None => base,
        }
    }

    fn update_last_loaded_sequence(&mut self, messages: &[MessageModel]) {
        if let Some(last_msg) = messages.last() {
            self.last_loaded_sequence = Some(last_msg.sequence);
        }
    }
}

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    // Pagination request handlers
    pub fn handle_next_page_request(&mut self) -> Option<Msg> {
        if self
            .queue_manager
            .queue_state
            .message_pagination
            .has_next_page
        {
            if let Err(e) = self.handle_next_page() {
                self.error_reporter
                    .report_simple(e, "Pagination", "next_page");
                return None;
            }
        }
        None
    }

    pub fn handle_previous_page_request(&mut self) -> Option<Msg> {
        if self
            .queue_manager
            .queue_state
            .message_pagination
            .has_previous_page
        {
            if let Err(e) = self.handle_previous_page() {
                self.error_reporter
                    .report_simple(e, "Pagination", "prev_page");
                return None;
            }
        }
        None
    }

    // Pagination navigation methods
    pub fn handle_next_page(&mut self) -> AppResult<()> {
        log::debug!(
            "Handle next page - current: {}, total_loaded: {}",
            self.queue_manager
                .queue_state
                .message_pagination
                .current_page,
            self.queue_manager
                .queue_state
                .message_pagination
                .total_pages_loaded
        );

        let next_page = self
            .queue_manager
            .queue_state
            .message_pagination
            .current_page
            + 1;

        if self
            .queue_state()
            .message_pagination
            .is_page_loaded(next_page)
        {
            self.switch_to_loaded_page(next_page);
        } else {
            log::debug!("Loading new page {} from API", next_page);
            
            // Check if we need to refresh stats cache
            let current_queue_name = self.queue_state().current_queue_name.clone().unwrap_or_default();
            let current_queue_type = self.queue_state().current_queue_type.clone();
            let cache_expired = self.queue_state().message_pagination
                .is_stats_cache_expired(&current_queue_name, &current_queue_type);
            
            if cache_expired {
                // Load messages with stats refresh (sequential to avoid stack overflow)
                self.load_messages_and_refresh_stats_sequential()?;
            } else {
                // Cache is fresh: Only load messages
                self.load_messages_from_api_with_count(config::get_config_or_panic().max_messages())?;
            }
        }

        Ok(())
    }

    /// Load messages and refresh stats sequentially when cache is expired
    fn load_messages_and_refresh_stats_sequential(&mut self) -> AppResult<()> {
        log::debug!("Loading messages and refreshing stats sequentially");
        
        // First, load messages normally
        self.load_messages_from_api_with_count(config::get_config_or_panic().max_messages())?;
        
        // Then, refresh stats in background (non-blocking)
        let service_bus_manager = self.get_service_bus_manager();
        let tx_to_main = self.state_manager.tx_to_main.clone();
        let current_queue_name = self.queue_state().current_queue_name.clone().unwrap_or_default();
        let current_queue_type = self.queue_state().current_queue_type.clone();

        self.task_manager.execute("Refreshing queue stats...", async move {
            use server::service_bus_manager::{ServiceBusCommand, ServiceBusResponse};

            log::debug!("Refreshing queue statistics in background");
            let stats_command = ServiceBusCommand::GetQueueStatistics {
                queue_name: current_queue_name.clone(),
                queue_type: current_queue_type.clone(),
            };
            
            match service_bus_manager.lock().await.execute_command(stats_command).await {
                ServiceBusResponse::QueueStatistics {
                    queue_name,
                    queue_type,
                    active_message_count,
                    dead_letter_message_count,
                    retrieved_at: _,
                } => {
                    log::debug!("Successfully refreshed queue stats: active={:?}", active_message_count);
                    let stats_cache = crate::app::updates::messages::pagination::QueueStatsCache::new(
                        queue_name,
                        queue_type,
                        active_message_count,
                        dead_letter_message_count,
                    );
                    
                    if let Err(e) = tx_to_main.send(Msg::MessageActivity(MessageActivityMsg::QueueStatsUpdated(stats_cache))) {
                        log::warn!("Failed to send queue stats update: {}", e);
                    }
                }
                ServiceBusResponse::Error { error } => {
                    log::warn!("Failed to refresh queue stats: {}", error);
                }
                _ => {
                    log::warn!("Unexpected response for queue statistics");
                }
            }

            log::debug!("Stats refresh completed");
            Ok(())
        });

        Ok(())
    }

    pub fn handle_previous_page(&mut self) -> AppResult<()> {
        log::debug!(
            "Handle previous page - current: {}",
            self.queue_manager
                .queue_state
                .message_pagination
                .current_page
        );

        if self
            .queue_manager
            .queue_state
            .message_pagination
            .current_page
            > 0
        {
            self.queue_state_mut()
                .message_pagination
                .go_to_previous_page();
            self.update_pagination_state();
            self.send_page_changed_message();
        }

        Ok(())
    }

    pub fn handle_page_changed(&mut self) -> Option<Msg> {
        if let Err(e) = self.update_current_page_view() {
            self.error_reporter
                .report_simple(e, "Pagination", "page_changed");
            return None;
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
        self.queue_state_mut().message_pagination.has_next_page = has_next;
        self.queue_state_mut().message_pagination.has_previous_page = has_previous;
        self.queue_state_mut().message_pagination.current_page = current_page;
        self.queue_state_mut().message_pagination.total_pages_loaded = total_pages_loaded;
        None
    }

    // Pagination utility methods
    pub fn reset_pagination_state(&mut self) {
        self.queue_state_mut().message_pagination.reset();

        // Also clear bulk selection state during pagination reset (e.g., during force reload)
        // This prevents old message IDs from persisting after bulk operations
        log::debug!("Clearing bulk selection state during pagination reset");
        self.queue_state_mut().bulk_selection.clear_all();
    }

    fn switch_to_loaded_page(&mut self, page: usize) {
        log::debug!("Page {} already loaded, switching view", page);
        self.queue_state_mut()
            .message_pagination
            .set_current_page(page);

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
            .tx_to_main()
            .send(Msg::MessageActivity(MessageActivityMsg::PageChanged))
        {
            log::error!("Failed to send page changed message: {}", e);
        }
    }

    fn update_pagination_state(&mut self) {
        let page_size = config::get_config_or_panic().max_messages();
        self.queue_state_mut().message_pagination.update(page_size);

        log::debug!(
            "Updated pagination state: current={}, total_loaded={}, has_prev={}, has_next={}",
            self.queue_manager
                .queue_state
                .message_pagination
                .current_page,
            self.queue_manager
                .queue_state
                .message_pagination
                .total_pages_loaded,
            self.queue_manager
                .queue_state
                .message_pagination
                .has_previous_page,
            self.queue_manager
                .queue_state
                .message_pagination
                .has_next_page
        );
    }

    pub fn update_current_page_view(&mut self) -> AppResult<()> {
        let page_size = config::get_config_or_panic().max_messages();
        let current_page_messages = self
            .queue_state()
            .message_pagination
            .get_current_page_messages(page_size);
        let (start_idx, end_idx) = self
            .queue_state()
            .message_pagination
            .calculate_page_bounds(page_size);

        log::debug!(
            "Updating view for page {}: showing messages {}-{} of {}",
            self.queue_manager
                .queue_state
                .message_pagination
                .current_page,
            start_idx,
            end_idx,
            &self
                .queue_manager
                .queue_state
                .message_pagination
                .all_loaded_messages
                .len()
        );

        self.queue_state_mut().messages = Some(current_page_messages);
        self.update_pagination_state();
        self.send_pagination_state_update()?;

        // Check if this is an initial load (when we should focus messages list)
        let is_initial_load = self
            .queue_manager
            .queue_state
            .message_pagination
            .current_page
            == 0
            && self
                .queue_manager
                .queue_state
                .message_pagination
                .total_pages_loaded
                == 1;

        if is_initial_load {
            // Initial load: remount with focus and set proper app state
            self.remount_messages_with_focus(true)?;
            self.set_app_state(AppState::MessagePicker);

            // Set focus to messages component
            if let Err(e) = self.app.active(&ComponentId::Messages) {
                log::error!("Failed to activate messages: {}", e);
            }
        } else {
            let messages_currently_focused = self
                .app
                .focus()
                .map(|cid| *cid == ComponentId::Messages)
                .unwrap_or(false);

            if messages_currently_focused {
                // Keep the teal border and cursor position
                self.remount_messages_with_focus(true)?;
            } else {
                // Reset cursor to position 0 when changing pages (no focus)
                self.remount_messages_with_cursor_control(false)?;
            }
        }

        Ok(())
    }

    fn send_pagination_state_update(&self) -> AppResult<()> {
        self.state_manager
            .tx_to_main
            .send(Msg::MessageActivity(
                MessageActivityMsg::PaginationStateUpdated {
                    has_next: self
                        .queue_manager
                        .queue_state
                        .message_pagination
                        .has_next_page,
                    has_previous: self
                        .queue_manager
                        .queue_state
                        .message_pagination
                        .has_previous_page,
                    current_page: self
                        .queue_manager
                        .queue_state
                        .message_pagination
                        .current_page,
                    total_pages_loaded: self
                        .queue_manager
                        .queue_state
                        .message_pagination
                        .total_pages_loaded,
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
        let page_size = config::get_config_or_panic().max_messages();
        let current_page = self
            .queue_manager
            .queue_state
            .message_pagination
            .current_page;
        let current_page_messages = self
            .queue_state()
            .message_pagination
            .get_current_page_messages(page_size);
        let current_page_size = current_page_messages.len();
        let page_is_under_filled = current_page_size < page_size as usize;

        // Check if this page should be auto-filled
        // We auto-fill if the page is under-filled AND we think there might be more messages available
        let should_auto_fill = page_is_under_filled
            && self
                .queue_manager
                .queue_state
                .message_pagination
                .has_next_page;

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
            self.queue_manager
                .queue_state
                .message_pagination
                .last_loaded_sequence
        );

        let tx_to_main = self.state_manager.tx_to_main.clone();
        let service_bus_manager = self.service_bus_manager.clone();

        let from_sequence = self
            .queue_state()
            .message_pagination
            .last_loaded_sequence
            .map(|seq| seq + 1);

        self.task_manager
            .execute("Loading more messages...", async move {
                use server::service_bus_manager::{ServiceBusCommand, ServiceBusResponse};

                let command = ServiceBusCommand::PeekMessages {
                    max_count: message_count,
                    from_sequence,
                };

                let response = service_bus_manager
                    .lock()
                    .await
                    .execute_command(command)
                    .await;

                let messages = match response {
                    ServiceBusResponse::MessagesReceived { messages } => {
                        log::info!(
                            "Loaded {} messages for backfill (requested {})",
                            messages.len(),
                            message_count
                        );
                        messages
                    }
                    ServiceBusResponse::Error { error } => {
                        log::error!("Failed to peek messages for backfill: {}", error);
                        return Err(AppError::ServiceBus(format!(
                            "Failed to peek messages for backfill: {}",
                            error
                        )));
                    }
                    _ => {
                        return Err(AppError::ServiceBus(
                            "Unexpected response for peek messages".to_string(),
                        ));
                    }
                };

                // Send backfill messages (different from NewMessagesLoaded)
                if !messages.is_empty() {
                    if let Err(e) = tx_to_main.send(Msg::MessageActivity(
                        crate::components::common::MessageActivityMsg::BackfillMessagesLoaded(
                            messages,
                        ),
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
            });

        Ok(())
    }
}