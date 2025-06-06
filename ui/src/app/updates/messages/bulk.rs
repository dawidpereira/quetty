use crate::app::model::Model;
use crate::components::common::{MessageActivityMsg, Msg, PopupActivityMsg, QueueActivityMsg};
use server::bulk_operations::MessageIdentifier;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Handle toggling message selection by message identifier
    pub fn handle_toggle_message_selection(
        &mut self,
        message_id: MessageIdentifier,
    ) -> Option<Msg> {
        log::debug!("Toggling selection for message: {:?}", message_id);

        let is_in_bulk_mode = self.queue_state.bulk_selection.selection_mode;
        self.queue_state.bulk_selection.toggle_selection(message_id);

        // Enter bulk mode if this is the first selection
        if !is_in_bulk_mode && self.queue_state.bulk_selection.has_selections() {
            self.queue_state.bulk_selection.enter_selection_mode();
            log::debug!("Entered bulk selection mode");
        }
        // Exit bulk mode if no selections remain
        else if is_in_bulk_mode && !self.queue_state.bulk_selection.has_selections() {
            self.queue_state.bulk_selection.exit_selection_mode();
            log::debug!("Exited bulk selection mode - no selections remaining");
        }

        // Always remount to refresh the display
        if let Err(e) = self.remount_messages() {
            return Some(Msg::Error(e));
        }

        log::debug!(
            "Selection count: {}",
            self.queue_state.bulk_selection.selection_count()
        );
        None
    }

    /// Handle toggling message selection by index
    pub fn handle_toggle_message_selection_by_index(&mut self, index: usize) -> Option<Msg> {
        if let Some(messages) = &self.queue_state.messages {
            if let Some(message) = messages.get(index) {
                let message_id = MessageIdentifier::from_message(message);
                return self.handle_toggle_message_selection(message_id);
            }
        }

        log::warn!("Attempted to toggle selection for invalid index: {}", index);
        None
    }

    /// Handle selecting all messages on current page
    pub fn handle_select_all_current_page(&mut self) -> Option<Msg> {
        if let Some(messages) = &self.queue_state.messages {
            self.queue_state.bulk_selection.select_all(messages);

            // Always remount to show updated selections
            if let Err(e) = self.remount_messages() {
                return Some(Msg::Error(e));
            }
        }

        None
    }

    /// Handle selecting all loaded messages across all pages
    pub fn handle_select_all_loaded_messages(&mut self) -> Option<Msg> {
        let all_messages = &self.queue_state.message_pagination.all_loaded_messages;
        self.queue_state.bulk_selection.select_all(all_messages);

        // Remount messages to update visual state
        if let Err(e) = self.remount_messages() {
            return Some(Msg::Error(e));
        }

        None
    }

    /// Handle clearing all selections
    pub fn handle_clear_all_selections(&mut self) -> Option<Msg> {
        if self.queue_state.bulk_selection.selection_mode {
            // In bulk mode, clear selections
            self.queue_state.bulk_selection.clear_all();

            // Remount messages to update visual state
            if let Err(e) = self.remount_messages() {
                return Some(Msg::Error(e));
            }

            None
        } else {
            // Not in bulk mode, handle as normal ESC (go back)
            Some(Msg::QueueActivity(QueueActivityMsg::QueueUnselected))
        }
    }

    /// Handle entering bulk mode
    pub fn handle_enter_bulk_mode(&mut self) -> Option<Msg> {
        self.queue_state.bulk_selection.enter_selection_mode();

        // Remount messages to update visual state
        if let Err(e) = self.remount_messages() {
            return Some(Msg::Error(e));
        }

        None
    }

    /// Handle exiting bulk mode
    pub fn handle_exit_bulk_mode(&mut self) -> Option<Msg> {
        self.queue_state.bulk_selection.exit_selection_mode();

        // Remount messages to update visual state
        if let Err(e) = self.remount_messages() {
            return Some(Msg::Error(e));
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

    /// Handle bulk send to DLQ operation
    pub fn handle_bulk_send_to_dlq(&mut self, message_ids: Vec<MessageIdentifier>) -> Option<Msg> {
        if message_ids.is_empty() {
            return None;
        }

        let count = message_ids.len();
        let title = "Send to Dead Letter Queue".to_string();
        let message = format!(
            "You are about to send {} message{} to the dead letter queue.\n\n📤 Action: Messages will be moved to the DLQ\n🔄 Result: Messages can be processed or resent later",
            count,
            if count == 1 { "" } else { "s" }
        );

        Some(Msg::PopupActivity(PopupActivityMsg::ShowConfirmation {
            title,
            message,
            on_confirm: Box::new(Msg::MessageActivity(MessageActivityMsg::BulkSendToDLQ(
                message_ids,
            ))),
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

    /// Handle bulk delete for currently selected messages
    pub fn handle_bulk_delete_selected(&mut self) -> Option<Msg> {
        let selected_messages = self.queue_state.bulk_selection.get_selected_messages();
        if selected_messages.is_empty() {
            // Fall back to single message delete if no bulk selections
            // This will be handled by the existing delete logic
            return None;
        }

        self.handle_bulk_delete_messages(selected_messages)
    }

    /// Handle bulk send to DLQ for currently selected messages
    pub fn handle_bulk_send_selected_to_dlq(&mut self) -> Option<Msg> {
        let selected_messages = self.queue_state.bulk_selection.get_selected_messages();
        if selected_messages.is_empty() {
            // Fall back to single message DLQ if no bulk selections
            // This will be handled by the existing DLQ logic
            return None;
        }

        self.handle_bulk_send_to_dlq(selected_messages)
    }

    /// Handle bulk resend from DLQ for currently selected messages
    pub fn handle_bulk_resend_selected_from_dlq(&mut self, delete_from_dlq: bool) -> Option<Msg> {
        let selected_messages = self.queue_state.bulk_selection.get_selected_messages();
        if selected_messages.is_empty() {
            // Fall back to single message resend if no bulk selections
            // This will be handled by the existing resend logic
            return None;
        }

        self.handle_bulk_resend_from_dlq_messages(selected_messages, delete_from_dlq)
    }
}
