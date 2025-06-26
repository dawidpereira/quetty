use super::operation_setup::{BulkOperationSetup, BulkOperationType};
use super::task_manager::{BulkSendData, BulkSendParams, BulkSendTaskParams, BulkTaskManager};
use crate::app::model::Model;
use crate::components::common::Msg;
use crate::error::AppError;
use server::bulk_operations::MessageIdentifier;
use server::model::BodyData;
use std::sync::Arc;
use tuirealm::terminal::TerminalAdapter;

/// Execute bulk resend from DLQ operation
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

    let params = BulkSendParams::new(
        target_queue,
        validated_operation.should_delete_source(),
        &loading_template.replace(&validated_operation.message_ids().len().to_string(), "{}"),
        &from_display,
        &to_display,
    );

    start_bulk_send_operation(model, validated_operation.message_ids().to_vec(), params)
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

/// Generic method to start bulk send operation with either message IDs or pre-fetched data
fn start_bulk_send_generic<T: TerminalAdapter>(
    model: &Model<T>,
    bulk_data: BulkSendData,
    operation_params: BulkSendParams,
) -> Option<Msg> {
    let task_manager = BulkTaskManager::new(model.taskpool.clone(), model.tx_to_main().clone());

    // Create task parameters
    let task_params = BulkSendTaskParams::new(
        bulk_data,
        operation_params,
        Arc::clone(&model.service_bus_manager),
        model.tx_to_main().clone(),
        model.queue_state().message_repeat_count,
        model.error_reporter.clone(),
    );

    // Execute the task
    task_manager.execute_bulk_send_task(task_params);
    None
}

/// Method to start bulk send operation with message retrieval
fn start_bulk_send_operation<T: TerminalAdapter>(
    model: &Model<T>,
    message_ids: Vec<MessageIdentifier>,
    params: BulkSendParams,
) -> Option<Msg> {
    // Use message IDs for retrieval-based operations (allows deletion)
    start_bulk_send_generic(
        model,
        BulkSendData::MessageIds(message_ids.iter().map(|id| id.to_string()).collect()),
        params,
    )
}

/// Method to start bulk send operation with pre-fetched message data
fn start_bulk_send_with_data_operation<T: TerminalAdapter>(
    model: &Model<T>,
    messages_data: Vec<(MessageIdentifier, Vec<u8>)>,
    params: BulkSendParams,
) -> Option<Msg> {
    start_bulk_send_generic(
        model,
        BulkSendData::MessageData(
            messages_data
                .iter()
                .map(|(id, data)| (id.to_string(), data.to_vec()))
                .collect(),
        ),
        params,
    )
}
