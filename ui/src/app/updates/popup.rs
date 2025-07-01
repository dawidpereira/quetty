use crate::app::model::Model;
use crate::components::common::{Msg, PopupActivityMsg};
use crate::error::AppError;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn update_popup(&mut self, msg: PopupActivityMsg) -> Option<Msg> {
        match msg {
            PopupActivityMsg::ShowError(error) => self.handle_show_error(error),
            PopupActivityMsg::CloseError => self.handle_close_error(),
            PopupActivityMsg::ShowWarning(message) => self.handle_show_warning(message),
            PopupActivityMsg::ShowSuccess(message) => self.handle_show_success(message),
            PopupActivityMsg::CloseSuccess => self.handle_close_success(),
            PopupActivityMsg::ShowConfirmation {
                title,
                message,
                on_confirm,
            } => self.handle_show_confirmation(title, message, on_confirm),
            PopupActivityMsg::ConfirmationResult(confirmed) => {
                self.handle_confirmation_result(confirmed)
            }
            PopupActivityMsg::ShowNumberInput {
                title,
                message,
                min_value,
                max_value,
            } => self.handle_show_number_input(title, message, min_value, max_value),
            PopupActivityMsg::NumberInputResult(value) => self.handle_number_input_result(value),
            PopupActivityMsg::ShowPageSizePopup => self.handle_show_page_size_popup(),
            PopupActivityMsg::PageSizeResult(size) => self.handle_page_size_result(size),
            PopupActivityMsg::ClosePageSize => self.handle_close_page_size(),
        }
    }

    fn handle_show_error(&mut self, error: AppError) -> Option<Msg> {
        if let Err(e) = self.mount_error_popup(&error) {
            self.error_reporter
                .report_mount_error("ErrorPopup", "mount", e);
            return None;
        }
        None
    }

    fn handle_close_error(&mut self) -> Option<Msg> {
        if let Err(e) = self.unmount_error_popup() {
            self.error_reporter
                .report_mount_error("ErrorPopup", "unmount", e);
            return None;
        }
        None
    }

    fn handle_show_warning(&mut self, message: String) -> Option<Msg> {
        // Create a properly formatted warning error using ErrorReporter's formatting
        let warning_error = self.error_reporter.create_warning_error(message);
        if let Err(e) = self.mount_error_popup(&warning_error) {
            self.error_reporter
                .report_mount_error("WarningPopup", "mount", e);
            return None;
        }
        None
    }

    fn handle_show_success(&mut self, message: String) -> Option<Msg> {
        if let Err(e) = self.mount_success_popup(&message) {
            self.error_reporter
                .report_mount_error("SuccessPopup", "mount", e);
            return None;
        }
        None
    }

    fn handle_close_success(&mut self) -> Option<Msg> {
        if let Err(e) = self.unmount_success_popup() {
            self.error_reporter
                .report_mount_error("SuccessPopup", "unmount", e);
            return None;
        }
        None
    }

    fn handle_show_confirmation(
        &mut self,
        title: String,
        message: String,
        on_confirm: Box<Msg>,
    ) -> Option<Msg> {
        // Store the action to perform on confirmation
        log::debug!("Storing confirmation action: {:?}", on_confirm);
        self.set_pending_confirmation_action(Some(on_confirm));

        if let Err(e) = self.mount_confirmation_popup(&title, &message) {
            self.error_reporter
                .report_mount_error("ConfirmationPopup", "mount", e);
            return None;
        }
        None
    }

    fn handle_confirmation_result(&mut self, confirmed: bool) -> Option<Msg> {
        log::debug!("Handling confirmation result: confirmed={}", confirmed);

        // Close the confirmation popup
        if let Err(e) = self.unmount_confirmation_popup() {
            self.error_reporter
                .report_mount_error("ConfirmationPopup", "unmount", e);
        }

        if confirmed {
            // Execute the stored action if user confirmed
            if let Some(action) = self.take_pending_confirmation_action() {
                log::debug!("Executing stored confirmation action: {:?}", action);
                Some(*action)
            } else {
                log::warn!("No pending confirmation action found");
                None
            }
        } else {
            // User cancelled, just clear the pending action
            log::debug!("User cancelled confirmation, clearing pending action");
            self.set_pending_confirmation_action(None);
            None
        }
    }

    fn handle_show_number_input(
        &mut self,
        title: String,
        message: String,
        min_value: usize,
        max_value: usize,
    ) -> Option<Msg> {
        // Use the new NumberInputPopup component
        if let Err(e) = self.mount_number_input_popup(title, message, min_value, max_value) {
            self.error_reporter
                .report_mount_error("NumberInputPopup", "mount", e);
            return None;
        }
        None
    }

    fn handle_number_input_result(&mut self, value: usize) -> Option<Msg> {
        log::debug!("Handling number input result: {}", value);

        // Close the number input popup
        if let Err(e) = self.unmount_number_input_popup() {
            self.error_reporter
                .report_mount_error("NumberInputPopup", "unmount", e);
        }

        Some(Msg::MessageActivity(
            crate::components::common::MessageActivityMsg::UpdateRepeatCount(value),
        ))
    }

    fn handle_show_page_size_popup(&mut self) -> Option<Msg> {
        if let Err(e) = self.mount_page_size_popup() {
            self.error_reporter
                .report_mount_error("PageSizePopup", "mount", e);
            return None;
        }
        None
    }

    fn handle_page_size_result(&mut self, size: usize) -> Option<Msg> {
        log::debug!("Handling page size result: {}", size);

        // Close the page size popup
        if let Err(e) = self.unmount_page_size_popup() {
            self.error_reporter
                .report_mount_error("PageSizePopup", "unmount", e);
        }

        // Update the page size in the application state
        crate::config::set_current_page_size(size as u32);
        log::info!("Page size changed to: {} messages per page", size);

        // Reset pagination state and reload messages
        self.queue_state_mut().message_pagination.reset();
        self.queue_state_mut().messages = None;

        // Immediately load the first page with the new page size
        let _ = self.load_messages_from_api_with_count(size as u32);
        None
    }

    fn handle_close_page_size(&mut self) -> Option<Msg> {
        if let Err(e) = self.unmount_page_size_popup() {
            self.error_reporter
                .report_mount_error("PageSizePopup", "unmount", e);
        }
        None
    }
}
