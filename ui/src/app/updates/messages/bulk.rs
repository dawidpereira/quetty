use crate::app::model::Model;
use crate::app::queue_state::MessageIdentifier;
use crate::components::common::{MessageActivityMsg, Msg, PopupActivityMsg, QueueActivityMsg};
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
        let title = "Bulk Delete Messages".to_string();
        let message = format!(
            "Are you sure you want to delete {} message{}?\nThis action will permanently remove the message{} and cannot be undone.",
            count,
            if count == 1 { "" } else { "s" },
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
        let title = "Bulk Send to Dead Letter Queue".to_string();
        let message = format!(
            "Are you sure you want to send {} message{} to the dead letter queue?",
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
    pub fn handle_bulk_resend_from_dlq(
        &mut self,
        message_ids: Vec<MessageIdentifier>,
    ) -> Option<Msg> {
        if message_ids.is_empty() {
            return None;
        }

        let count = message_ids.len();
        let title = "Bulk Resend from Dead Letter Queue".to_string();
        let message = format!(
            "Are you sure you want to resend {} message{} from the dead letter queue back to the main queue?",
            count,
            if count == 1 { "" } else { "s" }
        );

        Some(Msg::PopupActivity(PopupActivityMsg::ShowConfirmation {
            title,
            message,
            on_confirm: Box::new(Msg::MessageActivity(MessageActivityMsg::BulkResendFromDLQ(
                message_ids,
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
    pub fn handle_bulk_resend_selected_from_dlq(&mut self) -> Option<Msg> {
        let selected_messages = self.queue_state.bulk_selection.get_selected_messages();
        if selected_messages.is_empty() {
            // Fall back to single message resend if no bulk selections
            // This will be handled by the existing resend logic
            return None;
        }

        self.handle_bulk_resend_from_dlq(selected_messages)
    }
}
