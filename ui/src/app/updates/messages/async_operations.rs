use crate::app::model::Model;
use crate::components::common::{LoadingActivityMsg, MessageActivityMsg, Msg, PopupActivityMsg};
use crate::error::AppError;
use std::sync::mpsc::Sender;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Get current queue name or return error
    pub fn get_current_queue(&self) -> Result<String, AppError> {
        self.queue_manager
            .queue_state
            .current_queue_name
            .clone()
            .ok_or_else(|| AppError::State("No queue selected".to_string()))
    }
}

/// Send completion messages for async operations
pub fn send_completion_messages(
    tx_to_main: &Sender<Msg>,
    result: Result<(), AppError>,
    success_msg: &str,
) {
    // Always stop loading first
    if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Stop)) {
        log::error!("Failed to send loading stop message: {}", e);
    }

    match result {
        Ok(()) => {
            if let Err(e) = tx_to_main.send(Msg::PopupActivity(PopupActivityMsg::ShowSuccess(
                success_msg.to_string(),
            ))) {
                log::error!("Failed to send success message: {}", e);
            }

            if let Err(e) = tx_to_main.send(Msg::MessageActivity(
                MessageActivityMsg::MessagesSentSuccessfully,
            )) {
                log::error!("Failed to send messages sent successfully message: {}", e);
            }
        }
        Err(e) => {
            if let Err(err) = tx_to_main.send(Msg::PopupActivity(PopupActivityMsg::ShowError(e))) {
                log::error!("Failed to send error message: {}", err);
            }
        }
    }
}
