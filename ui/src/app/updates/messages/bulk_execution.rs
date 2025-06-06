use crate::app::model::Model;
use crate::components::common::{
    LoadingActivityMsg, MessageActivityMsg, Msg, PopupActivityMsg, QueueType,
};
use crate::config::CONFIG;
use crate::error::AppError;
use server::bulk_operations::MessageIdentifier;
use server::bulk_operations::{
    BulkOperationConfig, BulkOperationHandler, BulkOperationResult, ServiceBusOperationContext,
};
use server::consumer::Consumer;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tuirealm::terminal::TerminalAdapter;

// Constants for consistent queue display names
const DLQ_DISPLAY_NAME: &str = "Dead Letter Queue";
const MAIN_QUEUE_DISPLAY_NAME: &str = "Main Queue";

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Helper function to send messages to main thread with error logging
    fn send_message_or_log_error(tx: &Sender<Msg>, msg: Msg, operation: &str) {
        if let Err(e) = tx.send(msg) {
            log::error!("Failed to send {} message: {}", operation, e);
        }
    }

    /// Helper function to format queue direction display
    fn format_queue_direction(from_queue: &str, to_queue: &str) -> String {
        format!("From {} to {}", from_queue, to_queue)
    }
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
        Self::send_message_or_log_error(
            &tx_to_main,
            Msg::LoadingActivity(LoadingActivityMsg::Start(format!(
                "Bulk resending {} messages from dead letter queue...",
                message_ids.len()
            ))),
            "loading start",
        );

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

        let task = async move {
            log::debug!("Executing bulk resend operation in background task");

            // Create bulk operation configuration from app config
            let bulk_config = CONFIG.bulk();
            let bulk_operation_config = BulkOperationConfig::new(
                bulk_config.max_batch_size(),
                bulk_config.operation_timeout_secs(),
                bulk_config.order_warning_threshold(),
                bulk_config.batch_size_multiplier(),
            );

            let bulk_handler = BulkOperationHandler::new(bulk_operation_config);
            let operation_context =
                ServiceBusOperationContext::new(consumer, service_bus_client, main_queue_name);

            let result = bulk_handler
                .bulk_resend_from_dlq(operation_context, message_ids)
                .await;

            match result {
                Ok(operation_result) => {
                    log::info!(
                        "Bulk resend operation completed: {} successful, {} failed, {} not found",
                        operation_result.successful,
                        operation_result.failed,
                        operation_result.not_found
                    );
                    Self::handle_bulk_resend_success(&tx_to_main, operation_result);
                }
                Err(e) => {
                    log::error!("Failed to execute bulk resend operation: {}", e);
                    Self::handle_bulk_resend_error(
                        &tx_to_main,
                        &tx_to_main_err,
                        AppError::ServiceBus(e.to_string()),
                        DLQ_DISPLAY_NAME,
                        MAIN_QUEUE_DISPLAY_NAME,
                    );
                }
            }
        };

        taskpool.execute(task);

        None
    }

    /// Execute bulk resend-only from DLQ operation (without deleting from DLQ)
    pub fn handle_bulk_resend_from_dlq_only_execution(
        &mut self,
        message_ids: Vec<MessageIdentifier>,
    ) -> Option<Msg> {
        if message_ids.is_empty() {
            log::warn!("No messages provided for bulk resend-only operation");
            return None;
        }

        if let Err(error_msg) = self.validate_bulk_resend_request(&message_ids) {
            return Some(error_msg);
        }

        // For resend-only, we get message data from the current state (peeked messages)
        let messages_data = match self.extract_message_data_for_resend_only(&message_ids) {
            Ok(data) => data,
            Err(error_msg) => return Some(error_msg),
        };

        self.start_bulk_resend_only_operation(message_ids, messages_data)
    }

    /// Extract message data from current state for resend-only operation
    fn extract_message_data_for_resend_only(
        &self,
        message_ids: &[MessageIdentifier],
    ) -> Result<Vec<(MessageIdentifier, Vec<u8>)>, Msg> {
        let mut messages_data = Vec::new();

        // Get messages from pagination state (these are peeked messages)
        let all_messages = &self.queue_state.message_pagination.all_loaded_messages;

        for message_id in message_ids {
            // Find the message in our loaded state
            if let Some(message) = all_messages
                .iter()
                .find(|m| m.id == message_id.id && m.sequence == message_id.sequence)
            {
                // Extract the message body as bytes
                let body = match &message.body {
                    server::model::BodyData::ValidJson(json) => {
                        serde_json::to_vec(json).unwrap_or_default()
                    }
                    server::model::BodyData::RawString(s) => s.as_bytes().to_vec(),
                };
                messages_data.push((message_id.clone(), body));
                log::debug!("Extracted message data for {}", message_id.id);
            } else {
                log::error!("Message {} not found in current state", message_id.id);
                return Err(Msg::Error(AppError::State(format!(
                    "Message {} not found in current state for resend-only operation",
                    message_id.id
                ))));
            }
        }

        Ok(messages_data)
    }

    /// Starts the bulk resend-only operation in a background task
    fn start_bulk_resend_only_operation(
        &self,
        message_ids: Vec<MessageIdentifier>,
        messages_data: Vec<(MessageIdentifier, Vec<u8>)>,
    ) -> Option<Msg> {
        // Get the consumer before entering the async block
        let consumer = match self.get_consumer_for_bulk_operation() {
            Ok(consumer) => consumer,
            Err(error_msg) => return Some(error_msg),
        };
        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        // Show loading indicator
        Self::send_message_or_log_error(
            &tx_to_main,
            Msg::LoadingActivity(LoadingActivityMsg::Start(format!(
                "Bulk resending {} messages from dead letter queue (keeping in DLQ)...",
                message_ids.len()
            ))),
            "loading start",
        );

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
            "Starting bulk resend-only operation for {} messages from DLQ to queue {} (keeping in DLQ)",
            message_ids.len(),
            main_queue_name
        );

        let task = async move {
            log::debug!("Executing bulk resend-only operation in background task");

            // Create bulk operation configuration from app config
            let bulk_config = CONFIG.bulk();
            let bulk_operation_config = BulkOperationConfig::new(
                bulk_config.max_batch_size(),
                bulk_config.operation_timeout_secs(),
                bulk_config.order_warning_threshold(),
                bulk_config.batch_size_multiplier(),
            );

            let bulk_handler = BulkOperationHandler::new(bulk_operation_config);

            let operation_context =
                ServiceBusOperationContext::new(consumer, service_bus_client, main_queue_name);

            let result = bulk_handler
                .bulk_resend_from_dlq_only(operation_context, messages_data)
                .await;

            match result {
                Ok(operation_result) => {
                    log::info!(
                        "Bulk resend-only operation completed: {} successful, {} failed",
                        operation_result.successful,
                        operation_result.failed
                    );
                    Self::handle_bulk_resend_only_success(&tx_to_main, operation_result);
                }
                Err(e) => {
                    log::error!("Failed to execute bulk resend-only operation: {}", e);
                    Self::handle_bulk_resend_error(
                        &tx_to_main,
                        &tx_to_main_err,
                        AppError::ServiceBus(e.to_string()),
                        DLQ_DISPLAY_NAME,
                        MAIN_QUEUE_DISPLAY_NAME,
                    );
                }
            }
        };

        taskpool.execute(task);

        None
    }

    /// Handles successful bulk resend-only operation
    fn handle_bulk_resend_only_success(tx_to_main: &Sender<Msg>, result: BulkOperationResult) {
        log::info!(
            "Bulk resend-only operation completed successfully: {} successful, {} failed",
            result.successful,
            result.failed
        );

        // Stop loading indicator
        Self::send_message_or_log_error(
            tx_to_main,
            Msg::LoadingActivity(LoadingActivityMsg::Stop),
            "loading stop",
        );

        // For resend-only, we DON'T remove messages from local state since they stay in DLQ
        // Just reload the page to refresh the view
        Self::send_message_or_log_error(
            tx_to_main,
            Msg::MessageActivity(MessageActivityMsg::PageChanged),
            "page changed",
        );

        // Always show operation status to the user
        Self::show_bulk_resend_only_status(
            tx_to_main,
            &result,
            DLQ_DISPLAY_NAME,
            MAIN_QUEUE_DISPLAY_NAME,
        );
    }

    /// Show status of bulk resend-only operation to the user
    fn show_bulk_resend_only_status(
        tx_to_main: &Sender<Msg>,
        result: &BulkOperationResult,
        from_queue: &str,
        to_queue: &str,
    ) {
        let status_summary = Self::build_status_summary(result);
        let total = result.total_requested;

        if result.successful == total {
            let success_msg = format!(
                "✅ Bulk resend-only completed successfully!\n{}\n\n{}\n\n💡 Messages remain in {} for potential reprocessing.",
                Self::format_queue_direction(from_queue, to_queue),
                status_summary,
                from_queue
            );

            log::info!(
                "Bulk resend-only completed successfully: {}/{} messages sent to main queue",
                result.successful,
                result.total_requested
            );

            Self::send_message_or_log_error(
                tx_to_main,
                Msg::PopupActivity(PopupActivityMsg::ShowSuccess(success_msg)),
                "success",
            );
        } else if result.successful > 0 {
            let detailed_msg = format!(
                "✅ Bulk resend-only partially completed\n{}\n\n{}\n\n💡 Successfully sent messages remain in {} for potential reprocessing.\n{}",
                Self::format_queue_direction(from_queue, to_queue),
                status_summary,
                from_queue,
                if !result.error_details.is_empty() {
                    format!("\nError details:\n• {}", result.error_details.join("\n• "))
                } else {
                    "".to_string()
                }
            );

            log::info!("⚠️ Bulk resend-only partial success: {}", status_summary);

            Self::send_message_or_log_error(
                tx_to_main,
                Msg::PopupActivity(PopupActivityMsg::ShowSuccess(detailed_msg)),
                "partial success",
            );
        } else {
            let detailed_msg = format!(
                "❌ Bulk resend-only failed\n{}\n\n{}\n{}",
                Self::format_queue_direction(from_queue, to_queue),
                status_summary,
                if !result.error_details.is_empty() {
                    format!("\nError details:\n• {}", result.error_details.join("\n• "))
                } else {
                    "".to_string()
                }
            );

            log::error!("❌ Bulk resend-only operation failed: {}", status_summary);

            Self::send_message_or_log_error(
                tx_to_main,
                Msg::Error(AppError::ServiceBus(detailed_msg)),
                "bulk resend-only failure",
            );
        }
    }

    /// Handles successful bulk resend operation
    fn handle_bulk_resend_success(tx_to_main: &Sender<Msg>, result: BulkOperationResult) {
        log::info!(
            "Bulk resend operation completed successfully: {} successful, {} failed, {} not found",
            result.successful,
            result.failed,
            result.not_found
        );

        // Stop loading indicator
        Self::send_message_or_log_error(
            tx_to_main,
            Msg::LoadingActivity(LoadingActivityMsg::Stop),
            "loading stop",
        );

        // If we have successful operations, remove those specific messages from the local state
        if result.successful > 0 && !result.successful_message_ids.is_empty() {
            log::info!(
                "Removing {} successfully resent messages from local state",
                result.successful_message_ids.len()
            );

            // Use the exact messages that were successfully processed
            let messages_to_remove = result.successful_message_ids.clone();

            if let Err(e) = tx_to_main.send(Msg::MessageActivity(
                MessageActivityMsg::BulkRemoveMessagesFromState(messages_to_remove),
            )) {
                log::error!("Failed to send bulk remove messages message: {}", e);
                // Fall back to page reload if bulk removal fails
                Self::send_message_or_log_error(
                    tx_to_main,
                    Msg::MessageActivity(MessageActivityMsg::PageChanged),
                    "page changed fallback",
                );
            }
        } else {
            // No successful operations, just reload the page to be safe
            Self::send_message_or_log_error(
                tx_to_main,
                Msg::MessageActivity(MessageActivityMsg::PageChanged),
                "page changed",
            );
        }

        // Always show operation status to the user
        Self::show_bulk_operation_status(
            tx_to_main,
            &result,
            DLQ_DISPLAY_NAME,
            MAIN_QUEUE_DISPLAY_NAME,
        );
    }

    /// Show comprehensive status of bulk operation to the user
    fn show_bulk_operation_status(
        tx_to_main: &Sender<Msg>,
        result: &BulkOperationResult,
        from_queue: &str,
        to_queue: &str,
    ) {
        let status_summary = Self::build_status_summary(result);
        let total = result.total_requested;

        if result.successful == total {
            Self::show_complete_success(tx_to_main, result, from_queue, to_queue, &status_summary);
        } else if result.successful > 0 {
            Self::show_partial_success(tx_to_main, result, from_queue, to_queue, &status_summary);
        } else {
            Self::show_complete_failure(tx_to_main, result, from_queue, to_queue, &status_summary);
        }
    }

    /// Build status summary showing only non-zero counts
    fn build_status_summary(result: &BulkOperationResult) -> String {
        let total = result.total_requested;
        let mut status_lines = Vec::new();

        if result.successful > 0 {
            status_lines.push(format!("Processed: {}/{}", result.successful, total));
        }
        if result.failed > 0 {
            status_lines.push(format!("Failed: {}/{}", result.failed, total));
        }
        if result.not_found > 0 {
            status_lines.push(format!("Not Found: {}/{}", result.not_found, total));
        }

        status_lines.join("\n")
    }

    /// Handle complete success case
    fn show_complete_success(
        tx_to_main: &Sender<Msg>,
        result: &BulkOperationResult,
        from_queue: &str,
        to_queue: &str,
        status_summary: &str,
    ) {
        let success_msg = format!(
            "✅ Bulk resend completed successfully!\n{}\n\n{}",
            Self::format_queue_direction(from_queue, to_queue),
            status_summary
        );

        log::info!(
            "Bulk resend completed successfully: {}/{} messages processed",
            result.successful,
            result.total_requested
        );

        Self::send_message_or_log_error(
            tx_to_main,
            Msg::PopupActivity(PopupActivityMsg::ShowSuccess(success_msg)),
            "success",
        );
    }

    /// Handle partial success case
    fn show_partial_success(
        tx_to_main: &Sender<Msg>,
        result: &BulkOperationResult,
        from_queue: &str,
        to_queue: &str,
        status_summary: &str,
    ) {
        let detailed_msg = format!(
            "✅ Bulk resend partially completed\n{}\n\n{}\n{}",
            Self::format_queue_direction(from_queue, to_queue),
            status_summary,
            if !result.error_details.is_empty() {
                format!("\nError details:\n• {}", result.error_details.join("\n• "))
            } else {
                "".to_string()
            }
        );

        log::info!("⚠️ Bulk operation partial success: {}", status_summary);

        Self::send_message_or_log_error(
            tx_to_main,
            Msg::PopupActivity(PopupActivityMsg::ShowSuccess(detailed_msg)),
            "partial success",
        );
    }

    /// Handle complete failure case
    fn show_complete_failure(
        tx_to_main: &Sender<Msg>,
        result: &BulkOperationResult,
        from_queue: &str,
        to_queue: &str,
        status_summary: &str,
    ) {
        let detailed_msg = format!(
            "❌ Bulk resend failed\n{}\n\n{}\n{}",
            Self::format_queue_direction(from_queue, to_queue),
            status_summary,
            if !result.error_details.is_empty() {
                format!("\nError details:\n• {}", result.error_details.join("\n• "))
            } else {
                "".to_string()
            }
        );

        log::error!("❌ Bulk operation failed: {}", status_summary);

        let error_msg = AppError::State(detailed_msg);
        Self::send_message_or_log_error(tx_to_main, Msg::Error(error_msg), "error");
    }

    /// Handles bulk resend operation errors
    fn handle_bulk_resend_error(
        tx_to_main: &Sender<Msg>,
        tx_to_main_err: &Sender<Msg>,
        error: AppError,
        from_queue: &str,
        to_queue: &str,
    ) {
        log::error!("Error in bulk resend operation: {}", error);

        Self::send_message_or_log_error(
            tx_to_main,
            Msg::LoadingActivity(LoadingActivityMsg::Stop),
            "loading stop on error",
        );

        // Enhance error message with queue information
        let enhanced_error = AppError::State(format!(
            "❌ Bulk resend operation failed\n{}\n\nError: {}",
            Self::format_queue_direction(from_queue, to_queue),
            error
        ));

        let _ = tx_to_main_err.send(Msg::Error(enhanced_error));
    }
}
