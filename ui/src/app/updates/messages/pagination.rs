use crate::app::model::Model;
use crate::components::common::Msg;
use crate::config;
use crate::error::AppResult;
use server::model::MessageModel;
use server::service_bus_manager::QueueType;

use tuirealm::terminal::TerminalAdapter;

/// Simple cache for queue statistics
#[derive(Debug, Clone, PartialEq)]
pub struct QueueStatsCache {
    pub queue_name: String,
    pub auth_method: String,
    pub active_count: u64,
    pub dlq_count: u64,
    pub fetched_at: chrono::DateTime<chrono::Utc>,
    pub ttl_seconds: u64,
}

impl QueueStatsCache {
    pub fn new(queue_name: String, active_count: u64, dlq_count: u64) -> Self {
        let config = crate::config::get_config_or_panic();
        Self {
            queue_name,
            auth_method: config.azure_ad().auth_method.clone(),
            active_count,
            dlq_count,
            fetched_at: chrono::Utc::now(),
            ttl_seconds: config.queue_stats_cache_ttl_seconds(),
        }
    }

    /// Get the compound cache key that includes both queue name and auth method
    pub fn cache_key(&self) -> String {
        format!("{}_{}", self.queue_name, self.auth_method)
    }

    /// Create a cache key for a given queue name and auth method
    pub fn make_cache_key(queue_name: &str, auth_method: &str) -> String {
        format!("{queue_name}_{auth_method}")
    }

    pub fn is_expired(&self) -> bool {
        let age = chrono::Utc::now()
            .signed_duration_since(self.fetched_at)
            .num_seconds() as u64;
        age > self.ttl_seconds
    }

    pub fn get_count_for_type(&self, queue_type: &QueueType) -> u64 {
        match queue_type {
            QueueType::Main => self.active_count,
            QueueType::DeadLetter => self.dlq_count,
        }
    }

    pub fn age_seconds(&self) -> i64 {
        chrono::Utc::now()
            .signed_duration_since(self.fetched_at)
            .num_seconds()
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
    pub page_start_indices: Vec<usize>,
    /// Track if we're currently loading messages to prevent concurrent operations
    pub is_loading_messages: bool,
    /// Flag to indicate we should advance page after loading messages
    pub advance_after_load: bool,
}

impl MessagePaginationState {
    /// Calculate page bounds for the current page
    pub fn calculate_page_bounds(&self, page_size: u32) -> (usize, usize) {
        let page_size = page_size as usize;
        let start_idx = self.current_page * page_size;
        let end_idx = std::cmp::min(start_idx + page_size, self.all_loaded_messages.len());
        (start_idx, end_idx)
    }

    /// Reset pagination state
    pub fn reset(&mut self) {
        self.current_page = 0;
        self.has_next_page = false;
        self.has_previous_page = false;
        self.total_pages_loaded = 0;
        self.last_loaded_sequence = None;
        self.all_loaded_messages.clear();
        self.reached_end_of_queue = false;
        self.page_start_indices = vec![0]; // Initialize with page 0 starting at index 0
        self.is_loading_messages = false;
        self.advance_after_load = false;
    }

    /// Update pagination state based on current page and total loaded pages
    pub fn update(&mut self, page_size: u32) {
        self.has_previous_page = self.current_page > 0;
        self.has_next_page = self.calculate_has_next_page(page_size);

        // Ensure current page is within bounds
        let page_size_usize = page_size as usize;
        let max_pages = if self.all_loaded_messages.is_empty() {
            0
        } else {
            self.all_loaded_messages.len().div_ceil(page_size_usize)
        };

        if self.current_page >= max_pages && max_pages > 0 {
            self.current_page = max_pages - 1;
        }
    }

