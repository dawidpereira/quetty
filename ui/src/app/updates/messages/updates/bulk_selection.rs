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

        // Calculate the global index of the message across all loaded pages
        let page_size = config::get_current_page_size() as usize;
        let current_page = self.queue_state().message_pagination.current_page;
        let global_index = current_page * page_size + index;

        log::debug!(
            "Toggling selection for message: {message_id:?} (local_idx={index}, global_idx={global_index})"
        );

        let was_in_bulk_mode = self.queue_manager.queue_state.bulk_selection.selection_mode;
        let was_selected = self
            .queue_state_mut()
            .bulk_selection
            .toggle_selection(message_id, global_index);

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

        // Calculate global start index for the current page
        let page_size = config::get_current_page_size() as usize;
        let current_page = self.queue_state().message_pagination.current_page;
        let global_page_start_idx = current_page * page_size;

        self.queue_state_mut()
            .bulk_selection
            .select_all_with_offset(&current_messages, global_page_start_idx);

        // Also update the messages field to ensure UI consistency
        self.queue_state_mut().messages = Some(current_messages.clone());

        log::info!("Selected all {message_count} messages on current page");

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

        log::info!("Selected all {all_messages_count} loaded messages");

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
        log::info!("Cleared {selection_count} message selections");

        if let Err(e) = self.remount_messages_with_focus(true) {
            self.error_reporter
                .report_simple(e, "BulkSelection", "clear_all");
            return None;
        }

        None
    }
}
