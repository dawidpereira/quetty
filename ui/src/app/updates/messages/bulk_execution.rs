use crate::app::model::Model;
use crate::components::common::{LoadingActivityMsg, MessageActivityMsg, Msg, QueueType};
use crate::config::CONFIG;
use crate::error::AppError;
use server::bulk_operations::MessageIdentifier;
use server::bulk_operations::{BulkOperationConfig, BulkOperationHandler, BulkOperationResult};
use server::consumer::Consumer;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Execute bulk resend from DLQ operation
    pub fn handle_bulk_resend_from_dlq_execution(
        &mut self,
        message_ids: Vec<MessageIdentifier>,
    ) -> Option<Msg> {
        if message_ids.is_empty() {
            log::warn!("No messages provided for bulk resend operation");
            return None;
        }

        if let Err(error_msg) = self.validate_bulk_resend_request(&message_ids) {
            return Some(error_msg);
        }

        let consumer = match self.get_consumer_for_bulk_operation() {
            Ok(consumer) => consumer,
            Err(error_msg) => return Some(error_msg),
        };

        self.start_bulk_resend_operation(message_ids, consumer)
    }

    /// Validates that the bulk resend request is valid
    fn validate_bulk_resend_request(&self, message_ids: &[MessageIdentifier]) -> Result<(), Msg> {
        // Only allow resending from DLQ (not from main queue)
        if self.queue_state.current_queue_type != QueueType::DeadLetter {
            log::warn!("Cannot bulk resend messages from main queue - only from dead letter queue");
            return Err(Msg::Error(AppError::State(
                "Cannot bulk resend messages from main queue - only from dead letter queue"
                    .to_string(),
            )));
        }

        // Always log warning about potential message order changes in bulk operations
        log::warn!(
            "Bulk operation for {} messages may affect message order. Messages may not be processed in their original sequence.",
            message_ids.len()
        );

        log::info!(
            "Validated bulk resend request for {} messages",
            message_ids.len()
        );

        Ok(())
    }

    /// Gets the consumer for bulk operations
    fn get_consumer_for_bulk_operation(&self) -> Result<Arc<Mutex<Consumer>>, Msg> {
        match self.queue_state.consumer.clone() {
            Some(consumer) => Ok(consumer),
            None => {
                log::error!("No consumer available for bulk operation");
                Err(Msg::Error(AppError::State(
                    "No consumer available for bulk operation".to_string(),
                )))
            }
        }
    }

    /// Starts the bulk resend operation in a background task
    fn start_bulk_resend_operation(
        &self,
        message_ids: Vec<MessageIdentifier>,
        consumer: Arc<Mutex<Consumer>>,
    ) -> Option<Msg> {
        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        // Show loading indicator
        if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Start(format!(
            "Bulk resending {} messages from dead letter queue...",
            message_ids.len()
        )))) {
            log::error!("Failed to send loading start message: {}", e);
        }

        let tx_to_main_err = tx_to_main.clone();

        // Get the main queue name and service bus client for resending
        let main_queue_name = match self.get_main_queue_name_from_current_dlq() {
            Ok(name) => name,
            Err(e) => {
                log::error!("Failed to get main queue name: {}", e);
                return Some(Msg::Error(e));
            }
        };
        let service_bus_client = self.service_bus_client.clone();

        log::info!(
            "Starting bulk resend operation for {} messages from DLQ to queue {}",
            message_ids.len(),
            main_queue_name
        );

        // Clone message IDs for use in success handler (to know which messages to remove from UI)
        let original_message_ids = message_ids.clone();

        let task = async move {
            log::debug!("Executing bulk resend operation in background task");

            // Create bulk operation configuration from app config
            let bulk_config = CONFIG.bulk();
            let bulk_operation_config = BulkOperationConfig::new(
                bulk_config.max_batch_size(),
                bulk_config.operation_timeout_secs(),
                bulk_config.order_warning_threshold(),
            );

            let bulk_handler = BulkOperationHandler::new(bulk_operation_config);
            let result = bulk_handler
                .bulk_resend_from_dlq(consumer, message_ids, main_queue_name, service_bus_client)
                .await;

            match result {
                Ok(operation_result) => {
                    log::info!(
                        "Bulk resend operation completed: {} successful, {} failed, {} not found",
                        operation_result.successful,
                        operation_result.failed,
                        operation_result.not_found
                    );
                    Self::handle_bulk_resend_success(
                        &tx_to_main,
                        operation_result,
                        original_message_ids,
                    );
                }
                Err(e) => {
                    log::error!("Failed to execute bulk resend operation: {}", e);
                    Self::handle_bulk_resend_error(
                        &tx_to_main,
                        &tx_to_main_err,
                        AppError::ServiceBus(e.to_string()),
                    );
                }
            }
        };

        taskpool.execute(task);

        None
    }

    /// Handles successful bulk resend operation
    fn handle_bulk_resend_success(
        tx_to_main: &Sender<Msg>,
        result: BulkOperationResult,
        original_message_ids: Vec<MessageIdentifier>,
    ) {
        log::info!(
            "Bulk resend operation completed successfully: {} successful, {} failed, {} not found",
            result.successful,
            result.failed,
            result.not_found
        );

        // Stop loading indicator
        if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Stop)) {
            log::error!("Failed to send loading stop message: {}", e);
        }

        // If we have successful operations, remove those messages from the local state
        if result.successful > 0 {
            log::info!(
                "Removing {} successfully resent messages from local state",
                result.successful
            );

            // For now, we'll remove all the originally selected messages if any were successful
            // In a more sophisticated implementation, we could track exactly which messages were successful
            // But since the operation is atomic per message, if some succeeded, we assume they're the first N messages
            let messages_to_remove = original_message_ids
                .into_iter()
                .take(result.successful)
                .collect();

            if let Err(e) = tx_to_main.send(Msg::MessageActivity(
                MessageActivityMsg::BulkRemoveMessagesFromState(messages_to_remove),
            )) {
                log::error!("Failed to send bulk remove messages message: {}", e);
                // Fall back to page reload if bulk removal fails
                if let Err(e2) =
                    tx_to_main.send(Msg::MessageActivity(MessageActivityMsg::PageChanged))
                {
                    log::error!("Failed to send page changed fallback message: {}", e2);
                }
            }
        } else {
            // No successful operations, just reload the page to be safe
            if let Err(e) = tx_to_main.send(Msg::MessageActivity(MessageActivityMsg::PageChanged)) {
                log::error!("Failed to send page changed message: {}", e);
            }
        }

        // If there were any failures, we might want to show an error or info popup
        if result.failed > 0 || result.not_found > 0 {
            let error_msg = format!(
                "Bulk resend partially completed: {} successful, {} failed, {} not found. {}",
                result.successful,
                result.failed,
                result.not_found,
                if !result.error_details.is_empty() {
                    format!("Errors: {}", result.error_details.join(", "))
                } else {
                    "".to_string()
                }
            );
            log::warn!("{}", error_msg);

            // For partial failures, we'll just log but not show an error popup since some succeeded
            // The user will see the updated state when messages reload
        }
    }

    /// Handles bulk resend operation errors
    fn handle_bulk_resend_error(
        tx_to_main: &Sender<Msg>,
        tx_to_main_err: &Sender<Msg>,
        error: AppError,
    ) {
        log::error!("Error in bulk resend operation: {}", error);

        if let Err(err) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Stop)) {
            log::error!("Failed to send loading stop message: {}", err);
        }

        let _ = tx_to_main_err.send(Msg::Error(error));
    }
}
