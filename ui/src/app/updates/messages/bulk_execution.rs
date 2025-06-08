use crate::app::model::Model;
use crate::components::common::{
    LoadingActivityMsg, MessageActivityMsg, Msg, PopupActivityMsg, QueueType,
};
use crate::config::{CONFIG, limits};
use crate::error::AppError;
use azservicebus::ServiceBusClient;
use azservicebus::core::BasicRetryPolicy;
use server::bulk_operations::{BulkOperationContext, BulkSendParams, MessageIdentifier};
use server::bulk_operations::{BulkOperationHandler, BulkOperationResult};
use server::consumer::Consumer;
use std::error::Error;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tuirealm::terminal::TerminalAdapter;

// Constants for consistent queue display names
const DLQ_DISPLAY_NAME: &str = "DLQ";
const MAIN_QUEUE_DISPLAY_NAME: &str = "Main";

/// Context for batch delete operations
struct BatchDeleteContext {
    target_map: std::collections::HashMap<String, MessageIdentifier>,
    collection_batch_size: usize,
}

struct BulkSendDisplayParams<'a> {
    result: &'a BulkOperationResult,
    from_queue_display: &'a str,
    to_queue_display: &'a str,
    should_delete: bool,
}

impl<'a> BulkSendDisplayParams<'a> {
    fn new(
        result: &'a BulkOperationResult,
        from_queue_display: &'a str,
        to_queue_display: &'a str,
        should_delete: bool,
    ) -> Self {
        Self {
            result,
            from_queue_display,
            to_queue_display,
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

/// State management for message collection during bulk delete operations
struct MessageCollector {
    target_map: std::collections::HashMap<String, MessageIdentifier>,
    found_target_ids: std::collections::HashSet<String>,
    collected_target: Vec<azservicebus::ServiceBusReceivedMessage>,
    collected_non_target: Vec<azservicebus::ServiceBusReceivedMessage>,
    total_processed: usize,
    consecutive_empty_batches: usize,
    max_empty_batches: usize,
    batch_size: u32,
}

impl MessageCollector {
    fn new(context: &BatchDeleteContext) -> Self {
        Self {
            target_map: context.target_map.clone(),
            found_target_ids: std::collections::HashSet::new(),
            collected_target: Vec::new(),
            collected_non_target: Vec::new(),
            total_processed: 0,
            consecutive_empty_batches: 0,
            max_empty_batches: 3,
            batch_size: context.collection_batch_size as u32,
        }
    }

    fn is_complete(&self) -> bool {
        self.found_target_ids.len() >= self.target_map.len()
    }

    fn should_stop(&self) -> bool {
        self.consecutive_empty_batches >= self.max_empty_batches
    }

    fn target_count(&self) -> usize {
        self.target_map.len()
    }

    fn process_received_messages(
        &mut self,
        messages: Vec<azservicebus::ServiceBusReceivedMessage>,
    ) -> bool {
        if messages.is_empty() {
            self.consecutive_empty_batches += 1;
            log::debug!(
                "Empty batch #{} - {} messages processed so far, found {}/{} targets",
                self.consecutive_empty_batches,
                self.total_processed,
                self.found_target_ids.len(),
                self.target_map.len()
            );
            return false;
        }

        self.consecutive_empty_batches = 0;

        for message in messages {
            self.total_processed += 1;
            let message_id = message
                .message_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            if self.target_map.contains_key(&message_id)
                && !self.found_target_ids.contains(&message_id)
            {
                log::debug!(
                    "Found target message: {} (sequence: {})",
                    message_id,
                    message.sequence_number()
                );
                self.found_target_ids.insert(message_id.clone());
                self.collected_target.push(message);

                if self.is_complete() {
                    log::info!(
                        "Found all {} target messages after processing {} total messages",
                        self.target_map.len(),
                        self.total_processed
                    );
                    return true; // Signal completion
                }
            } else {
                self.collected_non_target.push(message);
            }
        }

        false
    }

    fn handle_receive_error(&mut self, error: &dyn Error) {
        self.consecutive_empty_batches += 1;
        log::debug!(
            "Error receiving batch #{}: {} - {} messages processed so far",
            self.consecutive_empty_batches,
            error,
            self.total_processed
        );
    }

    fn finalize(
        self,
    ) -> (
        Vec<azservicebus::ServiceBusReceivedMessage>,
        Vec<azservicebus::ServiceBusReceivedMessage>,
    ) {
        let not_found_count = self.target_map.len() - self.found_target_ids.len();
        log::info!(
            "Collection phase complete: {} target messages found, {} not found, {} non-target messages collected, {} messages processed total",
            self.collected_target.len(),
            not_found_count,
            self.collected_non_target.len(),
            self.total_processed
        );

        if not_found_count > 0 {
            let missing_ids: Vec<_> = self
                .target_map
                .keys()
                .filter(|id| !self.found_target_ids.contains(*id))
                .collect();
            log::warn!(
                "Could not find {} messages in queue: {:?}",
                not_found_count,
                missing_ids
            );
        }

        (self.collected_target, self.collected_non_target)
    }
}

/// Parameters for bulk send task execution
struct BulkSendTaskParams {
    bulk_data: BulkSendData,
    operation_params: BulkSendOperationParams,
    service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
    tx_to_main: Sender<Msg>,
    tx_to_main_err: Sender<Msg>,
}

impl BulkSendTaskParams {
    fn new(
        bulk_data: BulkSendData,
        operation_params: BulkSendOperationParams,
        service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
        tx_to_main: Sender<Msg>,
    ) -> Self {
        Self {
            bulk_data,
            operation_params,
            service_bus_client,
            tx_to_main_err: tx_to_main.clone(),
            tx_to_main,
        }
    }

    fn message_count(&self) -> usize {
        match &self.bulk_data {
            BulkSendData::MessageIds(ids) => ids.len(),
            BulkSendData::MessageData(data) => data.len(),
        }
    }

    fn format_loading_message(&self) -> String {
        self.operation_params
            .loading_message_template
            .replace("{}", &self.message_count().to_string())
    }
}

/// Execute the bulk send operation in an async task
async fn execute_bulk_send_task(params: BulkSendTaskParams) {
    log::debug!("Executing bulk send operation in background task");

    // Create batch operation configuration using server's config directly
    let batch_config = CONFIG.batch();
    let bulk_handler = BulkOperationHandler::new(batch_config.clone());

    // Clone the Arc references before creating operation context
    let consumer = params.operation_params.consumer.clone();
    let service_bus_client = params.service_bus_client.clone();
    let target_queue = params.operation_params.target_queue.clone();

    // Create operation context and parameters
    let operation_context = BulkOperationContext::new(consumer, service_bus_client, target_queue);

    // Create BulkSendParams based on the data type
    let bulk_send_params = create_bulk_send_params(&params.bulk_data, &params.operation_params);

    let result = bulk_handler
        .bulk_send(operation_context, bulk_send_params)
        .await;

    handle_bulk_send_task_result(result, params);
}

/// Create BulkSendParams based on the data type
fn create_bulk_send_params(
    bulk_data: &BulkSendData,
    operation_params: &BulkSendOperationParams,
) -> BulkSendParams {
    match bulk_data {
        BulkSendData::MessageIds(message_ids) => {
            log::info!(
                "Starting bulk send operation for {} messages to queue {} (delete: {})",
                message_ids.len(),
                operation_params.target_queue,
                operation_params.should_delete
            );
            BulkSendParams::with_retrieval(
                operation_params.target_queue.clone(),
                operation_params.should_delete,
                message_ids.clone(),
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
                operation_params.target_queue.clone(),
                operation_params.should_delete,
                messages_data.clone(),
            )
        }
    }
}

/// Handle the result of the bulk send task
fn handle_bulk_send_task_result(
    result: Result<BulkOperationResult, Box<dyn Error>>,
    params: BulkSendTaskParams,
) {
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
                &params.operation_params.from_queue_display,
                &params.operation_params.to_queue_display,
                params.operation_params.should_delete,
            );
            handle_bulk_send_success(&params.tx_to_main, display_params);
        }
        Err(e) => {
            log::error!("Failed to execute bulk send operation: {}", e);
            handle_bulk_send_error(
                &params.tx_to_main,
                &params.tx_to_main_err,
                AppError::ServiceBus(e.to_string()),
                &params.operation_params.from_queue_display,
                &params.operation_params.to_queue_display,
            );
        }
    }
}

