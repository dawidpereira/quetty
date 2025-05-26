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
            PopupActivityMsg::ShowConfirmation {
                title,
                message,
                on_confirm,
            } => self.handle_show_confirmation(title, message, on_confirm),
            PopupActivityMsg::ConfirmationResult(confirmed) => {
                self.handle_confirmation_result(confirmed)
            }
        }
    }

    fn handle_show_error(&mut self, error: AppError) -> Option<Msg> {
        if let Err(e) = self.mount_error_popup(&error) {
            log::error!("Failed to mount error popup: {}", e);
            Some(Msg::Error(e))
        } else {
            None
        }
    }

    fn handle_close_error(&mut self) -> Option<Msg> {
        if let Err(e) = self.unmount_error_popup() {
            log::error!("Failed to unmount error popup: {}", e);
            Some(Msg::Error(e))
        } else {
            None
        }
    }

    fn handle_show_confirmation(
        &mut self,
        title: String,
        message: String,
        on_confirm: Box<Msg>,
    ) -> Option<Msg> {
        // Store the action to perform on confirmation
        self.pending_confirmation_action = Some(on_confirm);

        if let Err(e) = self.mount_confirmation_popup(&title, &message) {
            log::error!("Failed to mount confirmation popup: {}", e);
            Some(Msg::Error(e))
        } else {
            None
        }
    }

    fn handle_confirmation_result(&mut self, confirmed: bool) -> Option<Msg> {
        // Close the confirmation popup
        if let Err(e) = self.unmount_confirmation_popup() {
            log::error!("Failed to unmount confirmation popup: {}", e);
        }

        if confirmed {
            // Execute the stored action if user confirmed
            if let Some(action) = self.pending_confirmation_action.take() {
                Some(*action)
            } else {
                log::warn!("No pending confirmation action found");
                None
            }
        } else {
            // User cancelled, just clear the pending action
            self.pending_confirmation_action = None;
            None
        }
    }
}

