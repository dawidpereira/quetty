use crate::app::model::Model;
use crate::components::common::{MessageActivityMsg, Msg};
use tuirealm::terminal::TerminalAdapter;



pub mod async_operations;
pub mod bulk;
pub mod bulk_execution;
pub mod loading;
pub mod pagination;
pub mod updates;
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

    /// Handle composition operations
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
}
