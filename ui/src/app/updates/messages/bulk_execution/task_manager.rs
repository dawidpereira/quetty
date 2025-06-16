use crate::components::common::{LoadingActivityMsg, Msg};
use crate::error::AppError;
use azservicebus::{ServiceBusClient, core::BasicRetryPolicy};
use server::bulk_operations::{BulkOperationContext, BulkSendParams, MessageIdentifier};
use server::bulk_operations::{BulkOperationHandler, BulkOperationResult};
use server::consumer::Consumer;
use server::taskpool::TaskPool;
use std::error::Error;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use tokio::sync::Mutex;

// Constants for consistent queue display names
pub const DLQ_DISPLAY_NAME: &str = "DLQ";
pub const MAIN_QUEUE_DISPLAY_NAME: &str = "Main";

/// Parameters for bulk send operations
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

/// Data types for bulk send operations
pub enum BulkSendData {
    MessageIds(Vec<MessageIdentifier>),
    MessageData(Vec<(MessageIdentifier, Vec<u8>)>),
}

impl BulkSendData {
    pub fn message_count(&self) -> usize {
        match self {
            BulkSendData::MessageIds(ids) => ids.len(),
            BulkSendData::MessageData(data) => data.len(),
        }
    }
}

/// Task parameters for async bulk send operations
pub struct BulkSendTaskParams {
    pub bulk_data: BulkSendData,
    pub operation_params: BulkSendOperationParams,
    pub service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
    pub tx_to_main: Sender<Msg>,
    pub tx_to_main_err: Sender<Msg>,
}

impl BulkSendTaskParams {
    pub fn new(
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

    pub fn message_count(&self) -> usize {
        self.bulk_data.message_count()
    }

    pub fn format_loading_message(&self) -> String {
        self.operation_params
            .loading_message_template
            .replace("{}", &self.message_count().to_string())
    }
}

/// Manages async task execution for bulk operations
pub struct BulkTaskManager {
    taskpool: TaskPool,
    tx_to_main: Sender<Msg>,
}

impl BulkTaskManager {
    pub fn new(taskpool: TaskPool, tx_to_main: Sender<Msg>) -> Self {
        Self {
            taskpool,
            tx_to_main,
        }
    }

    /// Execute a bulk send task with proper loading indicator management
    pub fn execute_bulk_send_task(&self, task_params: BulkSendTaskParams) {
        // Start loading indicator
        let loading_message = task_params.format_loading_message();
        Self::send_message_or_log_error(
            &self.tx_to_main,
            Msg::LoadingActivity(LoadingActivityMsg::Start(loading_message)),
            "loading start",
        );

        // Spawn bulk send task
        let task = execute_bulk_send_task(task_params);
        self.taskpool.execute(task);
    }

    /// Helper method to send a message to the main thread or log an error if it fails
    pub fn send_message_or_log_error(tx: &Sender<Msg>, msg: Msg, context: &str) {
        if let Err(e) = tx.send(msg) {
            log::error!("Failed to send {} message: {}", context, e);
        }
    }
}

/// Execute bulk send task asynchronously
pub async fn execute_bulk_send_task(params: BulkSendTaskParams) {
    log::info!(
        "Starting bulk send task for {} messages",
        params.message_count()
    );

    // Use the service bus client reference directly

    // Use the application's loaded configuration
    let bulk_handler = BulkOperationHandler::new(crate::config::CONFIG.batch().clone());

    // Create operation context and parameters
    let operation_context = BulkOperationContext::new(
        params.operation_params.consumer.clone(),
        params.service_bus_client.clone(),
        params.operation_params.target_queue.clone(),
    );

    // Create BulkSendParams based on the data type
    let bulk_send_params = create_bulk_send_params(&params.bulk_data, &params.operation_params);

    let result = bulk_handler
        .bulk_send(operation_context, bulk_send_params)
        .await;

    handle_bulk_send_task_result(result, params);
}

/// Create bulk send parameters based on data type
pub fn create_bulk_send_params(
    bulk_data: &BulkSendData,
    operation_params: &BulkSendOperationParams,
) -> BulkSendParams {
    match bulk_data {
        BulkSendData::MessageIds(message_ids) => BulkSendParams::with_retrieval(
            operation_params.target_queue.clone(),
            operation_params.should_delete,
            message_ids.clone(),
        ),
        BulkSendData::MessageData(messages_data) => BulkSendParams::with_message_data(
            operation_params.target_queue.clone(),
            operation_params.should_delete,
            messages_data.clone(),
        ),
    }
}

/// Handle the result of the bulk send task
pub fn handle_bulk_send_task_result(
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

            handle_bulk_send_success(
                &params.tx_to_main,
                operation_result,
                &params.operation_params,
                &params.bulk_data,
            );
        }
        Err(error) => {
            log::error!("Bulk send operation failed: {}", error);

            handle_bulk_send_error(
                &params.tx_to_main,
                &params.tx_to_main_err,
                error.to_string(),
                &params.operation_params.from_queue_display,
                &params.operation_params.to_queue_display,
            );
        }
    }
}

