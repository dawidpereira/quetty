use crate::app::model::AppState;
use crate::app::model::Model;
use crate::components::common::{ComponentId, MessageActivityMsg, Msg};
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
        let page_size = config::get_config_or_panic().max_messages();
        self.queue_state_mut().message_pagination.update(page_size);

        if let Err(e) = self.remount_messages_with_focus(true) {
            self.error_reporter
                .report_simple(e, "MessageStateHandler", "handle_messages_loaded");
            return None;
        }

        self.set_app_state(AppState::MessagePicker);
        if let Err(e) = self.app.active(&ComponentId::Messages) {
            log::error!("Failed to activate messages: {}", e);
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

        self.reset_pagination_state();
        self.load_messages();
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

    /// Handle new messages loaded (pagination)
    pub fn handle_new_messages_loaded(&mut self, new_messages: Vec<MessageModel>) -> Option<Msg> {
        let is_initial_load = self
            .queue_state()
            .message_pagination
            .all_loaded_messages
            .is_empty();

        self.queue_state_mut()
            .message_pagination
            .add_loaded_page(new_messages);

        if !is_initial_load {
            self.queue_state_mut()
                .message_pagination
                .advance_to_next_page();
        }

        if let Err(e) = self.update_current_page_view() {
            self.error_reporter
                .report_simple(e, "MessageState", "new_messages");
            return None;
        }

        self.set_app_state(AppState::MessagePicker);

        if !self
            .queue_state()
            .message_pagination
            .all_loaded_messages
            .is_empty()
        {
            if let Err(e) = self.remount_message_details(0) {
                self.error_reporter
                    .report_simple(e, "MessageState", "new_messages");
                return None;
            }
        }

        None
    }

    /// Handle backfill messages loaded
    pub fn handle_backfill_messages_loaded(
        &mut self,
        backfill_messages: Vec<MessageModel>,
    ) -> Option<Msg> {
        if backfill_messages.is_empty() {
            log::debug!("No messages loaded for backfill");
            return None;
        }

        self.add_backfill_messages_to_state(backfill_messages);
        self.ensure_pagination_consistency_after_backfill();
        self.update_pagination_and_view()
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
            log::debug!("Removing message ID: {}", msg_id);
        }

        let removed_count = self.remove_messages_from_pagination_state(&message_ids);
        let (target_page, remaining_message_count) =
            self.calculate_pagination_after_removal(removed_count);

        match self.calculate_and_execute_auto_loading(target_page, remaining_message_count) {
            Ok(true) => {
                // Auto-loading initiated, view will be updated when messages arrive
                return None;
            }
            Ok(false) => {
                // No auto-loading needed, continue with normal flow
            }
            Err(e) => {
                log::error!("Failed to auto-load messages after bulk removal: {}", e);
                // Continue with normal flow even if loading fails
            }
        }

        self.finalize_bulk_removal_pagination();
        self.update_pagination_and_view()
    }

    /// Update pagination and view after state changes
    pub fn update_pagination_and_view(&mut self) -> Option<Msg> {
        // Update the pagination state and messages data
        let page_size = config::get_config_or_panic().max_messages();
        let current_page_messages = self
            .queue_state()
            .message_pagination
            .get_current_page_messages(page_size);

        self.queue_state_mut().messages = Some(current_page_messages);

        // Update pagination state
        self.queue_state_mut().message_pagination.update(page_size);

        // Send pagination state update inline
        if let Err(e) = self.state_manager.tx_to_main.send(Msg::MessageActivity(
            MessageActivityMsg::PaginationStateUpdated {
                has_next: self.queue_manager.queue_state.message_pagination.has_next_page,
                has_previous: self.queue_manager.queue_state.message_pagination.has_previous_page,
                current_page: self.queue_manager.queue_state.message_pagination.current_page,
                total_pages_loaded: self.queue_manager.queue_state.message_pagination.total_pages_loaded,
            },
        )) {
            log::error!("Failed to send pagination state update: {}", e);
            self.error_reporter.report_simple(
                AppError::Component(e.to_string()),
                "MessageState",
                "pagination_update",
            );
            return None;
        }

        // Force cursor reset to position 0 after bulk removal
        // This is different from normal page navigation where we preserve cursor
        if let Err(e) = self.remount_messages_with_cursor_control(false) {
            self.error_reporter
                .report_simple(e, "MessageState", "cursor_reset");
            return None;
        }

        self.remount_message_details_safe()
    }

    /// Safely remount message details
    pub fn remount_message_details_safe(&mut self) -> Option<Msg> {
        let current_messages = self
            .queue_state()
            .message_pagination
            .get_current_page_messages(config::get_config_or_panic().max_messages());

        let index = if current_messages.is_empty() {
            0
        } else {
            std::cmp::min(
                self.queue_manager.queue_state.message_pagination.current_page,
                current_messages.len().saturating_sub(1),
            )
        };

        if let Err(e) = self.remount_message_details(index) {
            self.error_reporter
                .report_simple(e, "MessageState", "safe_remount");
            return None;
        }

        None
    }

    /// Calculate pagination after removal
    pub fn calculate_pagination_after_removal(&self, _removed_count: usize) -> (usize, usize) {
        let current_page = self.queue_manager.queue_state.message_pagination.current_page;
        let _total_pages = self.queue_manager.queue_state.message_pagination.total_pages_loaded;

        let current_messages = self
            .queue_state()
            .message_pagination
            .get_current_page_messages(config::get_config_or_panic().max_messages());
        let remaining_on_current_page = current_messages.len();

        // If current page is empty and we have other pages, move to previous page
        let target_page = if remaining_on_current_page == 0 && current_page > 0 {
            current_page - 1
        } else {
            current_page
        };

        let total_remaining_messages = self
            .queue_state()
            .message_pagination
            .all_loaded_messages
            .len();

        (target_page, total_remaining_messages)
    }

    /// Remove messages from pagination state and return count removed
    pub fn remove_messages_from_pagination_state(&mut self, message_ids: &[String]) -> usize {
        let initial_count = self
            .queue_state()
            .message_pagination
            .all_loaded_messages
            .len();
        log::debug!("Initial message count: {}", initial_count);

        // Remove messages from all_loaded_messages
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

        if let Some(ref mut messages) = self.queue_state_mut().messages {
            messages.retain(|msg| !message_ids.contains(&msg.id));
        }

        // Remove messages from bulk selection using proper MessageIdentifier objects
        // We need to find the actual MessageIdentifier objects with correct sequence numbers
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
            self.queue_manager.queue_state.bulk_selection.selected_messages.len()
        );

        self.queue_state_mut()
            .bulk_selection
            .remove_messages(&message_ids_to_remove);

        // If all selected messages were processed, clear the selection entirely
        // This prevents any residual state from accumulating across operations
        if self
            .queue_state()
            .bulk_selection
            .selected_messages
            .is_empty()
        {
            log::debug!("All selected messages processed, clearing bulk selection state");
            self.queue_state_mut().bulk_selection.clear_all();
        }

        let final_count = self
            .queue_state()
            .message_pagination
            .all_loaded_messages
            .len();
        let removed_count = initial_count.saturating_sub(final_count);

        log::info!(
            "Removed {} messages from pagination state (remaining: {})",
            removed_count,
            final_count
        );

        removed_count
    }

    /// Calculate and execute auto-loading if needed (using proper backfill logic)
    pub fn calculate_and_execute_auto_loading(
        &mut self,
        target_page: usize,
        _remaining_message_count: usize,
    ) -> Result<bool, AppError> {
        if target_page != self.queue_manager.queue_state.message_pagination.current_page {
            self.queue_state_mut().message_pagination.current_page = target_page;
        }

        // Update pagination state BEFORE making auto-loading decisions
        // This ensures has_next_page reflects the current state after message removal
        self.update_pagination_state_after_removal();

        let page_size = config::get_config_or_panic().max_messages();
        let current_page = self.queue_manager.queue_state.message_pagination.current_page;
        let current_page_messages = self
            .queue_state()
            .message_pagination
            .get_current_page_messages(page_size);
        let current_page_size = current_page_messages.len();
        let page_is_under_filled = current_page_size < page_size as usize;

        // For single message deletions, be more aggressive about backfilling
        // Check if this looks like a small deletion that should always trigger backfill
        let small_deletion_threshold = config::get_config_or_panic()
            .batch()
            .small_deletion_threshold();
        let is_small_deletion = page_is_under_filled
            && (page_size as usize - current_page_size) <= small_deletion_threshold;

        // Always try to backfill for small deletions, even if has_next_page is false
        // This handles the case where the pagination state hasn't been updated properly
        let should_auto_fill = page_is_under_filled
            && (self.queue_manager.queue_state.message_pagination.has_next_page ||
            is_small_deletion ||
            // Additional condition: if we have any loaded messages beyond the current page
            self.queue_manager.queue_state.message_pagination.all_loaded_messages.len() > (current_page + 1) * page_size as usize);

        if should_auto_fill {
            let messages_needed = page_size as usize - current_page_size;
            log::info!(
                "Page {} is under-filled with {} messages (expected {}), auto-loading {} more (has_next_page: {}, is_small_deletion: {}, total_loaded: {})",
                current_page + 1,
                current_page_size,
                page_size,
                messages_needed,
                self.queue_manager.queue_state.message_pagination.has_next_page,
                is_small_deletion,
                &self.queue_manager.queue_state
                    .message_pagination
                    .all_loaded_messages
                    .len()
            );

            self.load_messages_for_backfill(messages_needed as u32)?;
            return Ok(true);
        } else if page_is_under_filled {
            log::debug!(
                "Page {} has {} messages but no more messages available from API and not a small deletion - no auto-loading needed (has_next_page: {}, total_loaded: {})",
                current_page + 1,
                current_page_size,
                self.queue_manager.queue_state.message_pagination.has_next_page,
                &self.queue_manager.queue_state
                    .message_pagination
                    .all_loaded_messages
                    .len()
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

    /// Update pagination state after message removal (called before auto-loading decisions)
    fn update_pagination_state_after_removal(&mut self) {
        let messages_per_page = config::get_config_or_panic().max_messages();
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
        } else if self.queue_manager.queue_state.message_pagination.current_page >= new_total_pages {
            self.queue_state_mut().message_pagination.current_page = new_total_pages - 1;
        }

        // Update pagination controls based on current state
        self.queue_state_mut().message_pagination.has_previous_page =
            self.queue_manager.queue_state.message_pagination.current_page > 0;

        // For has_next_page, we need to be more optimistic about potential messages
        // If we're on the last loaded page but it's under-filled, there might be more messages
        let current_page_messages = self
            .queue_state()
            .message_pagination
            .get_current_page_messages(messages_per_page);
        let current_page_size = current_page_messages.len();
        let page_is_under_filled = current_page_size < messages_per_page as usize;

        self.queue_state_mut().message_pagination.has_next_page =
            self.queue_manager.queue_state.message_pagination.current_page < new_total_pages.saturating_sub(1)
                || (page_is_under_filled && new_total_pages > 0); // Assume more messages might be available if page is under-filled

        log::debug!(
            "Updated pagination state after removal: page {}/{}, current page: {}, has_next: {}, page_size: {}/{}",
            self.queue_manager.queue_state.message_pagination.current_page + 1,
            new_total_pages,
            self.queue_manager.queue_state.message_pagination.current_page,
            self.queue_manager.queue_state.message_pagination.has_next_page,
            current_page_size,
            messages_per_page
        );
    }

    /// Finalize bulk removal pagination
    pub fn finalize_bulk_removal_pagination(&mut self) {
        let messages_per_page = config::get_config_or_panic().max_messages();

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
        } else if self.queue_manager.queue_state.message_pagination.current_page >= new_total_pages {
            self.queue_state_mut().message_pagination.current_page = new_total_pages - 1;
        }

        // Update pagination controls
        self.queue_state_mut().message_pagination.has_previous_page =
            self.queue_manager.queue_state.message_pagination.current_page > 0;
        self.queue_state_mut().message_pagination.has_next_page =
            self.queue_manager.queue_state.message_pagination.current_page < new_total_pages.saturating_sub(1);

        log::debug!(
            "Finalized pagination: page {}/{}, current page: {}",
            self.queue_manager.queue_state.message_pagination.current_page + 1,
            new_total_pages,
            self.queue_manager.queue_state.message_pagination.current_page
        );
    }

    /// Handle bulk delete completion
    pub fn handle_bulk_delete_completed(
        &mut self,
        successful_count: usize,
        failed_count: usize,
        total_count: usize,
    ) -> Option<Msg> {
        // Show success/status popup
        let queue_name = match &self.queue_manager.queue_state.current_queue_type {
            server::service_bus_manager::QueueType::Main => "main queue",
            server::service_bus_manager::QueueType::DeadLetter => "dead letter queue",
        };

        let success_message = if failed_count == 0 {
            format!(
                "✅ Successfully deleted {} message{} from {}",
                successful_count,
                if successful_count == 1 { "" } else { "s" },
                queue_name
            )
        } else {
            format!(
                "⚠️ Bulk delete completed: {} successful, {} failed from {} (total: {})",
                successful_count, failed_count, queue_name, total_count
            )
        };

        Some(Msg::PopupActivity(
            crate::components::common::PopupActivityMsg::ShowSuccess(success_message),
        ))
    }

    /// Add backfill messages to state
    pub fn add_backfill_messages_to_state(&mut self, messages: Vec<MessageModel>) {
        log::info!("Adding {} backfill messages to state", messages.len());

        self.queue_state_mut()
            .message_pagination
            .all_loaded_messages
            .extend(messages);

        log::debug!(
            "Added backfill messages, total messages now: {}",
            &self.queue_manager.queue_state
                .message_pagination
                .all_loaded_messages
                .len()
        );
    }

    /// Ensure pagination consistency after backfill
    pub fn ensure_pagination_consistency_after_backfill(&mut self) {
        let total_messages = self
            .queue_state()
            .message_pagination
            .all_loaded_messages
            .len();
        let messages_per_page = config::get_config_or_panic().max_messages();

        let new_total_pages = if total_messages == 0 {
            0
        } else {
            total_messages.div_ceil(messages_per_page as usize)
        };

        self.queue_state_mut().message_pagination.total_pages_loaded = new_total_pages;

        if new_total_pages > 0
            && self.queue_manager.queue_state.message_pagination.current_page >= new_total_pages
        {
            self.queue_state_mut().message_pagination.current_page = new_total_pages - 1;
        }

        // Update pagination state
        self.queue_state_mut().message_pagination.has_previous_page =
            self.queue_manager.queue_state.message_pagination.current_page > 0;
        self.queue_state_mut().message_pagination.has_next_page =
            self.queue_manager.queue_state.message_pagination.current_page < new_total_pages.saturating_sub(1);

        log::debug!(
            "Ensured pagination consistency: {}/{} pages, current page: {}",
            self.queue_manager.queue_state.message_pagination.current_page + 1,
            new_total_pages,
            self.queue_manager.queue_state.message_pagination.current_page
        );
    }
}