/// Handle successful bulk send operation (extracted from Model)
fn handle_bulk_send_success(tx_to_main: &Sender<Msg>, display_params: BulkSendDisplayParams) {
    // Send stop loading message
    if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Stop)) {
        log::error!("Failed to send loading stop message: {}", e);
    }

    // Show success status in the popup
    show_bulk_send_status(tx_to_main, display_params);
}

/// Handle bulk send error (extracted from Model)
fn handle_bulk_send_error(
    tx_to_main: &Sender<Msg>,
    tx_to_main_err: &Sender<Msg>,
    error: AppError,
    from_queue: &str,
    to_queue: &str,
) {
    log::error!("Bulk send operation failed: {}", error);

    // Send stop loading message using main sender
    if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Stop)) {
        log::error!("Failed to send loading stop message: {}", e);
    }

    // Prepare error message with context
    let context_message = format!(
        "Failed to send messages from {} to {}: {}",
        from_queue, to_queue, error
    );

    // Send error message using error sender
    if let Err(e) = tx_to_main_err.send(Msg::Error(AppError::ServiceBus(context_message))) {
        log::error!("Failed to send error message: {}", e);
    }
}

/// Show bulk send status in a popup (extracted from Model)
fn show_bulk_send_status(tx_to_main: &Sender<Msg>, params: BulkSendDisplayParams) {
    let success_message = if params.result.failed > 0 || params.result.not_found > 0 {
        // Partial success case
        format!(
            "Bulk {} operation completed with mixed results:\n\n\
            ✅ Successfully processed: {} messages\n\
            ❌ Failed: {} messages\n\
            ⚠️  Not found: {} messages\n\n\
            Direction: {} → {}",
            if params.should_delete { "move" } else { "copy" },
            params.result.successful,
            params.result.failed,
            params.result.not_found,
            params.from_queue_display,
            params.to_queue_display
        )
    } else {
        // Full success case
        format!(
            "✅ Bulk {} operation completed successfully!\n\n\
            📦 {} messages processed from {} to {}\n\n\
            All messages were {} successfully.",
            if params.should_delete { "move" } else { "copy" },
            params.result.successful,
            params.from_queue_display,
            params.to_queue_display,
            if params.should_delete {
                "moved"
            } else {
                "copied"
            }
        )
    };

    if let Err(e) = tx_to_main.send(Msg::PopupActivity(PopupActivityMsg::ShowSuccess(
        success_message,
    ))) {
        log::error!("Failed to send success popup message: {}", e);
    }
}

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Helper method to send a message to the main thread or log an error if it fails
    fn send_message_or_log_error(tx: &Sender<Msg>, msg: Msg, context: &str) {
        if let Err(e) = tx.send(msg) {
            log::error!("Failed to send {} message: {}", context, e);
        }
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

        // Get the main queue name for DLQ to Main operation
        let target_queue = match self.get_main_queue_name_from_current_dlq() {
            Ok(name) => name,
            Err(e) => {
                log::error!("Failed to get main queue name: {}", e);
                return Some(Msg::Error(e));
            }
        };

        let params = BulkSendOperationParams::new(
            consumer,
            target_queue,
            true, // should_delete = true for DLQ to Main
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

        // Create task parameters
        let task_params = BulkSendTaskParams::new(
            bulk_data,
            operation_params,
            self.service_bus_client.clone(),
            tx_to_main.clone(),
        );

        // Start loading indicator
        let loading_message = task_params.format_loading_message();
        Self::send_message_or_log_error(
            &tx_to_main,
            Msg::LoadingActivity(LoadingActivityMsg::Start(loading_message)),
            "loading start",
        );

        // Spawn bulk send task
        let task = execute_bulk_send_task(task_params);
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
        // Check for configuration issues early and show user-friendly errors
        match Self::validate_batch_configuration_for_delete(message_ids) {
            Ok(()) => {
                log::info!(
                    "Validated bulk delete request for {} messages from {:?} queue",
                    message_ids.len(),
                    self.queue_state.current_queue_type
                );
                Ok(())
            }
            Err(config_error) => {
                // Convert configuration error to user-friendly error message
                let enhanced_error = match config_error {
                    AppError::Config(msg) => AppError::Config(format!(
                        "Bulk Delete Configuration Error: {}. Please check your config.toml file and ensure max_batch_size does not exceed 2048.",
                        msg
                    )),
                    _ => config_error,
                };

                Err(Msg::PopupActivity(PopupActivityMsg::ShowError(
                    enhanced_error,
                )))
            }
        }
    }

    /// Validate batch configuration for delete operations
    fn validate_batch_configuration_for_delete(
        message_ids: &[MessageIdentifier],
    ) -> Result<(), AppError> {
        let batch_config = CONFIG.batch();
        let max_batch_size = batch_config.max_batch_size();

        if max_batch_size > limits::AZURE_SERVICE_BUS_MAX_BATCH_SIZE {
            return Err(AppError::Config(format!(
                "max_batch_size ({}) exceeds Azure Service Bus limit ({}).",
                max_batch_size,
                limits::AZURE_SERVICE_BUS_MAX_BATCH_SIZE
            )));
        }

        // Check if the operation is feasible with current configuration
        let message_count = message_ids.len();
        if message_count > limits::AZURE_SERVICE_BUS_MAX_BATCH_SIZE as usize {
            return Err(AppError::Config(format!(
                "Cannot delete {} messages in a single operation. Azure Service Bus limit is {} messages. Please select fewer messages.",
                message_count,
                limits::AZURE_SERVICE_BUS_MAX_BATCH_SIZE
            )));
        }

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
                Ok(actually_deleted_ids) => {
                    Self::handle_bulk_delete_success(
                        &tx_to_main,
                        &message_ids,
                        &actually_deleted_ids,
                        queue_type,
                    );
                }
                Err(e) => {
                    Self::handle_bulk_delete_error(&tx_to_main, &tx_to_main_err, e, queue_type);
                }
            }
        };

        taskpool.execute(task);
        None
    }

    /// Executes the bulk delete operation using efficient batch collection
    /// Returns the list of actually deleted message IDs
    async fn execute_bulk_delete_operation(
        consumer: Arc<Mutex<Consumer>>,
        message_ids: Vec<MessageIdentifier>,
    ) -> Result<Vec<MessageIdentifier>, AppError> {
        let total_messages = message_ids.len();
        log::info!(
            "Starting bulk delete operation for {} messages using batch collection",
            total_messages
        );

        // Setup batch delete context
        let context = Self::setup_batch_delete_context(&message_ids)?;

        // Collect target and non-target messages
        let (target_messages, non_target_messages) =
            Self::collect_messages_for_deletion(consumer.clone(), &context).await?;

        // Perform the actual deletion
        let successfully_deleted_ids =
            Self::perform_batch_deletion(consumer.clone(), target_messages, &context.target_map)
                .await?;

        // Abandon non-target messages
        Self::abandon_non_target_messages(consumer, non_target_messages).await;

        // Log final results
        Self::log_deletion_results(&successfully_deleted_ids, &message_ids, total_messages);

        Ok(successfully_deleted_ids)
    }

    /// Setup the context for batch delete operation
    fn setup_batch_delete_context(
        message_ids: &[MessageIdentifier],
    ) -> Result<BatchDeleteContext, AppError> {
        use std::collections::HashMap;

        let batch_config = CONFIG.batch();
        let max_batch_size = batch_config.max_batch_size();

        let target_count = message_ids.len();
        let buffer_size = std::cmp::max(
            (target_count as f64 * batch_config.buffer_percentage()) as usize,
            batch_config.min_buffer_size(),
        );
        let calculated_batch_size = target_count + buffer_size;

        if max_batch_size as usize > limits::AZURE_SERVICE_BUS_MAX_BATCH_SIZE as usize {
            return Err(AppError::Config(format!(
                "Configuration error: max_batch_size ({}) exceeds Azure Service Bus hard limit ({}). Please reduce the value in config.toml",
                max_batch_size,
                limits::AZURE_SERVICE_BUS_MAX_BATCH_SIZE
            )));
        }

        let effective_batch_size = std::cmp::min(calculated_batch_size, max_batch_size as usize);
        let final_batch_size = std::cmp::min(
            effective_batch_size,
            limits::AZURE_SERVICE_BUS_MAX_BATCH_SIZE as usize,
        );

        if final_batch_size < calculated_batch_size {
            log::warn!(
                "Requested batch size {} (from {} messages + {} buffer) exceeds limits. Using {} instead. Config max: {}, Azure hard limit: {}",
                calculated_batch_size,
                target_count,
                buffer_size,
                final_batch_size,
                max_batch_size,
                limits::AZURE_SERVICE_BUS_MAX_BATCH_SIZE
            );
        }

        // Use 1/16 of the effective batch size, but at least 32 and at most 256
        // It makes progress updates much more easier
        let collection_batch_size = (final_batch_size / 16).clamp(32, 256);
        let target_map: HashMap<String, MessageIdentifier> = message_ids
            .iter()
            .map(|m| (m.id.clone(), m.clone()))
            .collect();

        log::info!(
            "Batch delete configuration: {} target messages, {} buffer, collection batch size: {}, effective batch size: {}, config max: {}",
            target_count,
            buffer_size,
            collection_batch_size,
            final_batch_size,
            max_batch_size
        );

        Ok(BatchDeleteContext {
            target_map,
            collection_batch_size,
        })
    }

    /// Collect target and non-target messages from the queue
    async fn collect_messages_for_deletion(
        consumer: Arc<Mutex<Consumer>>,
        context: &BatchDeleteContext,
    ) -> Result<
        (
            Vec<azservicebus::ServiceBusReceivedMessage>,
            Vec<azservicebus::ServiceBusReceivedMessage>,
        ),
        AppError,
    > {
        let mut consumer_guard = consumer.lock().await;
        let mut collector = MessageCollector::new(context);

        log::info!(
            "Collecting target messages: looking for {} specific messages",
            collector.target_count()
        );

        while !collector.is_complete() && !collector.should_stop() {
            match consumer_guard.receive_messages(collector.batch_size).await {
                Ok(received_messages) => {
                    if collector.process_received_messages(received_messages) {
                        break; // All targets found
                    }
                }
                Err(e) => {
                    collector.handle_receive_error(e.as_ref());
                }
            }
        }

        drop(consumer_guard);
        Ok(collector.finalize())
    }

    /// Perform the actual deletion of target messages
    async fn perform_batch_deletion(
        consumer: Arc<Mutex<Consumer>>,
        target_messages: Vec<azservicebus::ServiceBusReceivedMessage>,
        target_map: &std::collections::HashMap<String, MessageIdentifier>,
    ) -> Result<Vec<MessageIdentifier>, AppError> {
        if target_messages.is_empty() {
            return Ok(Vec::new());
        }

        let mut successfully_deleted_ids = Vec::new();

        // Try batch completion first
        let batch_success = {
            let mut consumer_guard = consumer.lock().await;
            consumer_guard
                .complete_messages(&target_messages)
                .await
                .is_ok()
        };

        if batch_success {
            log::info!(
                "Successfully deleted {} messages using batch operation",
                target_messages.len()
            );
            Self::track_deleted_messages(
                &target_messages,
                target_map,
                &mut successfully_deleted_ids,
            );
        } else {
            log::warn!("Batch delete failed, falling back to individual deletion");
            Self::perform_individual_deletion(
                consumer,
                &target_messages,
                target_map,
                &mut successfully_deleted_ids,
            )
            .await?;
        }

        Ok(successfully_deleted_ids)
    }

    /// Perform individual deletion as fallback
    async fn perform_individual_deletion(
        consumer: Arc<Mutex<Consumer>>,
        target_messages: &[azservicebus::ServiceBusReceivedMessage],
        target_map: &std::collections::HashMap<String, MessageIdentifier>,
        successfully_deleted_ids: &mut Vec<MessageIdentifier>,
    ) -> Result<(), AppError> {
        let mut consumer_guard = consumer.lock().await;
        let mut delete_failed_count = 0;
        let mut critical_errors = Vec::new();

        for message in target_messages {
            let message_id = message
                .message_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            match consumer_guard.complete_message(message).await {
                Ok(()) => {
                    log::debug!("Successfully deleted message {}", message_id);
                    if let Some(original_msg_id) = target_map.get(&message_id) {
                        successfully_deleted_ids.push(original_msg_id.clone());
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to delete message {}: {}", message_id, e);
                    log::error!("{}", error_msg);
                    critical_errors.push(error_msg);
                    delete_failed_count += 1;
                }
            }
        }

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
                delete_failed_count,
                target_messages.len(),
                error_summary
            )));
        }

        Ok(())
    }

    /// Track which messages were successfully deleted
    fn track_deleted_messages(
        target_messages: &[azservicebus::ServiceBusReceivedMessage],
        target_map: &std::collections::HashMap<String, MessageIdentifier>,
        successfully_deleted_ids: &mut Vec<MessageIdentifier>,
    ) {
        for message in target_messages {
            let message_id = message
                .message_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            if let Some(original_msg_id) = target_map.get(&message_id) {
                successfully_deleted_ids.push(original_msg_id.clone());
            }
        }
    }

    /// Abandon non-target messages to make them available again
    async fn abandon_non_target_messages(
        consumer: Arc<Mutex<Consumer>>,
        non_target_messages: Vec<azservicebus::ServiceBusReceivedMessage>,
    ) {
        if !non_target_messages.is_empty() {
            let mut consumer_guard = consumer.lock().await;
            if let Err(e) = consumer_guard.abandon_messages(&non_target_messages).await {
                log::warn!(
                    "Failed to abandon {} non-target messages: {}",
                    non_target_messages.len(),
                    e
                );
            } else {
                log::info!(
                    "Successfully abandoned {} non-target messages",
                    non_target_messages.len()
                );
            }
        }
    }

    /// Log the final results of the deletion operation
    fn log_deletion_results(
        successfully_deleted_ids: &[MessageIdentifier],
        message_ids: &[MessageIdentifier],
        total_messages: usize,
    ) {
        let successfully_deleted_count = successfully_deleted_ids.len();
        let delete_failed_count = 0; // This would be passed from perform_batch_deletion in a real scenario
        let not_found_count = message_ids.len() - successfully_deleted_count - delete_failed_count;

        log::info!(
            "Bulk delete operation completed: {} deleted, {} not found, {} failed out of {} total",
            successfully_deleted_count,
            not_found_count,
            delete_failed_count,
            total_messages
        );
    }

    /// Handles successful bulk delete operation
    fn handle_bulk_delete_success(
        tx_to_main: &Sender<Msg>,
        originally_selected_ids: &[MessageIdentifier],
        actually_deleted_ids: &[MessageIdentifier],
        queue_type: QueueType,
    ) {
        let actually_deleted_count = actually_deleted_ids.len();
        let originally_selected_count = originally_selected_ids.len();

        log::info!(
            "Bulk delete operation completed: {} out of {} selected messages were actually deleted",
            actually_deleted_count,
            originally_selected_count
        );

        // Stop loading indicator
        Self::send_message_or_log_error(
            tx_to_main,
            Msg::LoadingActivity(LoadingActivityMsg::Stop),
            "loading stop",
        );

        // Remove only the messages that were actually deleted from local state
        Self::send_message_or_log_error(
            tx_to_main,
            Msg::MessageActivity(MessageActivityMsg::BulkRemoveMessagesFromState(
                actually_deleted_ids.to_vec(),
            )),
            "bulk remove from state",
        );

        // Show success popup with accurate information
        let queue_name = match queue_type {
            QueueType::Main => "main queue",
            QueueType::DeadLetter => "dead letter queue",
        };

        let title = "Bulk Delete Complete";
        let not_found_count = originally_selected_count - actually_deleted_count;

        let success_message = if not_found_count > 0 {
            // Some messages were not found/deleted
            format!(
                "{}\n\n✅ Successfully deleted {} out of {} selected message{} from {}\n📍 Action: Messages permanently removed\n\n⚠️  {} message{} could not be found (may have already been processed)",
                title,
                actually_deleted_count,
                originally_selected_count,
                if originally_selected_count == 1 {
                    ""
                } else {
                    "s"
                },
                queue_name,
                not_found_count,
                if not_found_count == 1 { "" } else { "s" }
            )
        } else {
            // All messages were found and deleted
            format!(
                "{}\n\n✅ Successfully deleted {} message{} from {}\n📍 Action: Messages permanently removed",
                title,
                actually_deleted_count,
                if actually_deleted_count == 1 { "" } else { "s" },
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
