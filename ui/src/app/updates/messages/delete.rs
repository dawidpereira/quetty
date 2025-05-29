use crate::app::model::Model;
use crate::components::common::Msg;
use server::consumer::Consumer;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn handle_delete_message(&mut self, index: usize) -> Option<Msg> {
        // ⚠️ WARNING: Message deletion permanently removes the message from the queue

        // Validate the request
        let message = match self.validate_delete_request(index) {
            Ok(msg) => msg,
            Err(error_msg) => return Some(error_msg),
        };

        // Get required resources
        let consumer = match self.get_consumer_for_delete() {
            Ok(consumer) => consumer,
            Err(error_msg) => return Some(error_msg),
        };

        // Start the delete operation
        self.start_delete_operation(message, consumer);
        None
    }

    /// Validates that the delete request is valid and returns the target message
    fn validate_delete_request(&self, index: usize) -> Result<server::model::MessageModel, Msg> {
        // Get the message at the specified index
        let message = if let Some(messages) = &self.queue_state.messages {
            if let Some(msg) = messages.get(index) {
                msg.clone()
            } else {
                log::error!("Message index {} out of bounds", index);
                return Err(Msg::Error(crate::error::AppError::State(
                    "Message index out of bounds".to_string(),
                )));
            }
        } else {
            log::error!("No messages available");
            return Err(Msg::Error(crate::error::AppError::State(
                "No messages available".to_string(),
            )));
        };

        // Can delete from both main queue and DLQ
        log::debug!(
            "Validated delete request for message {} from {:?} queue",
            message.id,
            self.queue_state.current_queue_type
        );

        Ok(message)
    }

    /// Gets the consumer for delete operations
    fn get_consumer_for_delete(&self) -> Result<Arc<Mutex<Consumer>>, Msg> {
        match self.queue_state.consumer.clone() {
            Some(consumer) => Ok(consumer),
            None => {
                log::error!("No consumer available");
                Err(Msg::Error(crate::error::AppError::State(
                    "No consumer available".to_string(),
                )))
            }
        }
    }

    /// Starts the delete operation in a background task
    fn start_delete_operation(
        &self,
        message: server::model::MessageModel,
        consumer: Arc<Mutex<Consumer>>,
    ) {
        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        // Show loading indicator
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Start(
                "Deleting message from queue...".to_string(),
            ),
        )) {
            log::error!("Failed to send loading start message: {}", e);
        }

        let tx_to_main_err = tx_to_main.clone();
        let message_id = message.id.clone();
        let message_sequence = message.sequence;

        taskpool.execute(async move {
            let result =
                Self::execute_delete_operation(consumer, message_id.clone(), message_sequence).await;

            match result {
                Ok(()) => {
                    Self::handle_delete_success(&tx_to_main, &message_id, message_sequence);
                }
                Err(e) => {
                    Self::handle_delete_error(&tx_to_main, &tx_to_main_err, e);
                }
            }
        });
    }

    /// Executes the delete operation: find and complete the target message
    async fn execute_delete_operation(
        consumer: Arc<Mutex<Consumer>>,
        message_id: String,
        message_sequence: i64,
    ) -> Result<(), crate::error::AppError> {
        let mut consumer = consumer.lock().await;

        // Find the target message using shared utility
        let target_msg = super::utils::find_target_message(&mut consumer, &message_id, message_sequence).await?;

        // Complete the message to remove it from the queue
        log::info!("Deleting message {} from queue", message_id);
        consumer.complete_message(&target_msg).await.map_err(|e| {
            log::error!("Failed to delete message: {}", e);
            crate::error::AppError::ServiceBus(e.to_string())
        })?;

        log::info!("Successfully deleted message {} from queue", message_id);

        Ok(())
    }

    /// Handles successful delete operation
    fn handle_delete_success(
        tx_to_main: &Sender<crate::components::common::Msg>,
        message_id: &str,
        message_sequence: i64,
    ) {
        log::info!(
            "Delete operation completed successfully for message {} (sequence {})",
            message_id,
            message_sequence
        );

        // Stop loading indicator
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Stop,
        )) {
            log::error!("Failed to send loading stop message: {}", e);
        }

        // Remove the message from local state since it's been deleted
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::MessageActivity(
            crate::components::common::MessageActivityMsg::RemoveMessageFromState(
                message_id.to_string(),
                message_sequence,
            ),
        )) {
            log::error!("Failed to send remove message from state message: {}", e);
        }
    }

    /// Handles delete operation errors
    fn handle_delete_error(
        tx_to_main: &Sender<crate::components::common::Msg>,
        tx_to_main_err: &Sender<crate::components::common::Msg>,
        error: crate::error::AppError,
    ) {
        log::error!("Error in delete operation: {}", error);

        // Stop loading indicator
        if let Err(err) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Stop,
        )) {
            log::error!("Failed to send loading stop message: {}", err);
        }

        // Send error message
        let _ = tx_to_main_err.send(crate::components::common::Msg::Error(error));
    }
} 