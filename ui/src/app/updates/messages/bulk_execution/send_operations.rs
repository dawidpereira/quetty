use super::operation_setup::{BulkOperationContext, BulkOperationSetup, BulkOperationType};
use super::task_manager::{BulkSendData, BulkSendParams};
use crate::app::bulk_operation_processor::BulkOperationPostProcessor;
use crate::app::model::Model;
use crate::app::task_manager::ProgressReporter;
use crate::components::common::Msg;
use crate::error::AppError;
use server::bulk_operations::MessageIdentifier;
use server::model::BodyData;
use server::service_bus_manager::{ServiceBusCommand, ServiceBusResponse};
use std::sync::Arc;
use tuirealm::terminal::TerminalAdapter;

/// Execute bulk resend from DLQ operation (with deleting from DLQ)
pub fn handle_bulk_resend_from_dlq_execution<T: TerminalAdapter>(
    model: &mut Model<T>,
    message_ids: Vec<MessageIdentifier>,
) -> Option<Msg> {
    // Use BulkOperationSetup for validation and configuration
    let validated_operation = match BulkOperationSetup::new(model, message_ids)
        .operation_type(BulkOperationType::ResendFromDlq {
            delete_source: true,
        })
        .validate_and_build()
    {
        Ok(op) => op,
        Err(e) => {
            model
                .error_reporter
                .report_simple(e, "BulkResend", "validation");
            return None;
        }
    };

    // Get target queue from validated operation
    let target_queue = match validated_operation.get_target_queue() {
        Ok(queue) => queue,
        Err(e) => {
            model.error_reporter.report_service_bus_error(
                "get_target_queue",
                &e,
                Some("Check your queue configuration"),
            );
            return None;
        }
    };

    let (from_display, to_display) = validated_operation.get_queue_display_names();
    let loading_template = validated_operation.get_loading_message();
    let context = validated_operation.calculate_post_processing_context();

    // For resend WITH DELETE, we need to use message retrieval approach
    // so the server can actually receive and complete/delete the messages from DLQ
    start_bulk_send_operation(
        model,
        validated_operation.message_ids().to_vec(),
        BulkSendParams::new(
            target_queue,
            validated_operation.should_delete_source(),
            &loading_template.replace(&validated_operation.message_ids().len().to_string(), "{}"),
            &from_display,
            &to_display,
        ),
        context,
    )
}

/// Execute bulk resend-only from DLQ operation (without deleting from DLQ)
pub fn handle_bulk_resend_from_dlq_only_execution<T: TerminalAdapter>(
    model: &mut Model<T>,
    message_ids: Vec<MessageIdentifier>,
) -> Option<Msg> {
    // Use BulkOperationSetup for validation and configuration
    let validated_operation = match BulkOperationSetup::new(model, message_ids)
        .operation_type(BulkOperationType::ResendFromDlq {
            delete_source: false,
        })
        .validate_and_build()
    {
        Ok(op) => op,
        Err(e) => {
            model
                .error_reporter
                .report_simple(e, "BulkResendOnly", "validation");
            return None;
        }
    };

    // For resend-only, we get message data from the current state (peeked messages)
    let messages_data =
        match extract_message_data_from_current_state(model, validated_operation.message_ids()) {
            Ok(data) => data,
            Err(_) => return None,
        };

    // Get target queue from validated operation
    let target_queue = match validated_operation.get_target_queue() {
        Ok(queue) => queue,
        Err(e) => {
            model.error_reporter.report_service_bus_error(
                "get_target_queue",
                &e,
                Some("Check your queue configuration"),
            );
            return None;
        }
    };

    let (from_display, to_display) = validated_operation.get_queue_display_names();
    let loading_template = validated_operation.get_loading_message();
    let context = validated_operation.calculate_post_processing_context();

    start_bulk_send_with_data_operation(
        model,
        messages_data,
        BulkSendParams::new(
            target_queue,
            validated_operation.should_delete_source(),
            &loading_template.replace(&validated_operation.message_ids().len().to_string(), "{}"),
            &from_display,
            &to_display,
        ),
        context,
    )
}

