pub mod bulk;
pub mod delete;
pub mod dlq;
pub mod loading;
pub mod pagination;
pub mod utils;

// Re-export commonly used types
use crate::app::queue_state::MessageIdentifier;
pub use pagination::MessagePaginationState;

use crate::app::model::{AppState, Model};
use crate::components::common::{MessageActivityMsg, Msg};
use crate::config::CONFIG;
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
            MessageActivityMsg::SelectAllCurrentPage => self.handle_select_all_current_page(),
            MessageActivityMsg::SelectAllLoadedMessages => self.handle_select_all_loaded_messages(),
            MessageActivityMsg::ClearAllSelections => self.handle_clear_all_selections(),
            MessageActivityMsg::EnterBulkMode => self.handle_enter_bulk_mode(),
            MessageActivityMsg::ExitBulkMode => self.handle_exit_bulk_mode(),

            // Bulk operation handlers
            MessageActivityMsg::BulkDeleteMessages(message_ids) => {
                self.handle_bulk_delete_messages(message_ids)
            }
            MessageActivityMsg::BulkSendToDLQ(message_ids) => {
                self.handle_bulk_send_to_dlq(message_ids)
            }
            MessageActivityMsg::BulkResendFromDLQ(message_ids) => {
                self.handle_bulk_resend_from_dlq(message_ids)
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

    fn handle_remove_message_from_state(
        &mut self,
        message_id: String,
        message_sequence: i64,
    ) -> Option<Msg> {
        let page_size = CONFIG.max_messages() as u32;

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

        // Update the current page view with the new state
        if let Err(e) = self.update_current_page_view() {
            return Some(Msg::Error(e));
        }

        // Update message details if we have messages
        let current_page_messages = self
            .queue_state
            .message_pagination
            .get_current_page_messages(page_size);
        if !current_page_messages.is_empty() {
            if let Err(e) = self.remount_message_details(0) {
                return Some(Msg::Error(e));
            }
        }

        None
    }

    fn handle_queue_name_updated(&mut self, queue_name: String) -> Option<Msg> {
        self.queue_state.current_queue_name = Some(queue_name);
        None
    }

    // Note: handle_delete_message is implemented in the delete module
}
