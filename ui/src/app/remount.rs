use crate::app::model::Model;
use crate::components::common::ComponentId;
use crate::components::message_details::MessageDetails;
use crate::components::messages::{Messages, PaginationInfo};
use crate::components::namespace_picker::NamespacePicker;
use crate::components::queue_picker::QueuePicker;
use crate::error::{AppError, AppResult};
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn remount_message_details(&mut self, index: usize) -> AppResult<()> {
        let message = if let Some(messages) = &self.queue_state.messages {
            messages.get(index).cloned()
        } else {
            None
        };

        self.app
            .remount(
                ComponentId::MessageDetails,
                Box::new(MessageDetails::new(message)),
                Vec::default(),
            )
            .map_err(|e| AppError::Component(e.to_string()))?;

        Ok(())
    }

    pub fn remount_messages(&mut self) -> AppResult<()> {
        self.remount_messages_with_cursor_control(true)
    }

    pub fn remount_messages_with_cursor_control(&mut self, preserve_cursor: bool) -> AppResult<()> {
        log::debug!(
            "Remounting messages component, preserve_cursor: {}",
            preserve_cursor
        );

        // Preserve the current cursor position only if requested
        let current_position = if preserve_cursor && self.app.mounted(&ComponentId::Messages) {
            match self.app.state(&ComponentId::Messages) {
                Ok(tuirealm::State::One(tuirealm::StateValue::Usize(index))) => {
                    log::debug!("Preserving cursor position: {}", index);
                    Some(index)
                }
                _ => {
                    log::debug!("No cursor position to preserve");
                    None
                }
            }
        } else {
            if !preserve_cursor {
                log::debug!("Resetting cursor to position 0");
            } else {
                log::debug!("Messages component not mounted");
            }
            None
        };

        let pagination_info = self.create_pagination_info();

        // Get current selections for display
        let selected_messages = self.queue_state.bulk_selection.get_selected_messages();

        self.app
            .remount(
                ComponentId::Messages,
                Box::new(Messages::new_with_pagination_and_selections(
                    self.queue_state.messages.as_ref(),
                    Some(pagination_info),
                    selected_messages,
                )),
                Vec::default(),
            )
            .map_err(|e| AppError::Component(e.to_string()))?;

        // Restore cursor position using the Application's attr method (or reset to 0)
        let target_position = current_position.unwrap_or(0);
        if target_position > 0 || !preserve_cursor {
            log::debug!("Setting cursor position to: {}", target_position);
            match self.app.attr(
                &ComponentId::Messages,
                tuirealm::Attribute::Custom("cursor_position"),
                tuirealm::AttrValue::Number(target_position as isize),
            ) {
                Ok(_) => log::debug!("Successfully set cursor position attribute"),
                Err(e) => log::warn!("Failed to set cursor position attribute: {}", e),
            }
        }

        self.redraw = true;
        Ok(())
    }

    fn create_pagination_info(&self) -> PaginationInfo {
        let current_page_size = self
            .queue_state
            .messages
            .as_ref()
            .map(|msgs| msgs.len())
            .unwrap_or(0);

        PaginationInfo {
            current_page: self.queue_state.message_pagination.current_page,
            total_pages_loaded: self.queue_state.message_pagination.total_pages_loaded,
            total_messages_loaded: self
                .queue_state
                .message_pagination
                .all_loaded_messages
                .len(),
            current_page_size,
            has_next_page: self.queue_state.message_pagination.has_next_page,
            has_previous_page: self.queue_state.message_pagination.has_previous_page,
            queue_name: self.queue_state.current_queue_name.clone(),
            queue_type: self.queue_state.current_queue_type.clone(),
            bulk_mode: self.queue_state.bulk_selection.selection_mode,
            selected_count: self.queue_state.bulk_selection.selection_count(),
        }
    }

    pub fn remount_queue_picker(&mut self, queues: Option<Vec<String>>) -> AppResult<()> {
        self.app
            .remount(
                ComponentId::QueuePicker,
                Box::new(QueuePicker::new(queues)),
                Vec::default(),
            )
            .map_err(|e| AppError::Component(e.to_string()))?;

        Ok(())
    }

    pub fn remount_namespace_picker(&mut self, namespaces: Option<Vec<String>>) -> AppResult<()> {
        self.app
            .remount(
                ComponentId::NamespacePicker,
                Box::new(NamespacePicker::new(namespaces)),
                Vec::default(),
            )
            .map_err(|e| AppError::Component(e.to_string()))?;

        Ok(())
    }
}
