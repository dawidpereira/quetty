use crate::app::model::Model;
use crate::components::common::{
    LoadingActivityMsg, MessageActivityMsg, Msg, PopupActivityMsg, QueueType,
};
use crate::config::CONFIG;
use crate::error::AppError;
use server::bulk_operations::{BulkOperationContext, BulkSendParams, MessageIdentifier};
use server::bulk_operations::{BulkOperationHandler, BulkOperationResult};
use server::consumer::Consumer;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tuirealm::terminal::TerminalAdapter;

// Constants for consistent queue display names
const DLQ_DISPLAY_NAME: &str = "DLQ";
const MAIN_QUEUE_DISPLAY_NAME: &str = "Main";

struct BulkSendDisplayParams<'a> {
    result: &'a BulkOperationResult,
    from_queue_display: &'a str,
    to_queue_display: &'a str,
    target_queue: &'a str,
    should_delete: bool,
}

impl<'a> BulkSendDisplayParams<'a> {
    fn new(
        result: &'a BulkOperationResult,
        from_queue_display: &'a str,
        to_queue_display: &'a str,
        target_queue: &'a str,
        should_delete: bool,
    ) -> Self {
        Self {
            result,
            from_queue_display,
            to_queue_display,
            target_queue,
            should_delete,
        }
    }
}

pub struct BulkSendOperationParams {
    pub consumer: Arc<Mutex<Consumer>>,
    pub target_queue: String,
    pub should_delete: bool,
    pub loading_message_template: String,
    pub from_queue_display: String,
    pub to_queue_display: String,
}

impl BulkSendOperationParams {
    pub fn new(
        consumer: Arc<Mutex<Consumer>>,
        target_queue: String,
        should_delete: bool,
        loading_message_template: &str,
        from_queue_display: &str,
        to_queue_display: &str,
    ) -> Self {
        Self {
            consumer,
            target_queue,
            should_delete,
            loading_message_template: loading_message_template.to_string(),
            from_queue_display: from_queue_display.to_string(),
            to_queue_display: to_queue_display.to_string(),
        }
    }
}

