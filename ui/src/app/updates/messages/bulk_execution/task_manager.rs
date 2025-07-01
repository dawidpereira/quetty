use crate::app::bulk_operation_processor::BulkOperationPostProcessor;
use crate::components::common::{LoadingActivityMsg, Msg};
use crate::error::{AppError, ErrorReporter};

use server::bulk_operations::{BulkOperationResult, MessageIdentifier};
use server::service_bus_manager::{ServiceBusCommand, ServiceBusManager, ServiceBusResponse};
use server::taskpool::TaskPool;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use tokio::sync::Mutex;

/// Parameters for bulk send operations
#[derive(Debug, Clone)]
pub struct BulkSendParams {
    pub target_queue: String,
    pub should_delete: bool,
    pub loading_message_template: String,
    pub from_queue_display: String,
    pub to_queue_display: String,
}

impl BulkSendParams {
    pub fn new(
        target_queue: String,
        should_delete: bool,
        loading_message_template: &str,
        from_queue_display: &str,
        to_queue_display: &str,
    ) -> Self {
        Self {
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
    MessageIds(Vec<String>),
    MessageData(Vec<(String, Vec<u8>)>),
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
    pub operation_params: BulkSendParams,
    pub service_bus_manager: Arc<Mutex<ServiceBusManager>>,
    pub tx_to_main: Sender<Msg>,
    pub tx_to_main_err: Sender<Msg>,
    pub repeat_count: usize,
    pub error_reporter: crate::error::ErrorReporter,
    pub max_position: usize,
}

impl BulkSendTaskParams {
    pub fn new(
        bulk_data: BulkSendData,
        operation_params: BulkSendParams,
        service_bus_manager: Arc<Mutex<ServiceBusManager>>,
        tx_to_main: Sender<Msg>,
        repeat_count: usize,
        error_reporter: ErrorReporter,
        max_position: usize,
    ) -> Self {
        let tx_to_main_err = tx_to_main.clone();
        Self {
            bulk_data,
            operation_params,
            service_bus_manager,
            tx_to_main,
            tx_to_main_err,
            repeat_count,
            error_reporter,
            max_position,
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

    // Execute the bulk send operation using service bus manager
    let result = match &params.bulk_data {
        BulkSendData::MessageData(messages_data) => {
            // Convert to the format expected by the service bus manager
            let messages_data_converted: Vec<(MessageIdentifier, Vec<u8>)> = messages_data
                .iter()
                .map(|(id, data)| (MessageIdentifier::new(id.clone(), 0), data.clone()))
                .collect();

            let command = ServiceBusCommand::BulkSendPeeked {
                messages_data: messages_data_converted,
                target_queue: params.operation_params.target_queue.clone(),
                repeat_count: params.repeat_count,
            };

            let response = params
                .service_bus_manager
                .lock()
                .await
                .execute_command(command)
                .await;

            match response {
                ServiceBusResponse::MessagesSent { stats, .. } => {
                    log::info!("Bulk send with data completed successfully: {:?}", stats);
                    // Convert OperationStats to BulkOperationResult
                    let mut result = BulkOperationResult::new(params.message_count());
                    result.successful = stats.successful;
                    result.failed = stats.failed;
                    Ok(result)
                }
                ServiceBusResponse::Error { error } => {
                    log::error!("Bulk send with data failed: {}", error);
                    Err(AppError::ServiceBus(error.to_string()))
                }
                _ => Err(AppError::ServiceBus(
                    "Unexpected response for bulk send peeked".to_string(),
                )),
            }
        }
        BulkSendData::MessageIds(message_ids) => {
            // Convert to the format expected by the service bus manager
            let message_ids_converted: Vec<MessageIdentifier> = message_ids
                .iter()
                .map(|id| MessageIdentifier::new(id.clone(), 0))
                .collect();

            let command = ServiceBusCommand::BulkSend {
                message_ids: message_ids_converted,
                target_queue: params.operation_params.target_queue.clone(),
                should_delete_source: params.operation_params.should_delete,
                repeat_count: params.repeat_count,
                max_position: params.max_position,
            };

            let response = params
                .service_bus_manager
                .lock()
                .await
                .execute_command(command)
                .await;

            match response {
                ServiceBusResponse::BulkOperationCompleted { result } => {
                    log::info!("Bulk send with IDs completed successfully: {:?}", result);
                    Ok(result)
                }
                ServiceBusResponse::Error { error } => {
                    log::error!("Bulk send with IDs failed: {}", error);
                    Err(AppError::ServiceBus(error.to_string()))
                }
                _ => Err(AppError::ServiceBus(
                    "Unexpected response for bulk send".to_string(),
                )),
            }
        }
    };

    handle_bulk_send_task_result_simple(result, params);
}

/// Handle the result of the bulk send task (simplified version)
pub fn handle_bulk_send_task_result_simple(
    result: Result<BulkOperationResult, crate::error::AppError>,
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

            handle_bulk_send_success_simple(
                &params.tx_to_main,
                operation_result,
                &params.operation_params,
                &params.bulk_data,
                &params.error_reporter,
            );
        }
        Err(error) => {
            params.error_reporter.report_bulk_operation_error(
                "send",
                params.message_count(),
                &error,
            );

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

/// Handle successful bulk send operation (simplified version)
fn handle_bulk_send_success_simple(
    tx_to_main: &Sender<Msg>,
    result: BulkOperationResult,
    operation_params: &BulkSendParams,
    bulk_data: &BulkSendData,
    error_reporter: &crate::error::ErrorReporter,
) {
    // Stop loading indicator
    BulkTaskManager::send_message_or_log_error(
        tx_to_main,
        Msg::LoadingActivity(LoadingActivityMsg::Stop),
        "loading stop",
    );

    // Extract message IDs for centralized processing
    let message_ids = if operation_params.should_delete && result.successful > 0 {
        BulkOperationPostProcessor::extract_successfully_processed_message_ids(
            bulk_data,
            result.successful,
        )
    } else {
        vec![] // No message IDs needed if not deleting or no successful operations
    };

    // Get auto-reload threshold
    let auto_reload_threshold = crate::config::get_config_or_panic()
        .batch()
        .auto_reload_threshold();

    // Create context for centralized post-processing
    let context = BulkOperationPostProcessor::create_send_context(
        &result,
        message_ids,
        auto_reload_threshold,
        operation_params.from_queue_display.clone(),
        operation_params.to_queue_display.clone(),
        operation_params.should_delete,
    );

    // Use centralized post-processor to handle completion
    if let Err(e) =
        BulkOperationPostProcessor::handle_completion(&context, tx_to_main, error_reporter)
    {
        error_reporter.report_bulk_operation_error("complete", result.successful, &e);
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
