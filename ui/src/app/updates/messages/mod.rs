pub mod bulk;
pub mod bulk_execution;
pub mod delete;
pub mod dlq;
pub mod loading;
pub mod pagination;
pub mod utils;

// Re-export commonly used types
pub use pagination::MessagePaginationState;
use server::bulk_operations::MessageIdentifier;

use crate::app::model::{AppState, Model};
use crate::components::common::{MessageActivityMsg, Msg};
use crate::config::CONFIG;
use crate::error::AppError;
use server::consumer::Consumer;
use server::model::MessageModel;
use std::sync::Arc;
use tokio::sync::Mutex;
use tuirealm::terminal::TerminalAdapter;

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
            MessageActivityMsg::BackfillMessagesLoaded(backfill_messages) => {
                self.handle_backfill_messages_loaded(backfill_messages)
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
            MessageActivityMsg::DeleteMessage(index) => self.handle_delete_message(index),
            MessageActivityMsg::RemoveMessageFromState(message_id, message_sequence) => {
                self.handle_remove_message_from_state(message_id, message_sequence)
            }

            // Bulk selection handlers
            MessageActivityMsg::ToggleMessageSelection(message_id) => {
                self.handle_toggle_message_selection(message_id)
            }
            MessageActivityMsg::ToggleMessageSelectionByIndex(index) => {
                self.handle_toggle_message_selection_by_index(index)
            }
            MessageActivityMsg::SelectAllCurrentPage => self.handle_select_all_current_page(),
            MessageActivityMsg::SelectAllLoadedMessages => self.handle_select_all_loaded_messages(),
            MessageActivityMsg::ClearAllSelections => self.handle_clear_all_selections(),
            MessageActivityMsg::EnterBulkMode => self.handle_enter_bulk_mode(),
            MessageActivityMsg::ExitBulkMode => self.handle_exit_bulk_mode(),

            // Bulk operation handlers - with selected items
            MessageActivityMsg::BulkDeleteSelected => {
                if let Some(msg) = self.handle_bulk_delete_selected() {
                    Some(msg)
                } else {
                    // Fall back to single message delete using current cursor position
                    let index = 0; // We'd need to get the actual cursor position
                    self.handle_delete_message(index)
                }
            }
            MessageActivityMsg::BulkSendSelectedToDLQ => {
                if let Some(msg) = self.handle_bulk_send_selected_to_dlq() {
                    Some(msg)
                } else {
                    // Fall back to single message DLQ using current cursor position
                    let index = 0; // We'd need to get the actual cursor position
                    self.handle_send_message_to_dlq(index)
                }
            }
            MessageActivityMsg::BulkResendSelectedFromDLQ(delete_from_dlq) => {
                if let Some(msg) = self.handle_bulk_resend_selected_from_dlq(delete_from_dlq) {
                    Some(msg)
                } else {
                    // Fall back to single message resend using current cursor position
                    let index = 0; // We'd need to get the actual cursor position
                    self.handle_resend_message_from_dlq(index)
                }
            }

            // Bulk operation handlers - with specific message lists
            MessageActivityMsg::BulkDeleteMessages(message_ids) => {
                self.handle_bulk_delete_execution(message_ids)
            }
            MessageActivityMsg::BulkSendToDLQ(message_ids) => {
                self.handle_bulk_send_to_dlq_execution(message_ids)
            }
            MessageActivityMsg::BulkResendFromDLQ(message_ids, delete_from_dlq) => {
                // This is the confirmed execution - actually perform the bulk resend
                if delete_from_dlq {
                    self.handle_bulk_resend_from_dlq_execution(message_ids)
                } else {
                    self.handle_bulk_resend_from_dlq_only_execution(message_ids)
                }
            }
            MessageActivityMsg::BulkRemoveMessagesFromState(message_ids) => {
                self.handle_bulk_remove_messages_from_state(message_ids)
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

    fn handle_backfill_messages_loaded(
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

    /// Handle single message removal from state
    fn handle_remove_message_from_state(
        &mut self,
        message_id: String,
        message_sequence: i64,
    ) -> Option<Msg> {
        let page_size = CONFIG.max_messages();

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

        // Also remove the message from bulk selection if it was selected
        let message_identifier = MessageIdentifier::new(message_id, message_sequence);
        self.queue_state
            .bulk_selection
            .remove_messages(&[message_identifier]);

        // Check if we need to load more messages only if ALL messages are gone
        let should_load_more = self
            .queue_state
            .message_pagination
            .all_loaded_messages
            .is_empty();

        if should_load_more {
            log::info!("All messages removed, attempting to load more messages...");

            // Try to load a full page of messages since we have none
            if let Err(e) = self.load_messages_from_api_with_count(CONFIG.max_messages()) {
                log::error!(
                    "Failed to load messages after single removal left page empty: {}",
                    e
                );
                // Continue with normal flow even if loading fails
            } else {
                // If we successfully started loading, view will be updated when messages arrive
                return None;
            }
        }

        // Update view and message details using common utilities
        if let Some(msg) = self.update_pagination_and_view() {
            return Some(msg);
        }

        self.remount_message_details_safe()
    }

    /// Handle bulk removal of messages from state - now simplified and focused
    fn handle_bulk_remove_messages_from_state(
        &mut self,
        message_ids: Vec<MessageIdentifier>,
    ) -> Option<Msg> {
        if message_ids.is_empty() {
            return None;
        }

        let _removed_count = self.remove_messages_from_pagination_state(&message_ids);
        let (target_page, remaining_message_count) =
            self.calculate_pagination_after_removal(_removed_count);

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

        // 4. Finalize pagination state (only if no auto-loading occurred)
        self.finalize_bulk_removal_pagination();

        // 5. Update view and message details
        if let Some(msg) = self.update_pagination_and_view() {
            return Some(msg);
        }

        self.remount_message_details_safe()
    }

    fn handle_queue_name_updated(&mut self, queue_name: String) -> Option<Msg> {
        self.queue_state.current_queue_name = Some(queue_name);
        None
    }

    /// Update pagination and view - common pattern across handlers
    fn update_pagination_and_view(&mut self) -> Option<Msg> {
        if let Err(e) = self.update_current_page_view() {
            return Some(Msg::Error(e));
        }
        None
    }

    /// Safely remount message details if messages exist - common pattern across handlers
    fn remount_message_details_safe(&mut self) -> Option<Msg> {
        let current_page_messages = self
            .queue_state
            .message_pagination
            .get_current_page_messages(CONFIG.max_messages());

        if !current_page_messages.is_empty() {
            if let Err(e) = self.remount_message_details(0) {
                return Some(Msg::Error(e));
            }
        }
        None
    }

    /// Calculate pagination state after message removal - common logic
    fn calculate_pagination_after_removal(&mut self, _removed_count: usize) -> (usize, usize) {
        let page_size = CONFIG.max_messages();
        let remaining_message_count = self
            .queue_state
            .message_pagination
            .all_loaded_messages
            .len();

        // Calculate the maximum valid page after removal
        let max_valid_page_after_removal = if remaining_message_count == 0 {
            0
        } else {
            (remaining_message_count - 1) / page_size as usize
        };

        // Adjust current page if it's now invalid
        let target_page = self
            .queue_state
            .message_pagination
            .current_page
            .min(max_valid_page_after_removal);

        (target_page, remaining_message_count)
    }

    /// Remove multiple messages from pagination state - extracted logic
    fn remove_messages_from_pagination_state(
        &mut self,
        message_ids: &[MessageIdentifier],
    ) -> usize {
        let page_size = CONFIG.max_messages();
        let mut removed_count = 0;

        // Remove each message from pagination state
        for message_id in message_ids {
            let removed = self
                .queue_state
                .message_pagination
                .remove_message_by_id_and_sequence(&message_id.id, message_id.sequence, page_size);

            if removed {
                removed_count += 1;
                log::debug!(
                    "Removed message {} (sequence {}) from local state",
                    message_id.id,
                    message_id.sequence
                );
            } else {
                log::warn!(
                    "Message with ID {} and sequence {} not found in local state",
                    message_id.id,
                    message_id.sequence
                );
            }
        }

        log::info!(
            "Bulk removed {} out of {} messages from local state",
            removed_count,
            message_ids.len()
        );

        // Remove the messages from bulk selection if they were selected
        self.queue_state.bulk_selection.remove_messages(message_ids);

        removed_count
    }

    /// Calculate and execute auto-loading if needed after bulk removal
    fn calculate_and_execute_auto_loading(
        &mut self,
        target_page: usize,
        remaining_message_count: usize,
    ) -> Result<bool, AppError> {
        let page_size = CONFIG.max_messages();

        // Check if we should auto-load to fill the target page
        let current_page_messages = if target_page < remaining_message_count / page_size as usize {
            // Page is full, no need to auto-load
            page_size as usize
        } else {
            // Last page, might be under-filled
            remaining_message_count % page_size as usize
        };

        let messages_short = if current_page_messages < page_size as usize
            && self.queue_state.message_pagination.has_next_page
        {
            page_size as usize - current_page_messages
        } else {
            0
        };

        log::info!(
            "After bulk removal: {} messages remaining, target page {}, current page has {} messages, need {} more",
            remaining_message_count,
            target_page,
            current_page_messages,
            messages_short
        );

        // Update current page to the valid target page
        self.queue_state.message_pagination.current_page = target_page;

        if messages_short > 0 {
            // Try to auto-load the missing messages to fill the current page using backfill method
            log::info!(
                "Auto-loading {} messages to fill page {}",
                messages_short,
                target_page
            );

            self.load_messages_for_backfill(messages_short as u32)?;
            return Ok(true); // Auto-loading initiated
        }

        Ok(false) // No auto-loading needed
    }

    /// Finalize pagination state after bulk removal (when no auto-loading occurred)
    fn finalize_bulk_removal_pagination(&mut self) {
        let page_size = CONFIG.max_messages();

        // After bulk removal (and potential auto-loading), ensure we're on a valid page
        let total_messages = self
            .queue_state
            .message_pagination
            .all_loaded_messages
            .len();
        let max_valid_page = if total_messages == 0 {
            0
        } else {
            (total_messages - 1) / page_size as usize
        };

        // Ensure current page is valid - if we're beyond the last page, go to the last valid page
        if self.queue_state.message_pagination.current_page > max_valid_page {
            log::info!(
                "Current page {} is beyond last valid page {}, adjusting to page {}",
                self.queue_state.message_pagination.current_page,
                max_valid_page,
                max_valid_page
            );
            self.queue_state.message_pagination.current_page = max_valid_page;
        }

        // Recalculate total pages based on current messages
        let new_total_pages = if total_messages == 0 {
            0
        } else {
            ((total_messages - 1) / page_size as usize) + 1
        };

        // Update total_pages_loaded to reflect actual loaded pages
        // Don't let it be less than current_page + 1 to maintain navigation capability
        self.queue_state.message_pagination.total_pages_loaded =
            new_total_pages.max(self.queue_state.message_pagination.current_page + 1);

        // Update pagination flags based on new state
        self.queue_state.message_pagination.update(page_size);

        log::info!(
            "Bulk removal completed: now on page {}, current page has {} messages",
            self.queue_state.message_pagination.current_page,
            self.queue_state
                .message_pagination
                .get_current_page_messages(page_size)
                .len()
        );
    }

    /// Add backfill messages to state without changing pagination structure
    fn add_backfill_messages_to_state(&mut self, messages: Vec<MessageModel>) {
        log::info!(
            "Adding {} backfill messages to current page",
            messages.len()
        );

        // Add messages directly to the end of all_loaded_messages (doesn't create new page)
        for message in messages {
            self.queue_state
                .message_pagination
                .all_loaded_messages
                .push(message);
        }

        // Update last_loaded_sequence
        if let Some(last_message) = self
            .queue_state
            .message_pagination
            .all_loaded_messages
            .last()
        {
            self.queue_state.message_pagination.last_loaded_sequence = Some(last_message.sequence);
        }
    }

    /// Ensure pagination consistency after backfill messages are added
    fn ensure_pagination_consistency_after_backfill(&mut self) {
        let page_size = CONFIG.max_messages();
        let total_messages = self
            .queue_state
            .message_pagination
            .all_loaded_messages
            .len();

        // Recalculate total pages to include newly loaded messages
        let new_total_pages = if total_messages == 0 {
            0
        } else {
            ((total_messages - 1) / page_size as usize) + 1
        };

        // Calculate the maximum valid page (0-indexed)
        let max_valid_page = if total_messages == 0 {
            0
        } else {
            (total_messages - 1) / page_size as usize
        };

        // Ensure current page is valid after backfill
        if self.queue_state.message_pagination.current_page > max_valid_page {
            log::info!(
                "After backfill: Current page {} is beyond max valid page {}, adjusting to page {}",
                self.queue_state.message_pagination.current_page,
                max_valid_page,
                max_valid_page
            );
            self.queue_state.message_pagination.current_page = max_valid_page;
        }

        // Update total_pages_loaded to reflect actual loaded pages
        // Ensure it's at least as many as we need for current page + 1
        self.queue_state.message_pagination.total_pages_loaded =
            new_total_pages.max(self.queue_state.message_pagination.current_page + 1);

        log::debug!(
            "After backfill: {} total messages, {} total pages, current page {} (max valid: {})",
            total_messages,
            self.queue_state.message_pagination.total_pages_loaded,
            self.queue_state.message_pagination.current_page,
            max_valid_page
        );
    }
}
