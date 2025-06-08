use crate::app::model::Model;
use crate::components::common::{Msg, PopupActivityMsg};
use crate::config::CONFIG;
use crate::error::AppError;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Toggle selection of a message by its index
    pub fn handle_toggle_message_selection_by_index(&mut self, index: usize) -> Option<Msg> {
        use PopupActivityMsg;

        let current_messages = if let Some(ref messages) = self.queue_state.messages {
            messages.clone()
        } else {
            self.queue_state
                .message_pagination
                .get_current_page_messages(CONFIG.max_messages())
        };

        if index >= current_messages.len() {
            return Some(Msg::PopupActivity(PopupActivityMsg::ShowError(
                AppError::State("Invalid message index".to_string()),
            )));
        }

        let message = &current_messages[index];
        let message_id = server::bulk_operations::MessageIdentifier::from_message(message);

        log::debug!("Toggling selection for message: {:?}", message_id);

        let was_in_bulk_mode = self.queue_state.bulk_selection.selection_mode;
        let was_selected = self.queue_state.bulk_selection.toggle_selection(message_id);

        if was_selected {
            log::debug!("Selected message: {}", message.id);
        } else {
            log::debug!("Deselected message: {}", message.id);
        }

        if !was_in_bulk_mode && self.queue_state.bulk_selection.has_selections() {
            self.queue_state.bulk_selection.enter_selection_mode();
            log::debug!("Entered bulk selection mode");
        }
        // Exit bulk mode if no selections remain
        else if was_in_bulk_mode && !self.queue_state.bulk_selection.has_selections() {
            self.queue_state.bulk_selection.exit_selection_mode();
            log::debug!("Exited bulk selection mode - no selections remaining");
        }

        if let Err(e) = self.remount_messages_with_focus(true) {
            return Some(Msg::Error(e));
        }

        log::debug!(
            "Selection count: {}",
            self.queue_state.bulk_selection.selection_count()
        );

        None
    }

    /// Select all messages on the current page
    pub fn handle_select_all_current_page(&mut self) -> Option<Msg> {
        use PopupActivityMsg;

        // Get current page messages
        let current_messages = if let Some(ref messages) = self.queue_state.messages {
            messages.clone()
        } else {
            self.queue_state
                .message_pagination
                .get_current_page_messages(CONFIG.max_messages())
        };

        if current_messages.is_empty() {
            return Some(Msg::PopupActivity(PopupActivityMsg::ShowError(
                AppError::State("No messages to select".to_string()),
            )));
        }

        let message_count = current_messages.len();

        self.queue_state
            .bulk_selection
            .select_all(&current_messages);

        log::info!("Selected all {} messages on current page", message_count);

        if let Err(e) = self.remount_messages_with_focus(true) {
            return Some(Msg::Error(e));
        }

        None
    }

    /// Select all loaded messages across all pages
    pub fn handle_select_all_loaded_messages(&mut self) -> Option<Msg> {
        use crate::components::common::PopupActivityMsg;

        let all_messages = &self.queue_state.message_pagination.all_loaded_messages;
        let all_messages_count = all_messages.len();

        if all_messages_count == 0 {
            return Some(Msg::PopupActivity(PopupActivityMsg::ShowError(
                crate::error::AppError::State("No messages to select".to_string()),
            )));
        }

        self.queue_state.bulk_selection.select_all(all_messages);

        log::info!("Selected all {} loaded messages", all_messages_count);

        if let Err(e) = self.remount_messages_with_focus(true) {
            return Some(Msg::Error(e));
        }

        None
    }

    /// Clear all message selections
    pub fn handle_clear_all_selections(&mut self) -> Option<Msg> {
        let selection_count = self.queue_state.bulk_selection.selection_count();

        if selection_count == 0 {
            return None;
        }

        self.queue_state.bulk_selection.clear_all();
        log::info!("Cleared {} message selections", selection_count);

        if let Err(e) = self.remount_messages_with_focus(true) {
            return Some(Msg::Error(e));
        }

        None
    }
}

