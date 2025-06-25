use crate::app::model::Model;
use crate::components::common::{ComponentId, MessageActivityMsg, Msg, PopupActivityMsg};
use server::bulk_operations::MessageIdentifier;
use server::model::MessageModel;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Get the currently highlighted message
    fn get_current_message(&self) -> Option<MessageModel> {
        // Get the current cursor position from the messages component
        if let Ok(tuirealm::State::One(tuirealm::StateValue::Usize(selected_index))) =
            self.app.state(&ComponentId::Messages)
        {
            if let Some(current_messages) = &self.queue_manager.queue_state.messages {
                if selected_index < current_messages.len() {
                    return Some(current_messages[selected_index].clone());
                }
            }
        }
        None
    }

    /// Handle bulk delete operation
    pub fn handle_bulk_delete_messages(
        &mut self,
        message_ids: Vec<MessageIdentifier>,
    ) -> Option<Msg> {
        if message_ids.is_empty() {
            return None;
        }

        let count = message_ids.len();
        let title = "Delete Messages".to_string();
        let message = format!(
            "You are about to delete {} message{} from the queue.\n\n🗑️  Action: Messages will be permanently removed\n⚠️   Warning: This action CANNOT be undone!",
            count,
            if count == 1 { "" } else { "s" }
        );

        Some(Msg::PopupActivity(PopupActivityMsg::ShowConfirmation {
            title,
            message,
            on_confirm: Box::new(Msg::MessageActivity(
                MessageActivityMsg::BulkDeleteMessages(message_ids),
            )),
        }))
    }

    /// Handle bulk send to DLQ operation with deletion (move to DLQ)
    pub fn handle_bulk_send_to_dlq_with_delete(
        &mut self,
        message_ids: Vec<MessageIdentifier>,
    ) -> Option<Msg> {
        if message_ids.is_empty() {
            return None;
        }

        let count = message_ids.len();
        let title = "Move to Dead Letter Queue".to_string();
        let message = format!(
            "You are about to move {} message{} to the dead letter queue.\n\n📤 Action: Messages will be moved to the DLQ\n🗑️  Result: Messages will be DELETED from the main queue",
            count,
            if count == 1 { "" } else { "s" }
        );

        Some(Msg::PopupActivity(PopupActivityMsg::ShowConfirmation {
            title,
            message,
            on_confirm: Box::new(Msg::MessageActivity(
                MessageActivityMsg::BulkSendToDLQWithDelete(message_ids),
            )),
        }))
    }

    /// Handle bulk resend from DLQ operation
    pub fn handle_bulk_resend_from_dlq_messages(
        &mut self,
        message_ids: Vec<MessageIdentifier>,
        delete_from_dlq: bool,
    ) -> Option<Msg> {
        if message_ids.is_empty() {
            return None;
        }

        let count = message_ids.len();

        let (title, base_message) = if delete_from_dlq {
            (
                "Resend and Delete from DLQ".to_string(),
                format!(
                    "You are about to resend {} message{} from the DLQ to the main queue.\n\n📤 Action: Messages will be sent to the main queue\n🗑️  Result: Messages will be DELETED from the DLQ",
                    count,
                    if count == 1 { "" } else { "s" }
                ),
            )
        } else {
            (
                "Resend Only (Keep in DLQ)".to_string(),
                format!(
                    "You are about to resend {} message{} from the DLQ to the main queue.\n\n📤 Action: Messages will be sent to the main queue\n📄 Result: Messages will REMAIN in the DLQ for future processing",
                    count,
                    if count == 1 { "" } else { "s" }
                ),
            )
        };

        let mut message = base_message;

        // Only show order warning for operations that delete from DLQ (which changes order)
        if delete_from_dlq {
            message.push_str("\n\n⚠️  WARNING: Message order may change in the main queue!");
        }

        Some(Msg::PopupActivity(PopupActivityMsg::ShowConfirmation {
            title,
            message,
            on_confirm: Box::new(Msg::MessageActivity(MessageActivityMsg::BulkResendFromDLQ(
                message_ids,
                delete_from_dlq,
            ))),
        }))
    }

    /// Handle bulk delete for currently selected messages or current message
    pub fn handle_bulk_delete_selected(&mut self) -> Option<Msg> {
        let selected_messages = self
            .queue_manager
            .queue_state
            .bulk_selection
            .get_selected_messages();
        if !selected_messages.is_empty() {
            // Use bulk selected messages
            return self.handle_bulk_delete_messages(selected_messages);
        }

        // No bulk selections - use current message as single-item bulk operation
        if let Some(current_message) = self.get_current_message() {
            let current_message_id = MessageIdentifier::from_message(&current_message);
            return self.handle_bulk_delete_messages(vec![current_message_id]);
        }

        // No current message available
        None
    }

    /// Handle bulk send to DLQ with delete for currently selected messages or current message
    pub fn handle_bulk_send_selected_to_dlq_with_delete(&mut self) -> Option<Msg> {
        let selected_messages = self
            .queue_manager
            .queue_state
            .bulk_selection
            .get_selected_messages();
        if !selected_messages.is_empty() {
            // Use bulk selected messages
            return self.handle_bulk_send_to_dlq_with_delete(selected_messages);
        }

        // No bulk selections - use current message as single-item bulk operation
        if let Some(current_message) = self.get_current_message() {
            let current_message_id = MessageIdentifier::from_message(&current_message);
            return self.handle_bulk_send_to_dlq_with_delete(vec![current_message_id]);
        }

        // No current message available
        None
    }

    /// Handle bulk resend from DLQ for currently selected messages or current message
    pub fn handle_bulk_resend_selected_from_dlq(&mut self, delete_from_dlq: bool) -> Option<Msg> {
        let selected_messages = self
            .queue_manager
            .queue_state
            .bulk_selection
            .get_selected_messages();
        if !selected_messages.is_empty() {
            // Use bulk selected messages
            return self.handle_bulk_resend_from_dlq_messages(selected_messages, delete_from_dlq);
        }

        // No bulk selections - use current message as single-item bulk operation
        if let Some(current_message) = self.get_current_message() {
            let current_message_id = MessageIdentifier::from_message(&current_message);
            return self
                .handle_bulk_resend_from_dlq_messages(vec![current_message_id], delete_from_dlq);
        }

        // No current message available
        None
    }
}