    /// Calculate if there's a next page available
    fn calculate_has_next_page(&self, page_size: u32) -> bool {
        // If we've confirmed we've reached the end of the queue, no more pages
        if self.reached_end_of_queue {
            return false;
        }

        let page_size_usize = page_size as usize;

        // Calculate if we have enough messages for a next page
        let total_messages = self.all_loaded_messages.len();
        let current_page_start = self.current_page * page_size_usize;
        let current_page_end = current_page_start + page_size_usize;

        // If current page is full and we have more messages, there's a next page
        if current_page_end < total_messages {
            return true;
        }

        // If current page isn't full but we haven't reached end of queue, allow loading more
        if current_page_start < total_messages && !self.reached_end_of_queue {
            return true;
        }

        // If no messages loaded yet, we can try to load
        if total_messages == 0 && !self.reached_end_of_queue {
            return true;
        }

        false
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

    /// Get current page messages
    pub fn get_current_page_messages(&self, page_size: u32) -> Vec<MessageModel> {
        let (start_idx, end_idx) = self.calculate_page_bounds(page_size);

        log::debug!(
            "Getting page messages: current_page={}, start_idx={}, end_idx={}, total_messages={}",
            self.current_page,
            start_idx,
            end_idx,
            self.all_loaded_messages.len()
        );

        if start_idx < self.all_loaded_messages.len() {
            let page_messages = self.all_loaded_messages[start_idx..end_idx].to_vec();
            log::debug!(
                "Returning {} messages for page {}",
                page_messages.len(),
                self.current_page
            );

            // Log first and last message IDs for debugging
            if !page_messages.is_empty() {
                let first_id = page_messages
                    .first()
                    .map(|m| m.id.as_str())
                    .unwrap_or("<unknown>");
                let last_id = page_messages
                    .last()
                    .map(|m| m.id.as_str())
                    .unwrap_or("<unknown>");
                log::debug!(
                    "Page {} messages: first_id={}, last_id={}",
                    self.current_page,
                    first_id,
                    last_id
                );
            }

            page_messages
        } else {
            log::debug!(
                "start_idx {} >= total_messages {}, returning empty",
                start_idx,
                self.all_loaded_messages.len()
            );
            Vec::new()
        }
    }

    fn update_last_loaded_sequence(&mut self, messages: &[MessageModel]) {
        if let Some(last_msg) = messages.last() {
            self.last_loaded_sequence = Some(last_msg.sequence);
        }
    }

    /// Simple append messages without complex pagination logic
    pub fn append_messages(&mut self, messages: Vec<MessageModel>) {
        if messages.is_empty() {
            self.reached_end_of_queue = true;
            return;
        }

        self.all_loaded_messages.extend(messages.clone());
        self.update_last_loaded_sequence(&messages);

        // Update pagination state to reflect new messages
        let page_size = config::get_current_page_size() as usize;
        let new_total_pages = if self.all_loaded_messages.is_empty() {
            0
        } else {
            self.all_loaded_messages.len().div_ceil(page_size)
        };
        self.total_pages_loaded = new_total_pages;

        log::debug!(
            "Appended {} messages, total: {}, pages: {}",
            messages.len(),
            self.all_loaded_messages.len(),
            new_total_pages
        );
    }

    /// Extend the current page with additional messages (for backfill functionality)
    /// This method adds messages without incrementing the page count, used for filling incomplete pages
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

    /// Get total number of messages loaded
    pub fn total_messages(&self) -> usize {
        self.all_loaded_messages.len()
    }

    /// Set loading state
    pub fn set_loading(&mut self, loading: bool) {
        self.is_loading_messages = loading;
    }

    /// Check if currently loading
    pub fn is_loading(&self) -> bool {
        self.is_loading_messages
    }
}

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    // Simple pagination handlers
    pub fn handle_next_page_request(&mut self) -> Option<Msg> {
        // Check if we're already loading to prevent race conditions
        if self.is_loading() {
            log::debug!("Already loading, ignoring next page request");
            return None;
        }

        let current_page = self
            .queue_manager
            .queue_state
            .message_pagination
            .current_page;
        let has_next = self
            .queue_manager
            .queue_state
            .message_pagination
            .has_next_page;
        let total_messages = self
            .queue_manager
            .queue_state
            .message_pagination
            .all_loaded_messages
            .len();

        log::debug!(
            "Next page request: current_page={current_page}, has_next={has_next}, total_messages={total_messages}"
        );

        // Always call handle_next_page - it will check if it can actually go forward
        if let Err(e) = self.handle_next_page() {
            self.error_reporter
                .report_simple(e, "Pagination", "next_page");
            return None;
        }

        None
    }

