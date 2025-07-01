use crate::app::model::Model;
use crate::components::common::{Msg, PopupActivityMsg};
use crate::config;
use crate::error::AppError;
use server::bulk_operations::MessageIdentifier;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Toggle selection of a message by its index
    pub fn handle_toggle_message_selection_by_index(&mut self, index: usize) -> Option<Msg> {
        use PopupActivityMsg;

        // Always use pagination state as the source of truth for current page messages
        let current_messages = self
            .queue_state()
            .message_pagination
            .get_current_page_messages(config::get_current_page_size());

        if index >= current_messages.len() {
            return Some(Msg::PopupActivity(PopupActivityMsg::ShowError(
                AppError::State("Invalid message index".to_string()),
            )));
        }

        // Update the messages field to ensure UI consistency
        self.queue_state_mut().messages = Some(current_messages.clone());

        let message = &current_messages[index];
        let message_id = MessageIdentifier::from_message(message);

        log::debug!("Toggling selection for message: {:?}", message_id);

        let was_in_bulk_mode = self.queue_manager.queue_state.bulk_selection.selection_mode;
        let was_selected = self
            .queue_state_mut()
            .bulk_selection
            .toggle_selection(message_id, index);

        if was_selected {
            log::debug!("Selected message: {}", message.id);
        } else {
            log::debug!("Deselected message: {}", message.id);
        }

        if !was_in_bulk_mode
            && self
                .queue_manager
                .queue_state
                .bulk_selection
                .has_selections()
        {
            self.queue_state_mut().bulk_selection.enter_selection_mode();
            log::debug!("Entered bulk selection mode");
        }
        // Exit bulk mode if no selections remain
        else if was_in_bulk_mode
            && !self
                .queue_manager
                .queue_state
                .bulk_selection
                .has_selections()
        {
            self.queue_state_mut().bulk_selection.exit_selection_mode();
            log::debug!("Exited bulk selection mode - no selections remaining");
        }

        if let Err(e) = self.remount_messages_with_focus(true) {
            self.error_reporter
                .report_simple(e, "BulkSelection", "toggle_select");
            return None;
        }

        log::debug!(
            "Selection count: {}",
            self.queue_manager
                .queue_state
                .bulk_selection
                .selection_count()
        );

        None
    }

    /// Select all messages on the current page
    pub fn handle_select_all_current_page(&mut self) -> Option<Msg> {
        use PopupActivityMsg;

        // Always use pagination state as the source of truth for current page messages
        // This ensures we get the most up-to-date messages, including any backfill
        let current_messages = self
            .queue_state()
            .message_pagination
            .get_current_page_messages(config::get_current_page_size());

        if current_messages.is_empty() {
            return Some(Msg::PopupActivity(PopupActivityMsg::ShowError(
                AppError::State("No messages to select".to_string()),
            )));
        }

        let message_count = current_messages.len();

        self.queue_state_mut()
            .bulk_selection
            .select_all(&current_messages);

        // Also update the messages field to ensure UI consistency
        self.queue_state_mut().messages = Some(current_messages.clone());

        log::info!("Selected all {} messages on current page", message_count);

        if let Err(e) = self.remount_messages_with_focus(true) {
            self.error_reporter
                .report_simple(e, "BulkSelection", "select_all_page");
            return None;
        }

        None
    }

    /// Select all loaded messages across all pages
    pub fn handle_select_all_loaded_messages(&mut self) -> Option<Msg> {
        use crate::components::common::PopupActivityMsg;

        // Clone the messages to avoid borrowing issues
        let all_messages = self
            .queue_state()
            .message_pagination
            .all_loaded_messages
            .clone();
        let all_messages_count = all_messages.len();

        if all_messages_count == 0 {
            return Some(Msg::PopupActivity(PopupActivityMsg::ShowError(
                AppError::State("No messages to select".to_string()),
            )));
        }

        self.queue_state_mut()
            .bulk_selection
            .select_all(&all_messages);

        log::info!("Selected all {} loaded messages", all_messages_count);

        if let Err(e) = self.remount_messages_with_focus(true) {
            self.error_reporter
                .report_simple(e, "BulkSelection", "select_all_loaded");
            return None;
        }

        None
    }

    /// Clear all message selections
    pub fn handle_clear_all_selections(&mut self) -> Option<Msg> {
        let selection_count = self
            .queue_manager
            .queue_state
            .bulk_selection
            .selection_count();

        if selection_count == 0 {
            return None;
        }

        self.queue_state_mut().bulk_selection.clear_all();
        log::info!("Cleared {} message selections", selection_count);

        if let Err(e) = self.remount_messages_with_focus(true) {
            self.error_reporter
                .report_simple(e, "BulkSelection", "clear_all");
            return None;
        }

        None
    }
}
