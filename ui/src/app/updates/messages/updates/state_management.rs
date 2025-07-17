use crate::app::model::AppState;
use crate::app::model::Model;
use crate::components::common::{ComponentId, Msg};
use crate::config;
use crate::error::AppError;
use server::bulk_operations::MessageIdentifier;
use server::model::MessageModel;
use server::service_bus_manager::QueueInfo;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Handle initial messages loaded
    pub fn handle_messages_loaded(&mut self, messages: Vec<MessageModel>) -> Option<Msg> {
        // Initialize pagination state with the loaded messages
        self.queue_state_mut().message_pagination.reset();
        self.queue_state_mut()
            .message_pagination
            .add_loaded_page(messages.clone());

        // Set the current page messages
        self.queue_state_mut().messages = Some(messages);

        // Update pagination state to calculate has_next_page properly
        let page_size = config::get_current_page_size() as usize;
        self.queue_state_mut()
            .message_pagination
            .update(page_size as u32);

        if let Err(e) = self.remount_messages_with_focus(true) {
            self.error_reporter
                .report_simple(e, "MessageStateHandler", "handle_messages_loaded");
            return None;
        }

        self.set_app_state(AppState::MessagePicker);
        if let Err(e) = self.app.active(&ComponentId::Messages) {
            self.error_reporter.report_activation_error("Messages", &e);
        }

        if let Err(e) = self.remount_message_details(0) {
            self.error_reporter
                .report_simple(e, "MessageStateHandler", "handle_messages_loaded");
            return None;
        }

        self.set_redraw(true);
        Some(Msg::ForceRedraw)
    }

    /// Handle queue switch completion
    pub fn handle_queue_switched(&mut self, queue_info: QueueInfo) -> Option<Msg> {
        // Update the queue state with the new queue information
        self.queue_state_mut().current_queue_name = Some(queue_info.name.clone());
        self.queue_state_mut().current_queue_type = queue_info.queue_type.clone();

        log::info!(
            "Queue switched to: {} (type: {:?})",
            queue_info.name,
            queue_info.queue_type
        );

        // Reset pagination state for new queue
        self.queue_state_mut().message_pagination.reset();

        // Load messages for the new queue using the current page size
        let page_size = config::get_current_page_size();
        if let Err(e) = self.load_messages_from_api_with_count(page_size) {
            log::error!("Failed to load messages after queue switch: {e}");
        }

        None
    }

    /// Handle queue name update
    pub fn handle_queue_name_updated(&mut self, queue_name: String) -> Option<Msg> {
        self.queue_state_mut().current_queue_name = Some(queue_name);
        None
    }

    /// Handle previewing message details
    pub fn handle_preview_message_details(&mut self, index: usize) -> Option<Msg> {
        if let Err(e) = self.remount_message_details(index) {
            self.error_reporter
                .report_simple(e, "MessageState", "preview_details");
            return None;
        }
        None
    }

    /// Handle new messages being loaded
    pub fn handle_new_messages_loaded(&mut self, messages: Vec<MessageModel>) -> Option<Msg> {
        let message_count = messages.len();

        // Clear loading state first
        self.queue_state_mut().message_pagination.set_loading(false);

        // Update the message list
        self.queue_state_mut()
            .message_pagination
            .append_messages(messages);

        // Check if we should advance page after loading
        let page_size = config::get_current_page_size() as usize;
        let current_page = self.queue_state().message_pagination.current_page;
        let total_messages = self.queue_state().message_pagination.total_messages();
        let should_advance = self.queue_state().message_pagination.advance_after_load;

        log::debug!(
            "Messages loaded: count={message_count}, total={total_messages}, current_page={current_page}, should_advance={should_advance}"
        );

        // Handle page advancement if requested
        if should_advance && message_count > 0 {
            let next_page_start = (current_page + 1) * page_size;
            if next_page_start < total_messages {
                log::debug!(
                    "Advancing to next page {} -> {} after loading",
                    current_page,
                    current_page + 1
                );
                self.queue_state_mut().message_pagination.current_page += 1;
            }
            // Clear the flag
            self.queue_state_mut().message_pagination.advance_after_load = false;
        }

        // Update pagination state
        let page_size_u32 = config::get_current_page_size();
        self.queue_state_mut()
            .message_pagination
            .update(page_size_u32);

        log::info!(
            "Loaded {} messages, total: {}, current page: {}",
            message_count,
            total_messages,
            self.queue_state().message_pagination.current_page
        );

        // Check if we need auto-fill due to sequence gaps (only if we loaded some messages but less than expected)
        if message_count > 0
            && message_count < page_size_u32 as usize
            && !self.queue_state().message_pagination.reached_end_of_queue
        {
            let current_page_messages = self
                .queue_state()
                .message_pagination
                .get_current_page_messages(page_size_u32);
            if current_page_messages.len() < page_size_u32 as usize {
                log::info!(
                    "Page incomplete due to sequence gaps ({} < {}), triggering auto-fill",
                    current_page_messages.len(),
                    page_size_u32
                );

                // Try to auto-fill the current page
                match self.should_auto_fill() {
                    Ok(true) => {
                        if let Err(e) = self.execute_auto_fill() {
                            log::error!("Failed to execute auto-fill: {e}");
                        } else {
                            log::debug!("Auto-fill initiated for incomplete page");
                        }
                    }
                    Ok(false) => {
                        log::debug!("Auto-fill not needed or not possible");
                    }
                    Err(e) => {
                        log::error!("Error checking auto-fill: {e}");
                    }
                }
            }
        }

        // Set app state to MessagePicker and focus messages
        self.set_app_state(crate::app::model::AppState::MessagePicker);

        // Update current page view to show the correct messages for the current page
        if let Err(e) = self.update_current_page_view() {
            log::error!("Failed to update current page view after loading: {e}");
        }

        // Focus the messages component if we have messages
        if message_count > 0 {
            if let Err(e) = self
                .app
                .active(&crate::components::common::ComponentId::Messages)
            {
                log::error!("Failed to activate messages component: {e}");
            }

            // Also remount message details for the first message
            if let Err(e) = self.remount_message_details(0) {
                log::error!("Failed to remount message details: {e}");
            }
        }

        None
    }

    /// Handle backfill messages being loaded (uses extend_current_page to not increment page count)
    pub fn handle_backfill_messages_loaded(&mut self, messages: Vec<MessageModel>) -> Option<Msg> {
        let message_count = messages.len();

        // Clear loading state first
        self.queue_state_mut().message_pagination.set_loading(false);

        log::info!(
            "Backfill loaded {message_count} messages, extending current page without incrementing page count"
        );

        // Use extend_current_page instead of append_messages to preserve page structure
        self.queue_state_mut()
            .message_pagination
            .extend_current_page(messages);

        // Update pagination state
        let page_size_u32 = config::get_current_page_size();
        self.queue_state_mut()
            .message_pagination
            .update(page_size_u32);

        // Update current page view to show the correct messages for the current page
        if let Err(e) = self.update_current_page_view() {
            log::error!("Failed to update current page view after backfill: {e}");
        }

        // Focus the messages component if we have messages
        if message_count > 0 {
            if let Err(e) = self
                .app
                .active(&crate::components::common::ComponentId::Messages)
            {
                log::error!("Failed to activate messages component: {e}");
            }

            // Also remount message details for the first message
            if let Err(e) = self.remount_message_details(0) {
                log::error!("Failed to remount message details: {e}");
            }
        }

        log::debug!(
            "Backfill complete: current_page={}, total_messages={}, has_next={}",
            self.queue_state().message_pagination.current_page,
            self.queue_state().message_pagination.total_messages(),
            self.queue_state().message_pagination.has_next_page
        );

        None
    }

    /// Handle bulk removal of messages from state - now simplified and focused
    pub fn handle_bulk_remove_messages_from_state(
        &mut self,
        message_ids: Vec<String>,
    ) -> Option<Msg> {
        if message_ids.is_empty() {
            log::warn!("No message IDs provided for bulk removal");
            return None;
        }

        log::info!(
            "Starting bulk removal for {} message IDs",
            message_ids.len()
        );
        for msg_id in &message_ids {
            log::debug!("Removing message ID: {msg_id}");
        }

        // Invalidate and refresh stats cache for current queue since its size is changing
        if let Some(queue_name) = &self.queue_state().current_queue_name {
            let base_queue_name = if queue_name.ends_with("/$deadletterqueue") {
                queue_name.trim_end_matches("/$deadletterqueue").to_string()
            } else {
                queue_name.clone()
            };
            self.queue_state_mut()
                .stats_manager
                .invalidate_stats_cache_for_queue(&base_queue_name);

            // Immediately refresh the statistics to show updated counts
            if let Err(e) = self.load_queue_statistics_from_api(&base_queue_name) {
                log::error!("Failed to refresh queue statistics after bulk removal: {e}");
            }
        }

        let removed_count = self.remove_messages_from_pagination_state(&message_ids);
        let (target_page, _remaining_message_count) =
            self.calculate_pagination_after_removal(removed_count);

        match self.calculate_and_execute_auto_loading(target_page) {
            Ok(true) => {
                // Auto-loading initiated, view will be updated when messages arrive
                return None;
            }
            Ok(false) => {
                // No auto-loading needed, continue with normal flow
            }
            Err(e) => {
                self.error_reporter.report_loading_error(
                    "MessageState",
                    "auto_load_after_removal",
                    &e,
                );
                // Continue with normal flow even if loading fails
            }
        }

        self.finalize_bulk_removal_pagination();
        self.update_pagination_and_view()
    }

    /// Update pagination and view after state changes
    pub fn update_pagination_and_view(&mut self) -> Option<Msg> {
        // Update the pagination state and messages data
        let page_size = config::get_current_page_size();

        // After message removal, ensure the view shows the correct messages
        let current_page_messages = self
            .queue_state()
            .message_pagination
            .get_current_page_messages(page_size);

        log::debug!(
            "update_pagination_and_view: page={}, total_loaded={}, current_page_count={}",
            self.queue_state().message_pagination.current_page + 1,
            self.queue_state()
                .message_pagination
                .all_loaded_messages
                .len(),
            current_page_messages.len()
        );

        // Update the current page messages in state
        self.queue_state_mut().messages = Some(current_page_messages);

        // Update pagination state
        self.queue_state_mut().message_pagination.update(page_size);

        // Force cursor reset to position 0 after bulk removal
        // This is different from normal page navigation where we preserve cursor
        if let Err(e) = self.remount_messages_with_cursor_control(false) {
            self.error_reporter
                .report_simple(e, "MessageState", "cursor_reset");
        }

        self.remount_message_details_safe()
    }

    /// Safely remount message details
    pub fn remount_message_details_safe(&mut self) -> Option<Msg> {
        let current_messages = self
            .queue_state()
            .message_pagination
            .get_current_page_messages(config::get_current_page_size());

        let index = if current_messages.is_empty() {
            0
        } else {
            std::cmp::min(
                self.queue_manager
                    .queue_state
                    .message_pagination
                    .current_page,
                current_messages.len().saturating_sub(1),
            )
        };

        if let Err(e) = self.remount_message_details(index) {
            self.error_reporter
                .report_simple(e, "MessageState", "safe_remount");
        }

        None
    }

    /// Calculate pagination after removal
    pub fn calculate_pagination_after_removal(&self, _removed_count: usize) -> (usize, usize) {
        let page_size = config::get_current_page_size() as usize;
        let total_remaining_messages = self
            .queue_state()
            .message_pagination
            .all_loaded_messages
            .len();
        let current_page = self
            .queue_manager
            .queue_state
            .message_pagination
            .current_page;

        // Calculate how many pages we can form with remaining messages
        let total_possible_pages = if total_remaining_messages == 0 {
            0
        } else {
            total_remaining_messages.div_ceil(page_size)
        };

        // Determine target page - stay on current page if it has messages, otherwise move to last valid page
        let target_page = if total_possible_pages == 0 {
            0
        } else if current_page >= total_possible_pages {
            // Current page is beyond available data, move to last page
            total_possible_pages.saturating_sub(1)
        } else {
            // Check if current page actually has messages
            let current_page_start = current_page * page_size;
            if current_page_start < total_remaining_messages {
                // Current page has messages, stay here
                current_page
            } else {
                // Current page is empty, move to last valid page
                total_possible_pages.saturating_sub(1)
            }
        };

        log::debug!(
            "calculate_pagination_after_removal: current_page={current_page}, total_remaining={total_remaining_messages}, total_possible_pages={total_possible_pages}, target_page={target_page}"
        );

        (target_page, total_remaining_messages)
    }

    /// Remove messages from pagination state and return count removed
    pub fn remove_messages_from_pagination_state(&mut self, message_ids: &[String]) -> usize {
        let initial_count = self.get_loaded_message_count();
        log::debug!("Initial message count: {initial_count}");

        let removed_count = self.remove_from_loaded_messages(message_ids);
        self.remove_from_current_messages(message_ids);
        self.cleanup_bulk_selection(message_ids);

        self.log_removal_summary(initial_count, removed_count);
        removed_count
    }

    /// Get the current count of loaded messages
    fn get_loaded_message_count(&self) -> usize {
        self.queue_state()
            .message_pagination
            .all_loaded_messages
            .len()
    }

    /// Remove messages from all_loaded_messages and return count removed
    fn remove_from_loaded_messages(&mut self, message_ids: &[String]) -> usize {
        let initial_count = self.get_loaded_message_count();

        self.queue_state_mut()
            .message_pagination
            .all_loaded_messages
            .retain(|msg| {
                let should_keep = !message_ids.contains(&msg.id);

                if !should_keep {
                    log::debug!("Removing message: {} (sequence: {})", msg.id, msg.sequence);
                } else {
                    log::trace!("Keeping message: {} (sequence: {})", msg.id, msg.sequence);
                }

                should_keep
            });

        let final_count = self.get_loaded_message_count();
        initial_count.saturating_sub(final_count)
    }

    /// Remove messages from current page messages collection
    fn remove_from_current_messages(&mut self, message_ids: &[String]) {
        if let Some(ref mut messages) = self.queue_state_mut().messages {
            messages.retain(|msg| !message_ids.contains(&msg.id));
        }
    }

    /// Clean up bulk selection state by removing specified messages
    fn cleanup_bulk_selection(&mut self, message_ids: &[String]) {
        // Find the actual MessageIdentifier objects with correct sequence numbers
        let message_ids_to_remove: Vec<MessageIdentifier> = self
            .queue_state()
            .bulk_selection
            .selected_messages
            .iter()
            .filter(|msg_id| message_ids.contains(&msg_id.id))
            .cloned()
            .collect();

        log::debug!(
            "Removing {} messages from bulk selection (out of {} selected)",
            message_ids_to_remove.len(),
            self.queue_manager
                .queue_state
                .bulk_selection
                .selected_messages
                .len()
        );

        self.queue_state_mut()
            .bulk_selection
            .remove_messages(&message_ids_to_remove);

        // Clear selection entirely if all messages were processed
        if self
            .queue_state()
            .bulk_selection
            .selected_messages
            .is_empty()
        {
            log::debug!("All selected messages processed, clearing bulk selection state");
            self.queue_state_mut().bulk_selection.clear_all();
        }
    }

    /// Log summary of message removal operation
    fn log_removal_summary(&self, _initial_count: usize, removed_count: usize) {
        let final_count = self.get_loaded_message_count();
        log::info!(
            "Removed {removed_count} messages from pagination state (remaining: {final_count})"
        );
    }

    /// Calculate and execute auto-loading if needed (using proper backfill logic)
    pub fn calculate_and_execute_auto_loading(
        &mut self,
        target_page: usize,
    ) -> Result<bool, AppError> {
        self.update_current_page(target_page);
        self.update_pagination_state_after_removal();

        if !self.should_auto_fill()? {
            return Ok(false);
        }

        self.execute_auto_fill()
    }

    /// Update current page if it has changed
    fn update_current_page(&mut self, target_page: usize) {
        if target_page
            != self
                .queue_manager
                .queue_state
                .message_pagination
                .current_page
        {
            self.queue_state_mut().message_pagination.current_page = target_page;
        }
    }

    /// Determine if auto-fill should be executed based on current state
    fn should_auto_fill(&self) -> Result<bool, AppError> {
        let page_size = config::get_current_page_size();
        let current_page = self
            .queue_manager
            .queue_state
            .message_pagination
            .current_page;

        // Get actual current page messages using the pagination system
        let current_page_messages = self
            .queue_state()
            .message_pagination
            .get_current_page_messages(page_size);
        let actual_messages_on_page = current_page_messages.len();

        // Get total remaining messages
        let total_remaining_messages = self
            .queue_state()
            .message_pagination
            .all_loaded_messages
            .len();

        log::debug!(
            "Auto-fill check: page={}, total_remaining={}, actual_on_page={}, target_page_size={}, has_next_page={}, reached_end={}",
            current_page + 1,
            total_remaining_messages,
            actual_messages_on_page,
            page_size,
            self.queue_manager
                .queue_state
                .message_pagination
                .has_next_page,
            self.queue_manager
                .queue_state
                .message_pagination
                .reached_end_of_queue
        );

        // If page is full (has target page size), no auto-loading needed
        if actual_messages_on_page >= page_size as usize {
            log::debug!(
                "Page {} is full with {} messages (target: {}) - no auto-loading needed",
                current_page + 1,
                actual_messages_on_page,
                page_size
            );
            return Ok(false);
        }

        // If we've reached the end of the queue, no more messages available
        if self
            .queue_manager
            .queue_state
            .message_pagination
            .reached_end_of_queue
        {
            log::debug!("Reached end of queue - no auto-loading possible");
            return Ok(false);
        }

        // If we don't have next page capability from server, no auto-loading
        if !self
            .queue_manager
            .queue_state
            .message_pagination
            .has_next_page
        {
            log::debug!("No next page available from server - no auto-loading possible");
            return Ok(false);
        }

        // If the current page is under-filled but we still have plenty of messages for future pages,
        // then it might be due to bulk deletion and we shouldn't auto-fill unnecessarily.
        let messages_consumed_by_current_page = (current_page + 1) * (page_size as usize);
        let messages_available_for_future_pages =
            total_remaining_messages.saturating_sub(messages_consumed_by_current_page);
        let future_pages_worth = page_size as usize; // At least 1 full page worth for future

        // Only skip auto-fill if we have plenty of messages for future pages AND this looks like a bulk deletion scenario
        if messages_available_for_future_pages >= future_pages_worth && actual_messages_on_page > 0
        {
            log::debug!(
                "Current page has {actual_messages_on_page} messages, {messages_available_for_future_pages} messages available for future pages (>= {future_pages_worth} threshold) - likely bulk deletion scenario, skipping auto-fill"
            );
            return Ok(false);
        }

        // Page is under-filled and we should auto-load to fill it properly
        let messages_needed = page_size as usize - actual_messages_on_page;
        log::info!(
            "Page {} is under-filled ({} actual vs {} target) with {} available for future - will auto-load {} messages",
            current_page + 1,
            actual_messages_on_page,
            page_size,
            messages_available_for_future_pages,
            messages_needed
        );

        Ok(true)
    }

    /// Execute auto-fill by loading additional messages
    fn execute_auto_fill(&mut self) -> Result<bool, AppError> {
        let page_size = config::get_current_page_size();
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
        let messages_needed = page_size as usize - current_page_size;

        log::info!(
            "Page {} is under-filled with {} messages (expected {}), auto-loading {} more (has_next_page: {}, total_loaded: {})",
            current_page + 1,
            current_page_size,
            page_size,
            messages_needed,
            self.queue_manager
                .queue_state
                .message_pagination
                .has_next_page,
            self.queue_manager
                .queue_state
                .message_pagination
                .all_loaded_messages
                .len()
        );

        self.load_messages_for_backfill(messages_needed as u32)?;
        Ok(true)
    }

    /// Update pagination state after message removal (called before auto-loading decisions)
    fn update_pagination_state_after_removal(&mut self) {
        let messages_per_page = config::get_current_page_size();
        let total_messages = self
            .queue_state()
            .message_pagination
            .all_loaded_messages
            .len();

        let new_total_pages = if total_messages == 0 {
            0
        } else {
            total_messages.div_ceil(messages_per_page as usize)
        };

        self.queue_state_mut().message_pagination.total_pages_loaded = new_total_pages;

        // Ensure current page is within bounds
        if new_total_pages == 0 {
            self.queue_state_mut().message_pagination.current_page = 0;
        } else if self
            .queue_manager
            .queue_state
            .message_pagination
            .current_page
            >= new_total_pages
        {
            self.queue_state_mut().message_pagination.current_page =
                new_total_pages.saturating_sub(1);
        }

        // Update pagination controls based on current state
        self.queue_state_mut().message_pagination.has_previous_page = self
            .queue_manager
            .queue_state
            .message_pagination
            .current_page
            > 0;

        // For has_next_page, we need to be more optimistic about potential messages
        // If we're on the last loaded page but it's under-filled, there might be more messages
        let current_page_messages = self
            .queue_state()
            .message_pagination
            .get_current_page_messages(messages_per_page);
        let current_page_size = current_page_messages.len();
        let page_is_under_filled = current_page_size < messages_per_page as usize;

        self.queue_state_mut().message_pagination.has_next_page = self
            .queue_manager
            .queue_state
            .message_pagination
            .current_page
            < new_total_pages.saturating_sub(1)
            || (page_is_under_filled && new_total_pages > 0); // Assume more messages might be available if page is under-filled

        log::debug!(
            "Updated pagination state after removal: page {}/{}, current page: {}, has_next: {}, page_size: {}/{}",
            self.queue_manager
                .queue_state
                .message_pagination
                .current_page
                + 1,
            new_total_pages,
            self.queue_manager
                .queue_state
                .message_pagination
                .current_page,
            self.queue_manager
                .queue_state
                .message_pagination
                .has_next_page,
            current_page_size,
            messages_per_page
        );
    }

    /// Finalize bulk removal pagination
    pub fn finalize_bulk_removal_pagination(&mut self) {
        let messages_per_page = config::get_current_page_size();

        let total_messages = self
            .queue_state()
            .message_pagination
            .all_loaded_messages
            .len();
        let new_total_pages = if total_messages == 0 {
            0
        } else {
            total_messages.div_ceil(messages_per_page as usize)
        };

        self.queue_state_mut().message_pagination.total_pages_loaded = new_total_pages;

        // Ensure current page is within bounds
        if new_total_pages == 0 {
            self.queue_state_mut().message_pagination.current_page = 0;
        } else if self
            .queue_manager
            .queue_state
            .message_pagination
            .current_page
            >= new_total_pages
        {
            self.queue_state_mut().message_pagination.current_page =
                new_total_pages.saturating_sub(1);
        }

        // Update pagination controls
        self.queue_state_mut().message_pagination.has_previous_page = self
            .queue_manager
            .queue_state
            .message_pagination
            .current_page
            > 0;
        self.queue_state_mut().message_pagination.has_next_page = self
            .queue_manager
            .queue_state
            .message_pagination
            .current_page
            < new_total_pages.saturating_sub(1);

        log::debug!(
            "Finalized pagination: page {}/{}, current page: {}",
            self.queue_manager
                .queue_state
                .message_pagination
                .current_page
                + 1,
            new_total_pages,
            self.queue_manager
                .queue_state
                .message_pagination
                .current_page
        );
    }

    /// Handle bulk delete completion
    pub fn handle_bulk_delete_completed(
        &mut self,
        successful_count: usize,
        failed_count: usize,
        total_count: usize,
    ) -> Option<Msg> {
        // Invalidate and refresh stats cache for current queue since messages were deleted from it
        if let Some(queue_name) = &self.queue_state().current_queue_name {
            let base_queue_name = if queue_name.ends_with("/$deadletterqueue") {
                queue_name.trim_end_matches("/$deadletterqueue").to_string()
            } else {
                queue_name.clone()
            };
            self.queue_state_mut()
                .stats_manager
                .invalidate_stats_cache_for_queue(&base_queue_name);

            // Immediately refresh the statistics to show updated counts
            if let Err(e) = self.load_queue_statistics_from_api(&base_queue_name) {
                log::error!("Failed to refresh queue statistics after bulk delete: {e}");
            }
        }

        // Show success/status popup
        let queue_name = match &self.queue_manager.queue_state.current_queue_type {
            server::service_bus_manager::QueueType::Main => "main queue",
            server::service_bus_manager::QueueType::DeadLetter => "dead letter queue",
        };

        let not_found_count = total_count.saturating_sub(successful_count + failed_count);

        let success_message = crate::app::bulk_operation_processor::BulkOperationPostProcessor::format_bulk_operation_result_message(
            "delete",
            queue_name,
            successful_count,
            failed_count,
            not_found_count,
            total_count,
            true, // is_delete
        );

        Some(Msg::PopupActivity(
            crate::components::common::PopupActivityMsg::ShowSuccess(success_message),
        ))
    }

    /// Handle queue statistics update
    pub fn handle_queue_stats_updated(
        &mut self,
        stats_cache: crate::app::updates::messages::pagination::QueueStatsCache,
    ) -> Option<Msg> {
        log::info!("Updating queue stats cache for: {}", stats_cache.queue_name);

        // Update the cache
        self.queue_state_mut()
            .stats_manager
            .update_stats_cache(stats_cache);

        // Remount to show updated stats
        if let Err(e) = self.remount_messages() {
            log::error!("Failed to remount messages after stats update: {e}");
        }

        None
    }

    /// Load queue statistics from API (background, non-blocking)
    pub fn load_queue_statistics_from_api(
        &mut self,
        base_queue_name: &str,
    ) -> crate::error::AppResult<()> {
        let queue_name = base_queue_name.to_string();
        let Some(service_bus_manager) = self.service_bus_manager.clone() else {
            log::warn!("Service bus manager not initialized, cannot load queue statistics");
            return Ok(());
        };
        let tx_to_main = self.state_manager.tx_to_main.clone();

        log::info!("Loading statistics from API for queue: {queue_name}");

        self.task_manager.execute_background(async move {
            use server::service_bus_manager::{ServiceBusCommand, ServiceBusResponse};

            let stats_command = ServiceBusCommand::GetQueueStatistics {
                queue_name: queue_name.clone(),
                queue_type: server::service_bus_manager::QueueType::Main, // We get both counts regardless
            };

            match service_bus_manager
                .lock()
                .await
                .execute_command(stats_command)
                .await
            {
                ServiceBusResponse::QueueStatistics {
                    queue_name,
                    active_message_count,
                    dead_letter_message_count,
                    ..
                } => match (active_message_count, dead_letter_message_count) {
                    (Some(active_count), Some(dlq_count)) => {
                        log::info!(
                            "Loaded stats for {queue_name}: active={active_count}, dlq={dlq_count}"
                        );

                        let stats_cache =
                            crate::app::updates::messages::pagination::QueueStatsCache::new(
                                queue_name,
                                active_count,
                                dlq_count,
                            );

                        if let Err(e) =
                            tx_to_main.send(crate::components::common::Msg::MessageActivity(
                                crate::components::common::MessageActivityMsg::QueueStatsUpdated(
                                    stats_cache,
                                ),
                            ))
                        {
                            log::error!("Failed to send queue stats update: {e}");
                        }
                    }
                    _ => {
                        log::info!(
                            "Queue statistics not available for {queue_name} - no cache update"
                        );
                    }
                },
                ServiceBusResponse::Error { error } => {
                    // Check if this is an auth method incompatibility error
                    if error.to_string().contains("Unsupported auth method") {
                        log::info!(
                            "Queue statistics not available for authentication method: {error}"
                        );
                        // Don't update cache with 0 values for auth method incompatibility
                        return Ok(());
                    }

                    log::warn!("Failed to load queue stats for {queue_name}: {error}");
                    // For other errors, still don't update cache to prevent corruption
                }
                _ => {
                    log::warn!("Unexpected response for queue statistics for {queue_name}");
                }
            }

            Ok::<(), crate::error::AppError>(())
        });

        Ok(())
    }

    /// Handle refresh queue statistics request (triggered after bulk operations)
    pub fn handle_refresh_queue_statistics(&mut self) -> Option<Msg> {
        if let Some(queue_name) = &self.queue_state().current_queue_name {
            let base_queue_name = if queue_name.ends_with("/$deadletterqueue") {
                queue_name.trim_end_matches("/$deadletterqueue").to_string()
            } else {
                queue_name.clone()
            };

            log::info!("Refreshing queue statistics for: {base_queue_name}");

            // Invalidate current cache to force fresh data
            self.queue_state_mut()
                .stats_manager
                .invalidate_stats_cache_for_queue(&base_queue_name);

            // Load fresh statistics from API
            if let Err(e) = self.load_queue_statistics_from_api(&base_queue_name) {
                log::error!("Failed to refresh queue statistics: {e}");
            }
        }
        None
    }
}
