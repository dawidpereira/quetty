use crate::app::model::Model;
use crate::app::queue_state::MessageIdentifier;
use crate::components::common::{Msg, PopupActivityMsg, MessageActivityMsg};
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Handle toggling selection for a specific message
    pub fn handle_toggle_message_selection(&mut self, message_id: MessageIdentifier) -> Option<Msg> {
        let was_selected = self.queue_state.bulk_selection.toggle_selection(message_id);
        
        // Enter bulk mode if we just selected a message and weren't in bulk mode
        if !was_selected && !self.queue_state.bulk_selection.selection_mode {
            self.queue_state.bulk_selection.enter_selection_mode();
        }
        
        // Exit bulk mode if no messages are selected
        if !self.queue_state.bulk_selection.has_selections() {
            self.queue_state.bulk_selection.selection_mode = false;
        }
        
        // Remount messages to update visual state
        if let Err(e) = self.remount_messages() {
            return Some(Msg::Error(e));
        }
        
        None
    }
    
    /// Handle selecting all messages on the current page
    pub fn handle_select_all_current_page(&mut self) -> Option<Msg> {
        if let Some(messages) = &self.queue_state.messages {
            self.queue_state.bulk_selection.select_all(messages);
            
            // Remount messages to update visual state
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
        self.queue_state.bulk_selection.clear_all();
        
        // Remount messages to update visual state
        if let Err(e) = self.remount_messages() {
            return Some(Msg::Error(e));
        }
        
        None
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
    pub fn handle_bulk_delete_messages(&mut self, message_ids: Vec<MessageIdentifier>) -> Option<Msg> {
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
                MessageActivityMsg::BulkDeleteMessages(message_ids)
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
            on_confirm: Box::new(Msg::MessageActivity(
                MessageActivityMsg::BulkSendToDLQ(message_ids)
            )),
        }))
    }
    
    /// Handle bulk resend from DLQ operation
    pub fn handle_bulk_resend_from_dlq(&mut self, message_ids: Vec<MessageIdentifier>) -> Option<Msg> {
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
            on_confirm: Box::new(Msg::MessageActivity(
                MessageActivityMsg::BulkResendFromDLQ(message_ids)
            )),
        }))
    }
    
    /// Get the currently selected message identifiers
    pub fn get_selected_message_ids(&self) -> Vec<MessageIdentifier> {
        self.queue_state.bulk_selection.get_selected_messages()
    }
    
    /// Check if we're currently in bulk selection mode
    pub fn is_in_bulk_mode(&self) -> bool {
        self.queue_state.bulk_selection.selection_mode
    }
    
    /// Get the number of currently selected messages
    pub fn get_selection_count(&self) -> usize {
        self.queue_state.bulk_selection.selection_count()
    }
    
    /// Check if a specific message is selected
    pub fn is_message_selected(&self, message_id: &MessageIdentifier) -> bool {
        self.queue_state.bulk_selection.is_selected(message_id)
    }
} 