/// Execute bulk send to DLQ operation with deletion (move to DLQ)
pub fn handle_bulk_send_to_dlq_with_delete_execution<T: TerminalAdapter>(
    model: &mut Model<T>,
    message_ids: Vec<MessageIdentifier>,
) -> Option<Msg> {
    // Use BulkOperationSetup for validation and configuration
    let validated_operation = match BulkOperationSetup::new(model, message_ids)
        .operation_type(BulkOperationType::SendToDlq {
            delete_source: true,
        })
        .validate_and_build()
    {
        Ok(op) => op,
        Err(e) => {
            model
                .error_reporter
                .report_simple(e, "BulkSendToDlq", "validation");
            return None;
        }
    };

    // Get target queue from validated operation
    let target_queue = match validated_operation.get_target_queue() {
        Ok(queue) => queue,
        Err(e) => {
            model.error_reporter.report_service_bus_error(
                "get_target_queue",
                &e,
                Some("Check your queue configuration"),
            );
            return None;
        }
    };

    let (from_display, to_display) = validated_operation.get_queue_display_names();
    let loading_template = validated_operation.get_loading_message();
    let context = validated_operation.calculate_post_processing_context();

    let params = BulkSendParams::new(
        target_queue,
        validated_operation.should_delete_source(),
        &loading_template.replace(&validated_operation.message_ids().len().to_string(), "{}"),
        &from_display,
        &to_display,
    );

    start_bulk_send_operation(
        model,
        validated_operation.message_ids().to_vec(),
        params,
        context,
    )
}

/// Extract message data from current state (works for any queue)
fn extract_message_data_from_current_state<T: TerminalAdapter>(
    model: &Model<T>,
    message_ids: &[MessageIdentifier],
) -> Result<Vec<(MessageIdentifier, Vec<u8>)>, bool> {
    let mut messages_data = Vec::new();

    // Get messages from pagination state (these are peeked messages)
    let all_messages = &model.queue_state().message_pagination.all_loaded_messages;

    for message_id in message_ids {
        // Find the message in our loaded state
        if let Some(message) = all_messages.iter().find(|m| m.id == *message_id) {
            // Extract the message body as bytes
            let body = match &message.body {
                BodyData::ValidJson(json) => serde_json::to_vec(json).unwrap_or_default(),
                BodyData::RawString(s) => s.as_bytes().to_vec(),
            };
            messages_data.push((message_id.clone(), body));
            log::debug!("Extracted message data for {}", message_id);
        } else {
            let error = AppError::State(format!(
                "Message {} not found in current state for send operation",
                message_id
            ));
            model
                .error_reporter
                .report_loading_error("BulkSend", "extract_message_data", &error);
            return Err(true);
        }
    }

    log::info!(
        "Extracted data for {} messages for send operation",
        messages_data.len()
    );

    Ok(messages_data)
}

