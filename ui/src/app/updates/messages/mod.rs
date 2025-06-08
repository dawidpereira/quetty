use crate::app::model::{AppState, Model};
use crate::components::common::{
    ComponentId, LoadingActivityMsg, MessageActivityMsg, Msg, PopupActivityMsg,
};
use crate::config::CONFIG;
use crate::error::AppError;
use azservicebus::{ServiceBusClient, core::BasicRetryPolicy};
use server::bulk_operations::MessageIdentifier;
use server::consumer::Consumer;
use server::model::MessageModel;
use server::producer::ServiceBusClientProducerExt;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tuirealm::terminal::TerminalAdapter;

// Constants for bulk send configuration
const BULK_SEND_MIN_COUNT: usize = 1;
const BULK_SEND_MAX_COUNT: usize = 1000;

pub mod bulk;
pub mod bulk_execution;
pub mod loading;
pub mod pagination;
pub mod utils;
pub use pagination::MessagePaginationState;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn update_messages(&mut self, msg: MessageActivityMsg) -> Option<Msg> {
        match msg {
            // Message editing operations
            MessageActivityMsg::EditMessage(_)
            | MessageActivityMsg::CancelEditMessage
            | MessageActivityMsg::SendEditedMessage(_)
            | MessageActivityMsg::ReplaceEditedMessage(_, _) => self.handle_editing_operations(msg),

            // Bulk selection operations
            MessageActivityMsg::ToggleMessageSelectionByIndex(_)
            | MessageActivityMsg::SelectAllCurrentPage
            | MessageActivityMsg::SelectAllLoadedMessages
            | MessageActivityMsg::ClearAllSelections => self.handle_bulk_selection_operations(msg),

            // Bulk execution operations
            MessageActivityMsg::BulkDeleteSelected
            | MessageActivityMsg::BulkSendSelectedToDLQ
            | MessageActivityMsg::BulkResendSelectedFromDLQ(_)
            | MessageActivityMsg::BulkDeleteMessages(_)
            | MessageActivityMsg::BulkSendToDLQ(_)
            | MessageActivityMsg::BulkResendFromDLQ(_, _)
            | MessageActivityMsg::BulkRemoveMessagesFromState(_) => {
                self.handle_bulk_execution_operations(msg)
            }

            // Pagination operations
            MessageActivityMsg::NextPage
            | MessageActivityMsg::PreviousPage
            | MessageActivityMsg::PageChanged
            | MessageActivityMsg::PaginationStateUpdated { .. } => {
                self.handle_pagination_operations(msg)
            }

            // Message composition operations
            MessageActivityMsg::ComposeNewMessage
            | MessageActivityMsg::SetMessageRepeatCount
            | MessageActivityMsg::UpdateRepeatCount(_)
            | MessageActivityMsg::MessagesSentSuccessfully => {
                self.handle_composition_operations(msg)
            }

            // State management operations
            _ => self.handle_state_management_operations(msg),
        }
    }

    /// Handle message editing operations
    fn handle_editing_operations(&mut self, msg: MessageActivityMsg) -> Option<Msg> {
        match msg {
            MessageActivityMsg::EditMessage(index) => self.handle_edit_message(index),
            MessageActivityMsg::CancelEditMessage => self.handle_cancel_edit_message(),
            MessageActivityMsg::SendEditedMessage(content) => {
                self.handle_send_edited_message(content)
            }
            MessageActivityMsg::ReplaceEditedMessage(content, message_id) => {
                self.handle_replace_edited_message(content, message_id)
            }
            // Editing mode state tracking - handled in main update method
            MessageActivityMsg::EditingModeStarted | MessageActivityMsg::EditingModeStopped => None,
            _ => None,
        }
    }

    /// Handle bulk selection operations
    fn handle_bulk_selection_operations(&mut self, msg: MessageActivityMsg) -> Option<Msg> {
        match msg {
            MessageActivityMsg::ToggleMessageSelectionByIndex(index) => {
                self.handle_toggle_message_selection_by_index(index)
            }
            MessageActivityMsg::SelectAllCurrentPage => self.handle_select_all_current_page(),
            MessageActivityMsg::SelectAllLoadedMessages => self.handle_select_all_loaded_messages(),
            MessageActivityMsg::ClearAllSelections => self.handle_clear_all_selections(),
            _ => None,
        }
    }

    /// Handle bulk execution operations
    fn handle_bulk_execution_operations(&mut self, msg: MessageActivityMsg) -> Option<Msg> {
        match msg {
            MessageActivityMsg::BulkDeleteSelected => self.handle_bulk_delete_selected(),
            MessageActivityMsg::BulkSendSelectedToDLQ => self.handle_bulk_send_selected_to_dlq(),
            MessageActivityMsg::BulkResendSelectedFromDLQ(delete_from_dlq) => {
                self.handle_bulk_resend_selected_from_dlq(delete_from_dlq)
            }
            MessageActivityMsg::BulkDeleteMessages(message_ids) => {
                self.handle_bulk_delete_execution(message_ids)
            }
            MessageActivityMsg::BulkSendToDLQ(message_ids) => {
                self.handle_bulk_send_to_dlq_execution(message_ids)
            }
            MessageActivityMsg::BulkResendFromDLQ(message_ids, delete_from_dlq) => {
                if delete_from_dlq {
                    self.handle_bulk_resend_from_dlq_execution(message_ids)
                } else {
                    self.handle_bulk_resend_from_dlq_only_execution(message_ids)
                }
            }
            MessageActivityMsg::BulkRemoveMessagesFromState(message_ids) => {
                self.handle_bulk_remove_messages_from_state(message_ids)
            }
            _ => None,
        }
    }

    /// Handle pagination operations
    fn handle_pagination_operations(&mut self, msg: MessageActivityMsg) -> Option<Msg> {
        match msg {
            MessageActivityMsg::NextPage => self.handle_next_page_request(),
            MessageActivityMsg::PreviousPage => self.handle_previous_page_request(),
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
            _ => None,
        }
    }

    /// Handle message composition operations
    fn handle_composition_operations(&mut self, msg: MessageActivityMsg) -> Option<Msg> {
        match msg {
            MessageActivityMsg::ComposeNewMessage => self.handle_compose_new_message(),
            MessageActivityMsg::SetMessageRepeatCount => self.handle_set_message_repeat_count(),
            MessageActivityMsg::UpdateRepeatCount(count) => self.handle_update_repeat_count(count),
            MessageActivityMsg::MessagesSentSuccessfully => {
                self.handle_messages_sent_successfully()
            }
            _ => None,
        }
    }

    /// Handle state management operations
    fn handle_state_management_operations(&mut self, msg: MessageActivityMsg) -> Option<Msg> {
        match msg {
            MessageActivityMsg::MessagesLoaded(messages) => self.handle_messages_loaded(messages),
            MessageActivityMsg::ConsumerCreated(consumer) => self.handle_consumer_created(consumer),
            MessageActivityMsg::QueueNameUpdated(queue_name) => {
                self.handle_queue_name_updated(queue_name)
            }
            MessageActivityMsg::PreviewMessageDetails(index) => {
                self.handle_preview_message_details(index)
            }
            MessageActivityMsg::NewMessagesLoaded(new_messages) => {
                self.handle_new_messages_loaded(new_messages)
            }
            MessageActivityMsg::BackfillMessagesLoaded(backfill_messages) => {
                self.handle_backfill_messages_loaded(backfill_messages)
            }
            _ => None,
        }
    }

    // Message state management methods
    fn handle_edit_message(&mut self, index: usize) -> Option<Msg> {
        // Remount messages with unfocused state (white border)
        if let Err(e) = self.remount_messages_with_focus(false) {
            return Some(Msg::Error(e));
        }

        self.app_state = AppState::MessageDetails;

        // Set focus to message details component BEFORE remounting
        if let Err(e) = self.app.active(&ComponentId::MessageDetails) {
            log::error!("Failed to activate message details: {}", e);
        }

        // Remount message details - will automatically detect focus and use teal border
        if let Err(e) = self.remount_message_details(index) {
            return Some(Msg::Error(e));
        }

        Some(Msg::ForceRedraw)
    }

    fn handle_cancel_edit_message(&mut self) -> Option<Msg> {
        // Reset editing state
        self.is_editing_message = false;
        if let Err(e) = self.update_global_key_watcher_editing_state() {
            log::error!("Failed to update global key watcher: {}", e);
        }

        // Remount messages with focused state (teal border)
        if let Err(e) = self.remount_messages_with_focus(true) {
            return Some(Msg::Error(e));
        }

        self.app_state = AppState::MessagePicker;

        // Set focus to messages component BEFORE remounting message details
        if let Err(e) = self.app.active(&ComponentId::Messages) {
            log::error!("Failed to activate messages: {}", e);
        }

        // Remount message details - will automatically detect focus and use white border
        if let Err(e) = self.remount_message_details(0) {
            return Some(Msg::Error(e));
        }

        None
    }

    fn handle_messages_loaded(&mut self, messages: Vec<MessageModel>) -> Option<Msg> {
        self.queue_state.messages = Some(messages);

        // First remount messages with focus to ensure border color is correct
        if let Err(e) = self.remount_messages_with_focus(true) {
            return Some(Msg::Error(e));
        }

        self.app_state = AppState::MessagePicker;

        // Set focus to messages component BEFORE remounting message details
        if let Err(e) = self.app.active(&ComponentId::Messages) {
            log::error!("Failed to activate messages: {}", e);
        }

        // Remount message details - will automatically detect focus and use white border
        if let Err(e) = self.remount_message_details(0) {
            return Some(Msg::Error(e));
        }

        // Force redraw to ensure focus state is properly reflected
        self.redraw = true;

        Some(Msg::ForceRedraw)
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
            // Messages component should be active by default for initial loads
            // This will make message details use white border
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
    fn calculate_pagination_after_removal(&self, _removed_count: usize) -> (usize, usize) {
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

    // === Message Editing Handlers ===

    /// Helper function to send a message to a queue
    async fn send_message_to_queue(
        service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
        queue_name: String,
        content: String,
    ) -> Result<(), AppError> {
        let mut client = service_bus_client.lock().await;
        let mut producer = client
            .create_producer_for_queue(
                &queue_name,
                azservicebus::ServiceBusSenderOptions::default(),
            )
            .await
            .map_err(|e| AppError::ServiceBus(format!("Failed to create producer: {}", e)))?;

        let message = azservicebus::ServiceBusMessage::new(content.as_bytes().to_vec());

        producer
            .send_message(message)
            .await
            .map_err(|e| AppError::ServiceBus(format!("Failed to send message: {}", e)))?;

        producer
            .dispose()
            .await
            .map_err(|e| AppError::ServiceBus(format!("Failed to dispose producer: {}", e)))?;

        log::info!("Successfully sent message to queue: {}", queue_name);
        Ok(())
    }

    /// Helper function to send a message multiple times
    async fn send_message_multiple_times(
        service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
        queue_name: String,
        content: String,
        count: usize,
    ) -> Result<(), AppError> {
        log::info!("Sending message {} times to queue: {}", count, queue_name);

        // Send messages sequentially to avoid overwhelming the service
        for i in 1..=count {
            log::debug!("Sending message {}/{} to queue: {}", i, count, queue_name);
            Self::send_message_to_queue(
                service_bus_client.clone(),
                queue_name.clone(),
                content.clone(),
            )
            .await?;
        }

        log::info!(
            "Successfully sent {} messages to queue: {}",
            count,
            queue_name
        );
        Ok(())
    }

    /// Helper function to handle task completion messages
    fn send_completion_messages(
        tx_to_main: &Sender<Msg>,
        result: Result<(), AppError>,
        success_msg: &str,
    ) {
        // Stop loading screen
        let _ = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Stop));

        match result {
            Ok(()) => {
                let _ = tx_to_main.send(Msg::PopupActivity(PopupActivityMsg::ShowSuccess(
                    success_msg.to_string(),
                )));
                let _ =
                    tx_to_main.send(Msg::MessageActivity(MessageActivityMsg::CancelEditMessage));

                // Trigger auto-reload to show newly sent messages (if page not full)
                let _ = tx_to_main.send(Msg::MessageActivity(
                    MessageActivityMsg::MessagesSentSuccessfully,
                ));
            }
            Err(e) => {
                log::error!("Task failed: {}", e);
                let _ = tx_to_main.send(Msg::Error(e));
            }
        }
    }

    /// Helper function to validate queue selection
    fn get_current_queue(&self) -> Result<String, AppError> {
        self.queue_state
            .current_queue_name
            .clone()
            .ok_or_else(|| AppError::State("No queue selected".to_string()))
    }

    /// Handle sending edited message content as a new message
    fn handle_send_edited_message(&self, content: String) -> Option<Msg> {
        let queue_name = match self.get_current_queue() {
            Ok(name) => name,
            Err(e) => return Some(Msg::PopupActivity(PopupActivityMsg::ShowError(e))),
        };

        let repeat_count = self.queue_state.message_repeat_count;
        log::info!(
            "Sending edited message content to queue: {} ({} times)",
            queue_name,
            repeat_count
        );

        // Start loading screen with repeat count info
        let loading_message = if repeat_count == 1 {
            "Sending message...".to_string()
        } else {
            format!("Sending message {} times...", repeat_count)
        };

        let feedback_msg = Some(Msg::LoadingActivity(LoadingActivityMsg::Start(
            loading_message,
        )));

        // Start async task to send the message(s)
        let service_bus_client = self.service_bus_client.clone();
        let tx_to_main = self.tx_to_main.clone();
        let taskpool = &self.taskpool;

        let task = async move {
            let result = Self::send_message_multiple_times(
                service_bus_client,
                queue_name,
                content,
                repeat_count,
            )
            .await;

            let success_message = if repeat_count == 1 {
                "✅ Message sent successfully!".to_string()
            } else {
                format!("✅ {} messages sent successfully!", repeat_count)
            };

            Self::send_completion_messages(&tx_to_main, result, &success_message)
        };

        taskpool.execute(task);

        // Return immediate feedback, then stay in current state until task completes
        feedback_msg
    }

    /// Handle replacing original message with edited content (send new + delete original)
    fn handle_replace_edited_message(
        &self,
        content: String,
        message_id: MessageIdentifier,
    ) -> Option<Msg> {
        let queue_name = match self.get_current_queue() {
            Ok(name) => name,
            Err(e) => return Some(Msg::PopupActivity(PopupActivityMsg::ShowError(e))),
        };

        log::info!(
            "Replacing message {} with edited content in queue: {}",
            message_id.id,
            queue_name
        );

        // Start loading screen
        let feedback_msg = Some(Msg::LoadingActivity(LoadingActivityMsg::Start(
            "Replacing message...".to_string(),
        )));

        // Start async task to send new message and delete original
        let consumer = self.queue_state.consumer.clone();
        let service_bus_client = self.service_bus_client.clone();
        let tx_to_main = self.tx_to_main.clone();
        let taskpool = &self.taskpool;

        let task = async move {
            let result = async {
                // Step 1: Send new message with edited content
                Self::send_message_to_queue(service_bus_client, queue_name.clone(), content)
                    .await?;

                // Step 2: Delete original message
                match consumer {
                    Some(consumer) => Self::delete_message(consumer, &message_id).await?,
                    None => {
                        return Err(AppError::ServiceBus(
                            "No consumer available for message deletion".to_string(),
                        ));
                    }
                }

                log::info!(
                    "Successfully replaced message {} in queue: {}",
                    message_id.id,
                    queue_name
                );
                Ok::<(), AppError>(())
            }
            .await;

            // Remove the replaced message from local state if successful
            if result.is_ok() {
                let _ = tx_to_main.send(Msg::MessageActivity(
                    MessageActivityMsg::BulkRemoveMessagesFromState(vec![message_id]),
                ));
            }

            Self::send_completion_messages(&tx_to_main, result, "✅ Message replaced successfully!")
        };

        taskpool.execute(task);

        // Return immediate feedback, then stay in current state until task completes
        feedback_msg
    }

    /// Helper function to delete a message by completing it
    async fn delete_message(
        consumer: Arc<Mutex<Consumer>>,
        message_id: &MessageIdentifier,
    ) -> Result<(), AppError> {
        use crate::app::updates::messages::utils::find_target_message;

        let mut consumer_guard = consumer.lock().await;
        match find_target_message(&mut consumer_guard, &message_id.id, message_id.sequence).await {
            Ok(target_msg) => {
                consumer_guard
                    .complete_message(&target_msg)
                    .await
                    .map_err(|e| {
                        AppError::ServiceBus(format!("Failed to delete message: {}", e))
                    })?;
                log::info!("Successfully deleted message: {}", message_id.id);
                Ok(())
            }
            Err(e) => {
                log::warn!(
                    "Message {} not found (may have been already processed): {}",
                    message_id.id,
                    e
                );
                Ok(()) // Not an error - message may have been processed elsewhere
            }
        }
    }

    /// Handle opening empty message details in edit mode for composition
    fn handle_compose_new_message(&mut self) -> Option<Msg> {
        // Remount messages with unfocused state (white border)
        if let Err(e) = self.remount_messages_with_focus(false) {
            return Some(Msg::Error(e));
        }

        self.app_state = AppState::MessageDetails;

        // Set focus to message details component
        if let Err(e) = self.app.active(&ComponentId::MessageDetails) {
            log::error!("Failed to activate message details: {}", e);
        }

        // Remount message details in composition mode (empty, edit mode)
        if let Err(e) = self.remount_message_details_for_composition() {
            return Some(Msg::Error(e));
        }

        // Set editing state since composition mode starts in edit mode
        self.is_editing_message = true;
        if let Err(e) = self.update_global_key_watcher_editing_state() {
            log::error!("Failed to update global key watcher: {}", e);
        }

        Some(Msg::ForceRedraw)
    }

    /// Handle setting the repeat count for bulk message sending
    fn handle_set_message_repeat_count(&self) -> Option<Msg> {
        // Show number input popup
        Some(Msg::PopupActivity(PopupActivityMsg::ShowNumberInput {
            title: "Bulk Send Message".to_string(),
            message: "How many times should the message be sent?".to_string(),
            min_value: BULK_SEND_MIN_COUNT,
            max_value: BULK_SEND_MAX_COUNT,
        }))
    }

    /// Handle updating the repeat count (internal message)
    fn handle_update_repeat_count(&mut self, count: usize) -> Option<Msg> {
        self.queue_state.message_repeat_count = count;
        self.handle_compose_new_message()
    }

    /// Handle successful message sending by auto-reload only for empty queues
    fn handle_messages_sent_successfully(&mut self) -> Option<Msg> {
        let current_messages = self
            .queue_state
            .message_pagination
            .get_current_page_messages(CONFIG.max_messages());

        // Only auto-reload if the queue appears to be empty (0 messages shown)
        // This is the only case where auto-reload makes sense with Azure Service Bus
        if current_messages.is_empty() {
            log::info!("Queue appears empty, auto-reloading to show newly sent messages");

            // Reset entire pagination state to start fresh
            self.reset_pagination_state();

            // Force a fresh load from the beginning (like initial queue load)
            self.load_messages_from_beginning().err().map(Msg::Error)
        } else {
            log::info!(
                "Queue has {} messages, skipping auto-reload (newly sent messages may not be visible due to Azure Service Bus message ordering)",
                current_messages.len()
            );
            None
        }
    }

    /// Load messages from the beginning (fresh start), similar to initial queue load
    fn load_messages_from_beginning(&self) -> Result<(), AppError> {
        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        // Show loading indicator
        if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Start(
            "Loading messages...".to_string(),
        ))) {
            log::error!("Failed to send loading start message: {e}");
        }

        let consumer = self.queue_state.consumer.clone().ok_or_else(|| {
            log::error!("No consumer available");
            AppError::State("No consumer available".to_string())
        })?;

        let tx_to_main_err = tx_to_main.clone();
        taskpool.execute(async move {
            let result = Self::execute_fresh_message_load(tx_to_main.clone(), consumer).await;

            if let Err(e) = result {
                log::error!("Error loading messages from beginning: {e}");

                // Stop loading indicator even if there was an error
                if let Err(err) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Stop)) {
                    log::error!("Failed to send loading stop message: {err}");
                }

                // Send error message
                let _ = tx_to_main_err.send(Msg::Error(e));
            }
        });

        Ok(())
    }

    /// Execute fresh message loading from the beginning
    async fn execute_fresh_message_load(
        tx_to_main: Sender<Msg>,
        consumer: Arc<Mutex<Consumer>>,
    ) -> Result<(), AppError> {
        let mut consumer = consumer.lock().await;

        // Load from beginning (None sequence) with max messages
        let messages = consumer
            .peek_messages(CONFIG.max_messages(), None)
            .await
            .map_err(|e| {
                log::error!("Failed to peek messages from beginning: {e}");
                AppError::ServiceBus(e.to_string())
            })?;

        log::info!("Loaded {} messages from beginning", messages.len());

        // Stop loading indicator
        if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Stop)) {
            log::error!("Failed to send loading stop message: {e}");
        }

        // Send as new messages loaded to trigger proper pagination setup
        let activity_msg = if messages.is_empty() {
            MessageActivityMsg::MessagesLoaded(messages)
        } else {
            MessageActivityMsg::NewMessagesLoaded(messages)
        };

        tx_to_main
            .send(Msg::MessageActivity(activity_msg))
            .map_err(|e| {
                log::error!("Failed to send messages loaded message: {e}");
                AppError::Component(e.to_string())
            })?;

        Ok(())
    }
}