    pub fn handle_previous_page_request(&mut self) -> Option<Msg> {
        // Check if we're already loading to prevent race conditions
        if self.is_loading() {
            log::debug!("Already loading, ignoring previous page request");
            return None;
        }

        let current_page = self
            .queue_manager
            .queue_state
            .message_pagination
            .current_page;
        let has_previous = self
            .queue_manager
            .queue_state
            .message_pagination
            .has_previous_page;

        log::debug!(
            "Previous page request: current_page={current_page}, has_previous={has_previous}"
        );

        // Always call handle_previous_page - it will check if it can actually go back
        if let Err(e) = self.handle_previous_page() {
            self.error_reporter
                .report_simple(e, "Pagination", "prev_page");
            return None;
        }

        None
    }

    // Simple pagination navigation
    pub fn handle_next_page(&mut self) -> AppResult<()> {
        let page_size = config::get_current_page_size() as usize;
        let current_page = self.queue_state().message_pagination.current_page;
        let total_messages = self
            .queue_state()
            .message_pagination
            .all_loaded_messages
            .len();
        let next_page_start = (current_page + 1) * page_size;

        log::debug!(
            "Next page request: current_page={}, total_messages={}, next_page_start={}, reached_end={}",
            current_page,
            total_messages,
            next_page_start,
            self.queue_state().message_pagination.reached_end_of_queue
        );

        // Check if we have enough messages for the next page
        if next_page_start < total_messages {
            // We have enough messages, just advance to next page
            log::debug!(
                "Advancing to next page {} -> {} (have {} messages)",
                current_page,
                current_page + 1,
                total_messages
            );
            self.queue_state_mut().message_pagination.current_page += 1;
            self.queue_state_mut()
                .message_pagination
                .update(page_size as u32);

            log::debug!(
                "After next page: current={}, has_next={}, has_previous={}",
                self.queue_state().message_pagination.current_page,
                self.queue_state().message_pagination.has_next_page,
                self.queue_state().message_pagination.has_previous_page
            );

            if let Err(e) = self.update_current_page_view() {
                log::error!("Failed to update page view: {e}");
            }
        } else if !self.queue_state().message_pagination.reached_end_of_queue {
            // Need to load more messages first, then advance page
            log::debug!(
                "Loading more messages for next page (current: {current_page}, total: {total_messages})"
            );
            // Set a flag to indicate we want to advance after loading
            self.queue_state_mut().message_pagination.advance_after_load = true;
            self.load_messages_from_api_with_count(config::get_current_page_size())?;
        } else {
            log::debug!("Already at end of queue, cannot go to next page");
        }

        // Check if cache is expired and reload stats if needed
        self.check_and_reload_stats_if_expired();

        Ok(())
    }

    pub fn handle_previous_page(&mut self) -> AppResult<()> {
        // Just go to previous page in loaded messages
        if self.queue_state().message_pagination.current_page > 0 {
            log::debug!(
                "Going to previous page {} -> {}",
                self.queue_state().message_pagination.current_page,
                self.queue_state().message_pagination.current_page - 1
            );
            self.queue_state_mut().message_pagination.current_page -= 1;
            let page_size = config::get_current_page_size();
            self.queue_state_mut().message_pagination.update(page_size);

            log::debug!(
                "After previous page: current={}, has_next={}, has_previous={}",
                self.queue_state().message_pagination.current_page,
                self.queue_state().message_pagination.has_next_page,
                self.queue_state().message_pagination.has_previous_page
            );

            if let Err(e) = self.update_current_page_view() {
                log::error!("Failed to update page view: {e}");
            }
        } else {
            log::debug!("Already at first page, cannot go to previous page");
        }

        // Check if cache is expired and reload stats if needed
        self.check_and_reload_stats_if_expired();

        Ok(())
    }

    pub fn update_current_page_view(&mut self) -> AppResult<()> {
        // Update the current page messages in queue state before remounting
        let page_size = config::get_current_page_size();
        let current_page_messages = self
            .queue_state()
            .message_pagination
            .get_current_page_messages(page_size);

        log::debug!(
            "Updating current page view: page={}, messages_count={}",
            self.queue_state().message_pagination.current_page,
            current_page_messages.len()
        );

        if !current_page_messages.is_empty() {
            let first_id = current_page_messages
                .first()
                .map(|m| m.id.as_str())
                .unwrap_or("<unknown>");
            let last_id = current_page_messages
                .last()
                .map(|m| m.id.as_str())
                .unwrap_or("<unknown>");
            log::debug!("Current page messages: first_id={first_id}, last_id={last_id}");
        }

        // Update the queue state messages to reflect current page
        self.queue_state_mut().messages = Some(current_page_messages);

        if let Err(e) = self.remount_messages() {
            log::error!("Failed to remount messages: {e}");
        }
        Ok(())
    }