/// Execute bulk send operation with message data (peeked messages)
async fn execute_bulk_send_with_data(
    service_bus_manager: Arc<tokio::sync::Mutex<server::service_bus_manager::ServiceBusManager>>,
    messages_data: &[(MessageIdentifier, Vec<u8>)],
    target_queue: String,
    repeat_count: usize,
    progress: &ProgressReporter,
) -> Result<server::bulk_operations::BulkOperationResult, AppError> {
    progress.report_progress("Preparing message data...");
    let messages_data_converted: Vec<(MessageIdentifier, Vec<u8>)> = messages_data
        .iter()
        .map(|(id, data)| (id.clone(), data.clone()))
        .collect();
    let command = ServiceBusCommand::BulkSendPeeked {
        messages_data: messages_data_converted,
        target_queue,
        repeat_count,
    };
    progress.report_progress("Executing send operation...");
    let response = service_bus_manager
        .lock()
        .await
        .execute_command(command)
        .await;

    match response {
        ServiceBusResponse::MessagesSent { stats, .. } => {
            progress.report_progress(format!(
                "Completed: {} successful, {} failed",
                stats.successful, stats.failed
            ));
            log::info!("Bulk send with data completed successfully: {:?}", stats);

            // Convert OperationStats to BulkOperationResult
            let mut result = server::bulk_operations::BulkOperationResult::new(messages_data.len());
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

/// Execute bulk send operation with message IDs (retrieval-based)
async fn execute_bulk_send_with_ids(
    service_bus_manager: Arc<tokio::sync::Mutex<server::service_bus_manager::ServiceBusManager>>,
    message_ids: &[MessageIdentifier],
    target_queue: String,
    should_delete_source: bool,
    repeat_count: usize,
    max_position: usize,
    progress: &ProgressReporter,
) -> Result<server::bulk_operations::BulkOperationResult, AppError> {
    progress.report_progress("Preparing message IDs...");
    let message_ids_converted: Vec<MessageIdentifier> = message_ids.to_vec();
    let command = ServiceBusCommand::BulkSend {
        message_ids: message_ids_converted,
        target_queue,
        should_delete_source,
        repeat_count,
        max_position,
    };
    progress.report_progress("Executing send operation...");
    let response = service_bus_manager
        .lock()
        .await
        .execute_command(command)
        .await;

    match response {
        ServiceBusResponse::BulkOperationCompleted { result } => {
            progress.report_progress(format!(
                "Completed: {} successful, {} failed",
                result.successful, result.failed
            ));
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

/// Handle successful bulk send operation result
fn handle_bulk_send_success(
    operation_result: server::bulk_operations::BulkOperationResult,
    bulk_data: &BulkSendData,
    operation_params: &BulkSendParams,
    context: &BulkOperationContext,
    tx_to_main: &std::sync::mpsc::Sender<Msg>,
    error_reporter: &crate::error::ErrorReporter,
) -> Result<(), AppError> {
    log::info!(
        "Bulk send operation completed: {} successful, {} failed, {} not found",
        operation_result.successful,
        operation_result.failed,
        operation_result.not_found
    );

    // Extract message IDs for centralized processing
    let message_ids = if operation_params.should_delete && operation_result.successful > 0 {
        BulkOperationPostProcessor::extract_successfully_processed_message_ids(
            bulk_data,
            operation_result.successful,
        )
    } else {
        vec![] // No message IDs needed if not deleting or no successful operations
    };

    // Create context for centralized post-processing with proper values
    let context = BulkOperationPostProcessor::create_send_context(
        &operation_result,
        message_ids,
        context.auto_reload_threshold,
        operation_params.from_queue_display.clone(),
        operation_params.to_queue_display.clone(),
        operation_params.should_delete,
        context.current_message_count,
        context.selected_from_current_page,
    );

    // Use centralized post-processor to handle completion
    BulkOperationPostProcessor::handle_completion(&context, tx_to_main, error_reporter)
}

/// Handle bulk send operation error
fn handle_bulk_send_error(
    error: AppError,
    bulk_data: &BulkSendData,
    operation_params: &BulkSendParams,
    tx_to_main: &std::sync::mpsc::Sender<Msg>,
    error_reporter: &crate::error::ErrorReporter,
) {
    error_reporter.report_bulk_operation_error("send", bulk_data.message_count(), &error);

    // Prepare error message with context
    let context_message = format!(
        "Failed to send messages from {} to {}: {}",
        operation_params.from_queue_display, operation_params.to_queue_display, error
    );

    // Send error message
    if let Err(e) = tx_to_main.send(Msg::Error(AppError::ServiceBus(context_message))) {
        error_reporter.report_send_error("error", &e);
    }
}

/// Generic method to start bulk send operation with either message IDs or pre-fetched data
fn start_bulk_send_generic<T: TerminalAdapter>(
    model: &Model<T>,
    bulk_data: BulkSendData,
    operation_params: BulkSendParams,
    context: BulkOperationContext,
    max_position: usize,
) -> Option<Msg> {
    let service_bus_manager = model.service_bus_manager.clone();
    let loading_message = operation_params
        .loading_message_template
        .replace("{}", &bulk_data.message_count().to_string());
    let tx_to_main = model.tx_to_main().clone();
    let error_reporter = model.error_reporter.clone();
    let repeat_count = model.queue_state().message_repeat_count;

    // Generate unique operation ID for cancellation support
    let operation_id = format!(
        "bulk_send_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    // Use enhanced TaskManager with progress reporting and cancellation
    model.task_manager.execute_with_progress(
        loading_message,
        operation_id,
        move |progress: ProgressReporter| {
            Box::pin(async move {
                log::info!(
                    "Starting enhanced bulk send operation for {} messages",
                    bulk_data.message_count()
                );

                // Report initial progress
                progress.report_progress("Initializing...");

                // Execute the bulk send operation using service bus manager
                let result = match &bulk_data {
                    BulkSendData::MessageData(messages_data) => {
                        execute_bulk_send_with_data(
                            service_bus_manager.clone(),
                            messages_data,
                            operation_params.target_queue.clone(),
                            repeat_count,
                            &progress,
                        )
                        .await
                    }
                    BulkSendData::MessageIds(message_ids) => {
                        execute_bulk_send_with_ids(
                            service_bus_manager.clone(),
                            message_ids,
                            operation_params.target_queue.clone(),
                            operation_params.should_delete,
                            repeat_count,
                            max_position,
                            &progress,
                        )
                        .await
                    }
                };

                progress.report_progress("Finalizing...");

                // Handle the result
                match result {
                    Ok(operation_result) => handle_bulk_send_success(
                        operation_result,
                        &bulk_data,
                        &operation_params,
                        &context,
                        &tx_to_main,
                        &error_reporter,
                    ),
                    Err(error) => {
                        handle_bulk_send_error(
                            error,
                            &bulk_data,
                            &operation_params,
                            &tx_to_main,
                            &error_reporter,
                        );
                        Err(AppError::ServiceBus(
                            "Bulk send operation failed".to_string(),
                        ))
                    }
                }
            })
        },
    );

    None
}

/// Method to start bulk send operation with message retrieval
fn start_bulk_send_operation<T: TerminalAdapter>(
    model: &Model<T>,
    message_ids: Vec<MessageIdentifier>,
    params: BulkSendParams,
    context: BulkOperationContext,
) -> Option<Msg> {
    // Use actual highest selected index if available, otherwise get current message index
    let max_position = if let Some(highest_index) = model
        .queue_state()
        .bulk_selection
        .get_highest_selected_index()
    {
        highest_index
    } else if message_ids.len() == 1 {
        // Single message operation - get current message index from UI state
        if let Ok(tuirealm::State::One(tuirealm::StateValue::Usize(selected_index))) = model
            .app
            .state(&crate::components::common::ComponentId::Messages)
        {
            selected_index + 1 // Convert to 1-based position
        } else {
            // Fallback to page-based estimation
            let page_size = crate::config::get_config_or_panic().max_messages() as usize;
            let current_page = model.queue_state().message_pagination.current_page;
            (current_page + 1) * page_size
        }
    } else {
        // Fallback to page-based estimation
        let page_size = crate::config::get_config_or_panic().max_messages() as usize;
        let current_page = model.queue_state().message_pagination.current_page;
        (current_page + 1) * page_size
    };

    start_bulk_send_generic(
        model,
        BulkSendData::MessageIds(message_ids),
        params,
        context,
        max_position,
    )
}

/// Method to start bulk send operation with pre-fetched message data
fn start_bulk_send_with_data_operation<T: TerminalAdapter>(
    model: &Model<T>,
    messages_data: Vec<(MessageIdentifier, Vec<u8>)>,
    params: BulkSendParams,
    context: BulkOperationContext,
) -> Option<Msg> {
    // Use actual highest selected index if available, otherwise get current message index
    let max_position = if let Some(highest_index) = model
        .queue_state()
        .bulk_selection
        .get_highest_selected_index()
    {
        // get_highest_selected_index now returns 1-based position
        highest_index
    } else if messages_data.len() == 1 {
        // Single message operation - get current message index from UI state
        if let Ok(tuirealm::State::One(tuirealm::StateValue::Usize(selected_index))) = model
            .app
            .state(&crate::components::common::ComponentId::Messages)
        {
            selected_index + 1 // Convert to 1-based position
        } else {
            // Fallback to page-based estimation
            let page_size = crate::config::get_config_or_panic().max_messages() as usize;
            let current_page = model.queue_state().message_pagination.current_page;
            (current_page + 1) * page_size
        }
    } else {
        // Fallback to page-based estimation
        let page_size = crate::config::get_config_or_panic().max_messages() as usize;
        let current_page = model.queue_state().message_pagination.current_page;
        (current_page + 1) * page_size
    };

    start_bulk_send_generic(
        model,
        BulkSendData::MessageData(messages_data),
        params,
        context,
        max_position,
    )
}