enum BulkSendData {
    MessageIds(Vec<MessageIdentifier>),
    MessageData(Vec<(MessageIdentifier, Vec<u8>)>),
}

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
        format!("{} → {}", from_queue, to_queue)
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

        // Get the current queue name for DLQ to Main operation
        let current_queue_name = match &self.queue_state.current_queue_name {
            Some(name) => name.clone(),
            None => {
                log::error!("No current queue name available");
                return Some(Msg::Error(AppError::State(
                    "No current queue name available".to_string(),
                )));
            }
        };

        let params = BulkSendOperationParams::new(
            consumer,
            current_queue_name, // target_queue
            true,               // should_delete = true for DLQ to Main
            "Bulk resending {} messages from DLQ to main queue...",
            DLQ_DISPLAY_NAME,
            MAIN_QUEUE_DISPLAY_NAME,
        );

        self.start_bulk_send_operation(message_ids, params)
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

    /// Generic method to start bulk send operation with either message IDs or pre-fetched data
    fn start_bulk_send_generic(
        &self,
        bulk_data: BulkSendData,
        operation_params: BulkSendOperationParams,
    ) -> Option<Msg> {
        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        // Extract and format loading message with the actual count before moving operation_params
        let message_count = match &bulk_data {
            BulkSendData::MessageIds(ids) => ids.len(),
            BulkSendData::MessageData(data) => data.len(),
        };
        let loading_message = operation_params.loading_message_template.replace("{}", &message_count.to_string());

        // Start loading indicator
        Self::send_message_or_log_error(
            &tx_to_main,
            Msg::LoadingActivity(LoadingActivityMsg::Start(loading_message)),
            "loading start",
        );

        // Spawn bulk send task
        let tx_to_main_err = tx_to_main.clone();
        let service_bus_client = self.service_bus_client.clone();

        let task = async move {
            log::debug!("Executing bulk send operation in background task");

            // Create batch operation configuration using server's config directly
            let batch_config = CONFIG.batch();
            let bulk_handler = BulkOperationHandler::new(batch_config.clone());

            // Create operation context and parameters
            let operation_context = BulkOperationContext::new(
                operation_params.consumer,
                service_bus_client,
                operation_params.target_queue.clone(),
            );

            // Create BulkSendParams based on the data type
            let params = match bulk_data {
                BulkSendData::MessageIds(message_ids) => {
                    log::info!(
                        "Starting bulk send operation for {} messages to queue {} (delete: {})",
                        message_ids.len(),
                        operation_params.target_queue,
                        operation_params.should_delete
                    );
                    BulkSendParams::with_retrieval(
                        operation_context.target_queue.clone(),
                        operation_params.should_delete,
                        message_ids,
                    )
                }
                BulkSendData::MessageData(messages_data) => {
                    log::info!(
                        "Starting bulk send with data operation for {} messages to queue {} (delete: {})",
                        messages_data.len(),
                        operation_params.target_queue,
                        operation_params.should_delete
                    );
                    BulkSendParams::with_message_data(
                        operation_context.target_queue.clone(),
                        operation_params.should_delete,
                        messages_data,
                    )
                }
            };

            let result = bulk_handler.bulk_send(operation_context, params).await;

            match result {
                Ok(operation_result) => {
                    log::info!(
                        "Bulk send operation completed: {} successful, {} failed, {} not found",
                        operation_result.successful,
                        operation_result.failed,
                        operation_result.not_found
                    );
                    let display_params = BulkSendDisplayParams::new(
                        &operation_result,
                        &operation_params.from_queue_display,
                        &operation_params.to_queue_display,
                        &operation_params.target_queue,
                        operation_params.should_delete,
                    );
                    Self::handle_bulk_send_success(&tx_to_main, display_params);
                }
                Err(e) => {
                    log::error!("Failed to execute bulk send operation: {}", e);
                    Self::handle_bulk_send_error(
                        &tx_to_main,
                        &tx_to_main_err,
                        AppError::ServiceBus(e.to_string()),
                        &operation_params.from_queue_display,
                        &operation_params.to_queue_display,
                    );
                }
            }
        };

        taskpool.execute(task);
        None
    }

    /// Method to start bulk send operation with message retrieval
    fn start_bulk_send_operation(
        &self,
        message_ids: Vec<MessageIdentifier>,
        params: BulkSendOperationParams,
    ) -> Option<Msg> {
        self.start_bulk_send_generic(BulkSendData::MessageIds(message_ids), params)
    }

    /// Method to start bulk send operation with pre-fetched message data
    fn start_bulk_send_with_data_operation(
        &self,
        messages_data: Vec<(MessageIdentifier, Vec<u8>)>,
        params: BulkSendOperationParams,
    ) -> Option<Msg> {
        self.start_bulk_send_generic(BulkSendData::MessageData(messages_data), params)
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

        // Get the main queue name for DLQ to Main operation
        let target_queue = match self.get_main_queue_name_from_current_dlq() {
            Ok(name) => name,
            Err(e) => {
                log::error!("Failed to get main queue name: {}", e);
                return Some(Msg::Error(e));
            }
        };

        let consumer = match self.get_consumer_for_bulk_operation() {
            Ok(consumer) => consumer,
            Err(error_msg) => return Some(error_msg),
        };

        self.start_bulk_send_with_data_operation(
            messages_data,
            BulkSendOperationParams::new(
                consumer,
                target_queue,
                false, // should_delete = false for resend-only
                "Bulk copying {} messages from DLQ to main queue (keeping in DLQ)...",
                DLQ_DISPLAY_NAME,
                MAIN_QUEUE_DISPLAY_NAME,
            ),
        )
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

    /// Success handler for all bulk send operations
    fn handle_bulk_send_success(tx_to_main: &Sender<Msg>, display_params: BulkSendDisplayParams) {
        log::info!(
            "Bulk send operation completed successfully: {} successful, {} failed, {} not found",
            display_params.result.successful,
            display_params.result.failed,
            display_params.result.not_found
        );

        // Stop loading indicator
        Self::send_message_or_log_error(
            tx_to_main,
            Msg::LoadingActivity(LoadingActivityMsg::Stop),
            "loading stop",
        );

        // Handle state updates based on operation type
        if display_params.should_delete && !display_params.result.successful_message_ids.is_empty()
        {
            // Remove successfully processed messages from local state
            Self::send_message_or_log_error(
                tx_to_main,
                Msg::MessageActivity(MessageActivityMsg::BulkRemoveMessagesFromState(
                    display_params.result.successful_message_ids.clone(),
                )),
                "bulk remove from state",
            );
        } else if !display_params.should_delete {
            // For operations that don't delete, just reload the page to refresh the view
            Self::send_message_or_log_error(
                tx_to_main,
                Msg::MessageActivity(MessageActivityMsg::PageChanged),
                "page changed",
            );
        }

        // Always show operation status to the user
        Self::show_bulk_send_status(tx_to_main, display_params);
    }

    /// Status display for all bulk send operations
    fn show_bulk_send_status(tx_to_main: &Sender<Msg>, params: BulkSendDisplayParams) {
        let direction =
            Self::format_queue_direction(params.from_queue_display, params.to_queue_display);
        let operation_type = if params.should_delete {
            "moved"
        } else {
            "copied"
        };

        let title = format!(
            "Bulk Send Complete ({})",
            if params.should_delete {
                "Moved"
            } else {
                "Copied"
            }
        );

        // Use target queue to determine destination description (same logic as QueueOperationType)
        let destination_desc = if params.target_queue.ends_with("/$deadletterqueue") {
            "to dead letter queue"
        } else {
            "to main queue"
        };

        let status_message = if params.result.is_complete_success() {
            format!(
                "{}\n\n✅ Successfully {} {} message{} {}\n📍 Direction: {}",
                title,
                operation_type,
                params.result.successful,
                if params.result.successful == 1 {
                    ""
                } else {
                    "s"
                },
                destination_desc,
                direction
            )
        } else {
            format!(
                "{}\n\n⚠️  Bulk send completed with mixed results\n📍 Direction: {}\n✅ Successful: {}\n❌ Failed: {}\n❓ Not found: {}",
                title,
                direction,
                params.result.successful,
                params.result.failed,
                params.result.not_found
            )
        };

        Self::send_message_or_log_error(
            tx_to_main,
            Msg::PopupActivity(PopupActivityMsg::ShowSuccess(status_message)),
            "bulk send status",
        );
    }

    /// Error handler for all bulk send operations
    fn handle_bulk_send_error(
        tx_to_main: &Sender<Msg>,
        tx_to_main_err: &Sender<Msg>,
        error: AppError,
        from_queue: &str,
        to_queue: &str,
    ) {
        let direction = Self::format_queue_direction(from_queue, to_queue);
        log::error!("Error in bulk send operation ({}): {}", direction, error);

        // Stop loading indicator
        Self::send_message_or_log_error(
            tx_to_main,
            Msg::LoadingActivity(LoadingActivityMsg::Stop),
            "loading stop",
        );

        // Send error message
        let _ = tx_to_main_err.send(Msg::Error(error));
    }

    /// Execute bulk send to DLQ operation
    pub fn handle_bulk_send_to_dlq_execution(
        &mut self,
        message_ids: Vec<MessageIdentifier>,
    ) -> Option<Msg> {
        if message_ids.is_empty() {
            log::warn!("No messages provided for bulk send to DLQ operation");
            return None;
        }

        if let Err(error_msg) = self.validate_bulk_send_to_dlq_request(&message_ids) {
            return Some(error_msg);
        }

        let consumer = match self.get_consumer_for_bulk_operation() {
            Ok(consumer) => consumer,
            Err(error_msg) => return Some(error_msg),
        };

        // Get the current queue name for Main to DLQ operation
        let current_queue_name = match &self.queue_state.current_queue_name {
            Some(name) => name.clone(),
            None => {
                log::error!("No current queue name available");
                return Some(Msg::Error(AppError::State(
                    "No current queue name available".to_string(),
                )));
            }
        };

        // For DLQ operations, target queue is the DLQ queue name
        let target_queue = format!("{}/$deadletterqueue", current_queue_name);

        let params = BulkSendOperationParams::new(
            consumer,
            target_queue,
            true, // should_delete = true for Main to DLQ
            "Bulk sending {} messages to dead letter queue...",
            MAIN_QUEUE_DISPLAY_NAME,
            DLQ_DISPLAY_NAME,
        );

        self.start_bulk_send_operation(message_ids, params)
    }

    /// Validates that the bulk send to DLQ request is valid
    fn validate_bulk_send_to_dlq_request(
        &self,
        message_ids: &[MessageIdentifier],
    ) -> Result<(), Msg> {
        // Only allow sending to DLQ from main queue (not from DLQ itself)
        if self.queue_state.current_queue_type != QueueType::Main {
            log::warn!(
                "Cannot bulk send messages to DLQ from dead letter queue - only from main queue"
            );
            return Err(Msg::Error(AppError::State(
                "Cannot bulk send messages to DLQ from dead letter queue - only from main queue"
                    .to_string(),
            )));
        }

        // Always log warning about potential message order changes in bulk operations
        log::warn!(
            "Bulk operation for {} messages may affect message order. Messages may not be processed in their original sequence.",
            message_ids.len()
        );

        log::info!(
            "Validated bulk send to DLQ request for {} messages",
            message_ids.len()
        );

        Ok(())
    }

    /// Execute bulk delete operation - works for both main queue and DLQ
    pub fn handle_bulk_delete_execution(
        &mut self,
        message_ids: Vec<MessageIdentifier>,
    ) -> Option<Msg> {
        if message_ids.is_empty() {
            log::warn!("No messages provided for bulk delete operation");
            return None;
        }

        if let Err(error_msg) = self.validate_bulk_delete_request(&message_ids) {
            return Some(error_msg);
        }

        let consumer = match self.get_consumer_for_bulk_operation() {
            Ok(consumer) => consumer,
            Err(error_msg) => return Some(error_msg),
        };

        // Start the bulk delete operation
        self.start_bulk_delete_operation(message_ids, consumer)
    }

    /// Validates that the bulk delete request is valid
    fn validate_bulk_delete_request(&self, message_ids: &[MessageIdentifier]) -> Result<(), Msg> {
        // Can delete from both main queue and DLQ - no restrictions
        log::info!(
            "Validated bulk delete request for {} messages from {:?} queue",
            message_ids.len(),
            self.queue_state.current_queue_type
        );
        Ok(())
    }

    /// Starts the bulk delete operation in a background task
    fn start_bulk_delete_operation(
        &self,
        message_ids: Vec<MessageIdentifier>,
        consumer: Arc<Mutex<Consumer>>,
    ) -> Option<Msg> {
        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        // Start loading indicator
        let loading_message = format!("Bulk deleting {} messages...", message_ids.len());
        Self::send_message_or_log_error(
            &tx_to_main,
            Msg::LoadingActivity(LoadingActivityMsg::Start(loading_message)),
            "loading start",
        );

        // Spawn bulk delete task
        let tx_to_main_err = tx_to_main.clone();
        let queue_type = self.queue_state.current_queue_type.clone();

        let task = async move {
            log::debug!("Executing bulk delete operation in background task");

            let result = Self::execute_bulk_delete_operation(consumer, message_ids.clone()).await;

            match result {
                Ok(()) => {
                    Self::handle_bulk_delete_success(&tx_to_main, &message_ids, queue_type);
                }
                Err(e) => {
                    Self::handle_bulk_delete_error(&tx_to_main, &tx_to_main_err, e, queue_type);
                }
            }
        };

        taskpool.execute(task);
        None
    }

    /// Executes the bulk delete operation: find and complete target messages
    async fn execute_bulk_delete_operation(
        consumer: Arc<Mutex<Consumer>>,
        message_ids: Vec<MessageIdentifier>,
    ) -> Result<(), AppError> {
        use crate::app::updates::messages::utils::find_target_message;

        let mut consumer = consumer.lock().await;
        let total_messages = message_ids.len();

        log::info!(
            "Starting bulk delete operation for {} messages",
            total_messages
        );

        let mut successfully_deleted = 0;
        let mut not_found_count = 0;
        let mut delete_failed_count = 0;
        let mut critical_errors = Vec::new();

        // Process each message individually
        for message_id in &message_ids {
            match find_target_message(&mut consumer, &message_id.id, message_id.sequence).await {
                Ok(target_msg) => {
                    // Complete the message to delete it
                    match consumer.complete_message(&target_msg).await {
                        Ok(()) => {
                            log::debug!("Successfully deleted message {}", message_id.id);
                            successfully_deleted += 1;
                        }
                        Err(e) => {
                            let error_msg =
                                format!("Failed to delete message {}: {}", message_id.id, e);
                            log::error!("{}", error_msg);
                            critical_errors.push(error_msg);
                            delete_failed_count += 1;
                        }
                    }
                }
                Err(_e) => {
                    // Message not found - this is common and not critical
                    // The message may have been processed by another consumer, expired, etc.
                    log::info!(
                        "Message {} was not found in queue (likely already processed/expired)",
                        message_id.id
                    );
                    not_found_count += 1;
                }
            }
        }

        log::info!(
            "Bulk delete operation completed: {} deleted, {} not found, {} failed out of {} total",
            successfully_deleted,
            not_found_count,
            delete_failed_count,
            total_messages
        );

        // Only consider it a critical failure if actual deletions failed
        // Messages not found are treated as already processed (success)
        if delete_failed_count > 0 {
            let error_summary = if critical_errors.len() <= 3 {
                critical_errors.join("; ")
            } else {
                format!(
                    "{} (and {} more errors)",
                    critical_errors[..3].join("; "),
                    critical_errors.len() - 3
                )
            };

            return Err(AppError::ServiceBus(format!(
                "Bulk delete partially failed: {} out of {} messages could not be deleted due to errors. Errors: {}",
                delete_failed_count, total_messages, error_summary
            )));
        }

        Ok(())
    }

    /// Handles successful bulk delete operation
    fn handle_bulk_delete_success(
        tx_to_main: &Sender<Msg>,
        message_ids: &[MessageIdentifier],
        queue_type: QueueType,
    ) {
        log::info!(
            "Bulk delete operation completed successfully for {} messages",
            message_ids.len()
        );

        // Stop loading indicator
        Self::send_message_or_log_error(
            tx_to_main,
            Msg::LoadingActivity(LoadingActivityMsg::Stop),
            "loading stop",
        );

        // Remove messages from local state
        Self::send_message_or_log_error(
            tx_to_main,
            Msg::MessageActivity(MessageActivityMsg::BulkRemoveMessagesFromState(
                message_ids.to_vec(),
            )),
            "bulk remove from state",
        );

        // Show success popup
        let queue_name = match queue_type {
            QueueType::Main => "main queue",
            QueueType::DeadLetter => "dead letter queue",
        };

        let title = "Bulk Delete Complete";

        // Add concurrent processing note only for main queue, not DLQ
        let success_message = if matches!(queue_type, QueueType::Main) {
            format!(
                "{}\n\n✅ Successfully deleted {} message{} from {}\n📍 Action: Messages permanently removed\n\nℹ️  Note: Some messages may have already been processed by other consumers",
                title,
                message_ids.len(),
                if message_ids.len() == 1 { "" } else { "s" },
                queue_name
            )
        } else {
            format!(
                "{}\n\n✅ Successfully deleted {} message{} from {}\n📍 Action: Messages permanently removed",
                title,
                message_ids.len(),
                if message_ids.len() == 1 { "" } else { "s" },
                queue_name
            )
        };

        Self::send_message_or_log_error(
            tx_to_main,
            Msg::PopupActivity(PopupActivityMsg::ShowSuccess(success_message)),
            "success popup",
        );
    }

    /// Handles bulk delete operation errors
    fn handle_bulk_delete_error(
        tx_to_main: &Sender<Msg>,
        tx_to_main_err: &Sender<Msg>,
        error: AppError,
        queue_type: QueueType,
    ) {
        log::error!("Error in bulk delete operation: {}", error);

        // Stop loading indicator
        Self::send_message_or_log_error(
            tx_to_main,
            Msg::LoadingActivity(LoadingActivityMsg::Stop),
            "loading stop",
        );

        // Show error message
        let queue_name = match queue_type {
            QueueType::Main => "main queue",
            QueueType::DeadLetter => "dead letter queue",
        };

        let enhanced_error = AppError::ServiceBus(format!(
            "Failed to delete messages from {}: {}",
            queue_name, error
        ));

        Self::send_message_or_log_error(tx_to_main_err, Msg::Error(enhanced_error), "error");
    }

    /// Get the main queue name for DLQ to Main operation
    fn get_main_queue_name_from_current_dlq(&self) -> Result<String, AppError> {
        let current_queue_name = self
            .queue_state
            .current_queue_name
            .as_ref()
            .ok_or_else(|| AppError::State("No current queue name available".to_string()))?;

        // Remove the DLQ suffix to get the main queue name
        let main_queue_name = if current_queue_name.ends_with("/$deadletterqueue") {
            current_queue_name
                .strip_suffix("/$deadletterqueue")
                .unwrap_or(current_queue_name)
                .to_string()
        } else {
            // If it doesn't end with DLQ suffix, assume it's already the main queue name
            current_queue_name.clone()
        };

        Ok(main_queue_name)
    }
}