    // Missing method stubs for compatibility
    pub fn reset_pagination_state(&mut self) {
        self.queue_state_mut().message_pagination.reset();
    }

    pub fn load_messages_for_backfill(&mut self, message_count: u32) -> AppResult<()> {
        // Check if already loading to prevent concurrent operations
        if self.queue_state().message_pagination.is_loading() {
            log::debug!("Already loading messages, skipping backfill request");
            return Ok(());
        }

        log::info!(
            "Loading {} messages for backfill, last_sequence: {:?}",
            message_count,
            self.queue_state().message_pagination.last_loaded_sequence
        );

        // Set loading state
        self.queue_state_mut().message_pagination.set_loading(true);

        let tx_to_main = self.state_manager.tx_to_main.clone();
        let service_bus_manager = match self.get_service_bus_manager() {
            Some(manager) => manager,
            None => {
                log::error!(
                    "Service Bus manager not initialized - cannot load messages for backfill"
                );
                self.queue_state_mut().message_pagination.set_loading(false);
                return Err(crate::error::AppError::Config(
                    "Service Bus manager not initialized. Please configure authentication first."
                        .to_string(),
                ));
            }
        };
        let from_sequence = self
            .queue_state()
            .message_pagination
            .last_loaded_sequence
            .map(|seq| seq + 1);

        self.task_manager
            .execute("Loading additional messages...", async move {
                let result = Self::execute_backfill_task(
                    tx_to_main.clone(),
                    service_bus_manager,
                    from_sequence,
                    message_count,
                )
                .await;

                // Always send a message to clear loading state, even on error
                if let Err(e) = &result {
                    log::error!("Error in backfill task: {e}");
                    // Send empty message list to clear loading state
                    let _ = tx_to_main.send(crate::components::common::Msg::MessageActivity(
                        crate::components::common::MessageActivityMsg::BackfillMessagesLoaded(
                            Vec::new(),
                        ),
                    ));
                }
                result
            });

        Ok(())
    }

    /// Execute backfill loading task
    async fn execute_backfill_task(
        tx_to_main: std::sync::mpsc::Sender<crate::components::common::Msg>,
        service_bus_manager: std::sync::Arc<
            tokio::sync::Mutex<server::service_bus_manager::ServiceBusManager>,
        >,
        from_sequence: Option<i64>,
        message_count: u32,
    ) -> Result<(), crate::error::AppError> {
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
                log::info!("Loaded {} messages for backfill", messages.len());

                // Debug: log sequence range of received messages
                if !messages.is_empty() {
                    let first_seq = messages.first().map(|m| m.sequence).unwrap_or(-1);
                    let last_seq = messages.last().map(|m| m.sequence).unwrap_or(-1);
                    log::debug!(
                        "Backfill messages with sequences: {} to {} (count: {})",
                        first_seq,
                        last_seq,
                        messages.len()
                    );
                }

                messages
            }
            ServiceBusResponse::Error { error } => {
                log::error!("Failed to load messages for backfill: {error}");
                return Err(crate::error::AppError::ServiceBus(error.to_string()));
            }
            _ => {
                return Err(crate::error::AppError::ServiceBus(
                    "Unexpected response for backfill peek messages".to_string(),
                ));
            }
        };

        // Send messages with BackfillMessagesLoaded to trigger extend_current_page logic
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::MessageActivity(
            crate::components::common::MessageActivityMsg::BackfillMessagesLoaded(messages),
        )) {
            log::error!("Failed to send backfill messages: {e}");
            return Err(crate::error::AppError::Component(e.to_string()));
        }

        Ok(())
    }

    /// Check if stats cache is expired and reload if needed
    fn check_and_reload_stats_if_expired(&mut self) {
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

        if self
            .queue_state()
            .stats_manager
            .is_stats_cache_expired(&base_queue_name)
        {
            log::info!("Stats cache expired for {base_queue_name}, reloading from API");
            if let Err(e) = self.load_queue_statistics_from_api(&base_queue_name) {
                log::error!("Failed to reload queue statistics: {e}");
            }
        }
    }

    /// Check if currently loading messages
    fn is_loading(&self) -> bool {
        self.queue_manager
            .queue_state
            .message_pagination
            .is_loading_messages
    }
}
