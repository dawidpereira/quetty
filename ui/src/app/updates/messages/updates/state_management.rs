use std::sync::Arc;

use crate::app::model::{AppState, Model};
use crate::components::common::{ComponentId, Msg};
use crate::error::AppError;
use server::bulk_operations::MessageIdentifier;
use server::consumer::Consumer;
use server::model::MessageModel;
use tokio::sync::Mutex;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Handle initial messages loaded
    pub fn handle_messages_loaded(&mut self, messages: Vec<MessageModel>) -> Option<Msg> {
        self.queue_state.messages = Some(messages);
        if let Err(e) = self.remount_messages_with_focus(true) {
            return Some(Msg::Error(e));
        }

        self.app_state = AppState::MessagePicker;
        if let Err(e) = self.app.active(&ComponentId::Messages) {
            log::error!("Failed to activate messages: {}", e);
        }

        if let Err(e) = self.remount_message_details(0) {
            return Some(Msg::Error(e));
        }

        self.redraw = true;
        Some(Msg::ForceRedraw)
    }

    /// Handle consumer creation
    pub fn handle_consumer_created(&mut self, consumer: Consumer) -> Option<Msg> {
        self.queue_state.consumer = Some(Arc::new(Mutex::new(consumer)));
        if let Some(pending_queue) = &self.queue_state.pending_queue {
            self.queue_state.current_queue_name = Some(pending_queue.clone());
        }

        self.reset_pagination_state();
        if let Err(e) = self.load_messages() {
            return Some(Msg::Error(e));
        }
        None
    }

    /// Handle queue name update
    pub fn handle_queue_name_updated(&mut self, queue_name: String) -> Option<Msg> {
        self.queue_state.current_queue_name = Some(queue_name);
        None
    }

    /// Handle previewing message details
    pub fn handle_preview_message_details(&mut self, index: usize) -> Option<Msg> {
        if let Err(e) = self.remount_message_details(index) {
            return Some(Msg::Error(e));
        }
        None
    }

    /// Handle new messages loaded (pagination)
    pub fn handle_new_messages_loaded(&mut self, new_messages: Vec<MessageModel>) -> Option<Msg> {
        let is_initial_load = self
            .queue_state
            .message_pagination
            .all_loaded_messages
            .is_empty();

        self.queue_state
            .message_pagination
            .add_loaded_page(new_messages);

        if !is_initial_load {
            self.queue_state.message_pagination.advance_to_next_page();
        }

        if let Err(e) = self.update_current_page_view() {
            return Some(Msg::Error(e));
        }

        self.app_state = AppState::MessagePicker;

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
        message_ids: Vec<MessageIdentifier>,
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
            log::debug!(
                "Removing message ID: {}, sequence: {}",
                msg_id.id,
                msg_id.sequence
            );
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
        if let Err(e) = self.update_current_page_view() {
            return Some(Msg::Error(e));
        }
        self.remount_message_details_safe()
    }

    /// Safely remount message details
    pub fn remount_message_details_safe(&mut self) -> Option<Msg> {
        let current_messages = self
            .queue_state
            .message_pagination
            .get_current_page_messages(crate::config::CONFIG.max_messages());

        let index = if current_messages.is_empty() {
            0
        } else {
            std::cmp::min(
                self.queue_state.message_pagination.current_page,
                current_messages.len().saturating_sub(1),
            )
        };

        if let Err(e) = self.remount_message_details(index) {
            return Some(Msg::Error(e));
        }

        None
    }

    /// Calculate pagination after removal
    pub fn calculate_pagination_after_removal(&self, _removed_count: usize) -> (usize, usize) {
        let current_page = self.queue_state.message_pagination.current_page;
        let _total_pages = self.queue_state.message_pagination.total_pages_loaded;

        let current_messages = self
            .queue_state
            .message_pagination
            .get_current_page_messages(crate::config::CONFIG.max_messages());
        let remaining_on_current_page = current_messages.len();

        // If current page is empty and we have other pages, move to previous page
        let target_page = if remaining_on_current_page == 0 && current_page > 0 {
            current_page - 1
        } else {
            current_page
        };

        let total_remaining_messages = self
            .queue_state
            .message_pagination
            .all_loaded_messages
            .len();

        (target_page, total_remaining_messages)
    }

    /// Remove messages from pagination state and return count removed
    pub fn remove_messages_from_pagination_state(
        &mut self,
        message_ids: &[MessageIdentifier],
    ) -> usize {
        let initial_count = self
            .queue_state
            .message_pagination
            .all_loaded_messages
            .len();
        log::debug!("Initial message count: {}", initial_count);

        // Remove messages from all_loaded_messages
        self.queue_state
            .message_pagination
            .all_loaded_messages
            .retain(|msg| {
                let msg_identifier = MessageIdentifier {
                    id: msg.id.clone(),
                    sequence: msg.sequence,
                };
                let should_keep = !message_ids.contains(&msg_identifier);

                if !should_keep {
                    log::debug!("Removing message: {} (sequence: {})", msg.id, msg.sequence);
                } else {
                    log::trace!("Keeping message: {} (sequence: {})", msg.id, msg.sequence);
                }

                should_keep
            });

        if let Some(ref mut messages) = self.queue_state.messages {
            messages.retain(|msg| {
                let msg_identifier = MessageIdentifier {
                    id: msg.id.clone(),
                    sequence: msg.sequence,
                };
                !message_ids.contains(&msg_identifier)
            });
        }

        self.queue_state.bulk_selection.remove_messages(message_ids);

        let final_count = self
            .queue_state
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
        if target_page != self.queue_state.message_pagination.current_page {
            self.queue_state.message_pagination.current_page = target_page;
        }

        let page_size = crate::config::CONFIG.max_messages();
        let current_page = self.queue_state.message_pagination.current_page;
        let current_page_messages = self
            .queue_state
            .message_pagination
            .get_current_page_messages(page_size);
        let current_page_size = current_page_messages.len();
        let page_is_under_filled = current_page_size < page_size as usize;

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

            self.load_messages_for_backfill(messages_needed as u32)?;
            return Ok(true);
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

    /// Finalize bulk removal pagination
    pub fn finalize_bulk_removal_pagination(&mut self) {
        let messages_per_page = crate::config::CONFIG.max_messages();

        let total_messages = self
            .queue_state
            .message_pagination
            .all_loaded_messages
            .len();
        let new_total_pages = if total_messages == 0 {
            0
        } else {
            total_messages.div_ceil(messages_per_page as usize)
        };

        self.queue_state.message_pagination.total_pages_loaded = new_total_pages;

        // Ensure current page is within bounds
        if new_total_pages == 0 {
            self.queue_state.message_pagination.current_page = 0;
        } else if self.queue_state.message_pagination.current_page >= new_total_pages {
            self.queue_state.message_pagination.current_page = new_total_pages - 1;
        }

        // Update pagination controls
        self.queue_state.message_pagination.has_previous_page =
            self.queue_state.message_pagination.current_page > 0;
        self.queue_state.message_pagination.has_next_page =
            self.queue_state.message_pagination.current_page < new_total_pages.saturating_sub(1);

        log::debug!(
            "Finalized pagination: page {}/{}, current page: {}",
            self.queue_state.message_pagination.current_page + 1,
            new_total_pages,
            self.queue_state.message_pagination.current_page
        );
    }

    /// Add backfill messages to state
    pub fn add_backfill_messages_to_state(&mut self, messages: Vec<MessageModel>) {
        log::info!("Adding {} backfill messages to state", messages.len());

        self.queue_state
            .message_pagination
            .all_loaded_messages
            .extend(messages);

        log::debug!(
            "Added backfill messages, total messages now: {}",
            self.queue_state
                .message_pagination
                .all_loaded_messages
                .len()
        );
    }

    /// Ensure pagination consistency after backfill
    pub fn ensure_pagination_consistency_after_backfill(&mut self) {
        let total_messages = self
            .queue_state
            .message_pagination
            .all_loaded_messages
            .len();
        let messages_per_page = crate::config::CONFIG.max_messages();

        let new_total_pages = if total_messages == 0 {
            0
        } else {
            total_messages.div_ceil(messages_per_page as usize)
        };

        self.queue_state.message_pagination.total_pages_loaded = new_total_pages;

        if new_total_pages > 0
            && self.queue_state.message_pagination.current_page >= new_total_pages
        {
            self.queue_state.message_pagination.current_page = new_total_pages - 1;
        }

        // Update pagination state
        self.queue_state.message_pagination.has_previous_page =
            self.queue_state.message_pagination.current_page > 0;
        self.queue_state.message_pagination.has_next_page =
            self.queue_state.message_pagination.current_page < new_total_pages.saturating_sub(1);

        log::debug!(
            "Ensured pagination consistency: {}/{} pages, current page: {}",
            self.queue_state.message_pagination.current_page + 1,
            new_total_pages,
            self.queue_state.message_pagination.current_page
        );
    }
}