/// Handle successful bulk send operation
fn handle_bulk_send_success(
    tx_to_main: &Sender<Msg>,
    result: BulkOperationResult,
    operation_params: &BulkSendOperationParams,
    bulk_data: &BulkSendData,
) {
    // Stop loading indicator
    BulkTaskManager::send_message_or_log_error(
        tx_to_main,
        Msg::LoadingActivity(LoadingActivityMsg::Stop),
        "loading stop",
    );

    // If messages should be deleted (moved, not copied), remove them from the local state
    if operation_params.should_delete && result.successful > 0 {
        let message_ids_to_remove =
            extract_successfully_processed_message_ids(bulk_data, result.successful);

        if !message_ids_to_remove.is_empty() {
            log::info!(
                "Removing {} successfully processed messages from local state",
                message_ids_to_remove.len()
            );
            BulkTaskManager::send_message_or_log_error(
                tx_to_main,
                Msg::MessageActivity(
                    crate::components::common::MessageActivityMsg::BulkRemoveMessagesFromState(
                        message_ids_to_remove,
                    ),
                ),
                "bulk remove from state",
            );
        }
    }

    // Show success message
    let success_message = format_bulk_send_success_message(&result, operation_params);

    if let Err(e) = tx_to_main.send(Msg::PopupActivity(
        crate::components::common::PopupActivityMsg::ShowSuccess(success_message),
    )) {
        log::error!("Failed to send success popup message: {}", e);
    }
}

/// Handle bulk send operation error
fn handle_bulk_send_error(
    tx_to_main: &Sender<Msg>,
    tx_to_main_err: &Sender<Msg>,
    error: String,
    from_queue: &str,
    to_queue: &str,
) {
    // Stop loading indicator
    BulkTaskManager::send_message_or_log_error(
        tx_to_main,
        Msg::LoadingActivity(LoadingActivityMsg::Stop),
        "loading stop",
    );

    // Prepare error message with context
    let context_message = format!(
        "Failed to send messages from {} to {}: {}",
        from_queue, to_queue, error
    );

    // Send error message using error sender
    BulkTaskManager::send_message_or_log_error(
        tx_to_main_err,
        Msg::Error(AppError::ServiceBus(context_message)),
        "error",
    );
}

/// Extract message IDs that were successfully processed for removal from local state
fn extract_successfully_processed_message_ids(
    bulk_data: &BulkSendData,
    successful_count: usize,
) -> Vec<MessageIdentifier> {
    match bulk_data {
        BulkSendData::MessageIds(message_ids) => {
            // Take up to the successful count from the original message IDs
            // Note: This assumes the bulk operation processes messages in order
            // For more precise tracking, we would need the actual IDs from the operation result
            message_ids.iter().take(successful_count).cloned().collect()
        }
        BulkSendData::MessageData(messages_data) => {
            // Extract message IDs from the message data
            messages_data
                .iter()
                .take(successful_count)
                .map(|(id, _)| id.clone())
                .collect()
        }
    }
}

/// Format success message for bulk send operations
fn format_bulk_send_success_message(
    result: &BulkOperationResult,
    operation_params: &BulkSendOperationParams,
) -> String {
    if result.failed > 0 || result.not_found > 0 {
        // Partial success case
        format!(
            "Bulk {} operation completed with mixed results:\n\n\
            ✅ Successfully processed: {} messages\n\
            ❌ Failed: {} messages\n\
            ⚠️  Not found: {} messages\n\n\
            Direction: {} → {}",
            if operation_params.should_delete {
                "move"
            } else {
                "copy"
            },
            result.successful,
            result.failed,
            result.not_found,
            operation_params.from_queue_display,
            operation_params.to_queue_display
        )
    } else {
        // Full success case
        format!(
            "✅ Bulk {} operation completed successfully!\n\n\
            📦 {} messages processed from {} to {}\n\n\
            All messages were {} successfully.",
            if operation_params.should_delete {
                "move"
            } else {
                "copy"
            },
            result.successful,
            operation_params.from_queue_display,
            operation_params.to_queue_display,
            if operation_params.should_delete {
                "moved"
            } else {
                "copied"
            }
        )
    }
}
