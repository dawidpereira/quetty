use crate::app::model::Model;
use crate::components::common::ComponentId;
use crate::components::message_details::MessageDetails;
use crate::components::messages::{Messages, PaginationInfo};
use crate::components::namespace_picker::NamespacePicker;
use crate::components::queue_picker::QueuePicker;
use crate::components::state::ComponentStateMount;
use crate::error::{AppError, AppResult};
use tuirealm::terminal::TerminalAdapter;

use crate::app::managers::state_manager::AppState;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn remount_message_details(&mut self, index: usize) -> AppResult<()> {
        // Automatically determine focus based on which component is currently active
        let is_focused = match self.app.focus() {
            Some(focused_id) => *focused_id == ComponentId::MessageDetails,
            None => false, // Default to unfocused if no component is focused
        };

        let message = if let Some(messages) = &self.queue_manager.queue_state.messages {
            messages.get(index).cloned()
        } else {
            None
        };

        // Use ComponentState extension trait for single-call remounting
        self.app.remount_with_state(
            ComponentId::MessageDetails,
            MessageDetails::new_with_focus(message, is_focused),
            Vec::default(),
        )?;

        Ok(())
    }

    pub fn remount_message_details_for_composition(&mut self) -> AppResult<()> {
        // Always focused when in composition mode
        let is_focused = true;

        // No message - empty composition mode
        let message = None;

        // Get the current repeat count from queue state
        let repeat_count = self.queue_manager.queue_state.message_repeat_count;

        // Use ComponentState extension trait for single-call remounting
        self.app.remount_with_state(
            ComponentId::MessageDetails,
            MessageDetails::new_for_composition_with_repeat_count(
                message,
                is_focused,
                repeat_count,
            ),
            Vec::default(),
        )?;

        Ok(())
    }

    pub fn remount_messages_with_cursor_control(&mut self, preserve_cursor: bool) -> AppResult<()> {
        log::debug!("Remounting messages component, preserve_cursor: {preserve_cursor}");

        // Preserve the current cursor position only if requested
        let current_position = if preserve_cursor && self.app.mounted(&ComponentId::Messages) {
            match self.app.state(&ComponentId::Messages) {
                Ok(tuirealm::State::One(tuirealm::StateValue::Usize(index))) => {
                    log::debug!("Preserving cursor position: {index}");
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
        let selected_messages = self
            .queue_manager
            .queue_state
            .bulk_selection
            .get_selected_messages();

        self.app
            .remount(
                ComponentId::Messages,
                Box::new(Messages::new_with_pagination_and_selections(
                    self.queue_manager.queue_state.messages.as_ref(),
                    Some(pagination_info),
                    selected_messages,
                )),
                Vec::default(),
            )
            .map_err(|e| AppError::Component(e.to_string()))?;

        // Restore cursor position using the Application's attr method (or reset to 0)
        let target_position = current_position.unwrap_or(0);
        if target_position > 0 || !preserve_cursor {
            log::debug!("Setting cursor position to: {target_position}");
            match self.app.attr(
                &ComponentId::Messages,
                tuirealm::Attribute::Custom("cursor_position"),
                tuirealm::AttrValue::Number(target_position as isize),
            ) {
                Ok(_) => log::debug!("Successfully set cursor position attribute"),
                Err(e) => log::warn!("Failed to set cursor position attribute: {e}"),
            }
        }

        self.set_redraw(true);
        Ok(())
    }

    pub fn remount_messages(&mut self) -> AppResult<()> {
        self.remount_messages_with_focus(true)
    }

    pub fn remount_messages_with_focus(&mut self, is_focused: bool) -> AppResult<()> {
        self.remount_messages_with_cursor_and_focus_control(true, is_focused)
    }

    pub fn remount_messages_with_cursor_and_focus_control(
        &mut self,
        preserve_cursor: bool,
        is_focused: bool,
    ) -> AppResult<()> {
        log::debug!(
            "Remounting messages component, preserve_cursor: {preserve_cursor}, is_focused: {is_focused}"
        );

        // Preserve the current cursor position only if requested
        let current_position = if preserve_cursor && self.app.mounted(&ComponentId::Messages) {
            match self.app.state(&ComponentId::Messages) {
                Ok(tuirealm::State::One(tuirealm::StateValue::Usize(index))) => {
                    log::debug!("Preserving cursor position: {index}");
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
        let selected_messages = self
            .queue_manager
            .queue_state
            .bulk_selection
            .get_selected_messages();

        self.app
            .remount(
                ComponentId::Messages,
                Box::new(Messages::new_with_pagination_selections_and_focus(
                    self.queue_manager.queue_state.messages.as_ref(),
                    Some(pagination_info),
                    selected_messages,
                    is_focused,
                )),
                Vec::default(),
            )
            .map_err(|e| AppError::Component(e.to_string()))?;

        // Restore cursor position using the Application's attr method (or reset to 0)
        let target_position = current_position.unwrap_or(0);
        if target_position > 0 || !preserve_cursor {
            log::debug!("Setting cursor position to: {target_position}");
            match self.app.attr(
                &ComponentId::Messages,
                tuirealm::Attribute::Custom("cursor_position"),
                tuirealm::AttrValue::Number(target_position as isize),
            ) {
                Ok(_) => log::debug!("Successfully set cursor position attribute"),
                Err(e) => log::warn!("Failed to set cursor position attribute: {e}"),
            }
        }

        self.set_redraw(true);
        Ok(())
    }

    fn create_pagination_info(&self) -> PaginationInfo {
        // Get current page size directly from config (dynamic)
        let page_size = crate::config::get_current_page_size();
        let current_page_messages = self
            .queue_state()
            .message_pagination
            .get_current_page_messages(page_size);
        let current_page_size = current_page_messages.len();

        // Calculate correct pagination values based on actual loaded messages
        let total_messages_loaded = self
            .queue_state()
            .message_pagination
            .all_loaded_messages
            .len();

        let total_pages_loaded = if total_messages_loaded == 0 {
            0
        } else {
            total_messages_loaded.div_ceil(page_size as usize)
        };

        log::debug!(
            "create_pagination_info: total_messages_loaded={}, total_pages_loaded={}, current_page={}, page_size={}",
            total_messages_loaded,
            total_pages_loaded,
            self.queue_manager
                .queue_state
                .message_pagination
                .current_page,
            page_size
        );

        // Extract queue statistics from cache if available
        let (queue_total_messages, queue_stats_age_seconds) = {
            let mut current_name = self
                .queue_manager
                .queue_state
                .current_queue_name
                .clone()
                .unwrap_or_default();
            if current_name.ends_with("/$deadletterqueue") {
                current_name = current_name
                    .trim_end_matches("/$deadletterqueue")
                    .to_string();
            }
            let current_type = &self.queue_manager.queue_state.current_queue_type;

            if let Some(cache) = self
                .queue_state()
                .stats_manager
                .get_cached_stats(&current_name)
            {
                let count = cache.get_count_for_type(current_type);
                (Some(count), Some(cache.age_seconds()))
            } else {
                (None, None)
            }
        };

        PaginationInfo {
            current_page: self
                .queue_manager
                .queue_state
                .message_pagination
                .current_page,
            total_pages_loaded, // Use calculated value, not stored value
            total_messages_loaded,
            current_page_size,
            has_next_page: self
                .queue_manager
                .queue_state
                .message_pagination
                .has_next_page,
            has_previous_page: self
                .queue_manager
                .queue_state
                .message_pagination
                .has_previous_page,
            queue_name: self.queue_manager.queue_state.current_queue_name.clone(),
            queue_type: self.queue_manager.queue_state.current_queue_type.clone(),
            bulk_mode: self.queue_manager.queue_state.bulk_selection.selection_mode,
            selected_count: self
                .queue_manager
                .queue_state
                .bulk_selection
                .selection_count(),
            queue_total_messages,
            queue_stats_age_seconds,
        }
    }

    pub fn remount_queue_picker(&mut self, queues: Option<Vec<String>>) -> AppResult<()> {
        // If we're in discovery mode and namespace picker is mounted, unmount it
        // This prevents view errors when transitioning from namespace picker to queue picker
        if self.app.mounted(&ComponentId::NamespacePicker) {
            log::debug!("Unmounting namespace picker before mounting queue picker");
            if let Err(e) = self.app.umount(&ComponentId::NamespacePicker) {
                log::warn!("Failed to unmount namespace picker: {e}");
            }
        }

        self.app
            .remount(
                ComponentId::QueuePicker,
                Box::new(QueuePicker::new(queues)),
                Vec::default(),
            )
            .map_err(|e| AppError::Component(e.to_string()))?;

        // Activate the queue picker component to ensure it receives events
        self.app
            .active(&ComponentId::QueuePicker)
            .map_err(|e| AppError::Component(e.to_string()))?;

        // Set the app state to QueuePicker
        self.set_app_state(AppState::QueuePicker);

        Ok(())
    }

    pub fn remount_namespace_picker(&mut self, namespaces: Option<Vec<String>>) -> AppResult<()> {
        // Check if component is mounted, if not mount it first
        if self.app.mounted(&ComponentId::NamespacePicker) {
            self.app
                .remount(
                    ComponentId::NamespacePicker,
                    Box::new(NamespacePicker::new(namespaces)),
                    Vec::default(),
                )
                .map_err(|e| AppError::Component(e.to_string()))?;
        } else {
            // Component not mounted yet (e.g., after authentication), mount it
            self.app
                .mount(
                    ComponentId::NamespacePicker,
                    Box::new(NamespacePicker::new(namespaces)),
                    Vec::default(),
                )
                .map_err(|e| AppError::Component(e.to_string()))?;
        }

        // Activate the namespace picker component to ensure it receives events
        self.app
            .active(&ComponentId::NamespacePicker)
            .map_err(|e| AppError::Component(e.to_string()))?;

        // Set the app state to NamespacePicker
        self.set_app_state(AppState::NamespacePicker);

        Ok(())
    }
}
