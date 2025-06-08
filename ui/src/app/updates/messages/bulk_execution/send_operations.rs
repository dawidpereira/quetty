use super::task_manager::{
    BulkSendData, BulkSendOperationParams, BulkSendTaskParams, BulkTaskManager, DLQ_DISPLAY_NAME,
    MAIN_QUEUE_DISPLAY_NAME,
};
use super::validation::{validate_bulk_resend_request, validate_bulk_send_to_dlq_request};
use crate::app::model::Model;
use crate::components::common::Msg;
use crate::error::AppError;
use server::bulk_operations::MessageIdentifier;
use server::model::BodyData;
use tuirealm::terminal::TerminalAdapter;

/// Execute bulk resend from DLQ operation
pub fn handle_bulk_resend_from_dlq_execution<T: TerminalAdapter>(
    model: &mut Model<T>,
    message_ids: Vec<MessageIdentifier>,
) -> Option<Msg> {
    if message_ids.is_empty() {
        log::warn!("No messages provided for bulk resend operation");
        return None;
    }

    if let Err(error_msg) = validate_bulk_resend_request(model, &message_ids) {
        return Some(error_msg);
    }

    let consumer = match get_consumer_for_bulk_operation(model) {
        Ok(consumer) => consumer,
        Err(error_msg) => return Some(error_msg),
    };

    // Get the main queue name for DLQ to Main operation
    let target_queue = match get_main_queue_name_from_current_dlq(model) {
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

    start_bulk_send_operation(model, message_ids, params)
}

/// Execute bulk resend-only from DLQ operation (without deleting from DLQ)
pub fn handle_bulk_resend_from_dlq_only_execution<T: TerminalAdapter>(
    model: &mut Model<T>,
    message_ids: Vec<MessageIdentifier>,
) -> Option<Msg> {
    if message_ids.is_empty() {
        log::warn!("No messages provided for bulk resend-only operation");
        return None;
    }

    if let Err(error_msg) = validate_bulk_resend_request(model, &message_ids) {
        return Some(error_msg);
    }

    // For resend-only, we get message data from the current state (peeked messages)
    let messages_data = match extract_message_data_for_resend_only(model, &message_ids) {
        Ok(data) => data,
        Err(error_msg) => return Some(error_msg),
    };

    // Get the main queue name for DLQ to Main operation
    let target_queue = match get_main_queue_name_from_current_dlq(model) {
        Ok(name) => name,
        Err(e) => {
            log::error!("Failed to get main queue name: {}", e);
            return Some(Msg::Error(e));
        }
    };

    let consumer = match get_consumer_for_bulk_operation(model) {
        Ok(consumer) => consumer,
        Err(error_msg) => return Some(error_msg),
    };

    start_bulk_send_with_data_operation(
        model,
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

/// Execute bulk send to DLQ operation
pub fn handle_bulk_send_to_dlq_execution<T: TerminalAdapter>(
    model: &mut Model<T>,
    message_ids: Vec<MessageIdentifier>,
) -> Option<Msg> {
    if message_ids.is_empty() {
        log::warn!("No messages provided for bulk send to DLQ operation");
        return None;
    }

    if let Err(error_msg) = validate_bulk_send_to_dlq_request(model, &message_ids) {
        return Some(error_msg);
    }

    let consumer = match get_consumer_for_bulk_operation(model) {
        Ok(consumer) => consumer,
        Err(error_msg) => return Some(error_msg),
    };

    // Get the DLQ name for Main to DLQ operation
    let target_queue = format!(
        "{}/$deadletterqueue",
        model
            .queue_state
            .current_queue_name
            .as_ref()
            .unwrap_or(&"unknown".to_string())
    );

    let params = BulkSendOperationParams::new(
        consumer,
        target_queue,
        true, // should_delete = true for Main to DLQ
        "Bulk sending {} messages from main queue to DLQ...",
        MAIN_QUEUE_DISPLAY_NAME,
        DLQ_DISPLAY_NAME,
    );

    start_bulk_send_operation(model, message_ids, params)
}

/// Extract message data from current state for resend-only operation
fn extract_message_data_for_resend_only<T: TerminalAdapter>(
    model: &Model<T>,
    message_ids: &[MessageIdentifier],
) -> Result<Vec<(MessageIdentifier, Vec<u8>)>, Msg> {
    let mut messages_data = Vec::new();

    // Get messages from pagination state (these are peeked messages)
    let all_messages = &model.queue_state.message_pagination.all_loaded_messages;

    for message_id in message_ids {
        // Find the message in our loaded state
        if let Some(message) = all_messages
            .iter()
            .find(|m| m.id == message_id.id && m.sequence == message_id.sequence)
        {
            // Extract the message body as bytes
            let body = match &message.body {
                BodyData::ValidJson(json) => serde_json::to_vec(json).unwrap_or_default(),
                BodyData::RawString(s) => s.as_bytes().to_vec(),
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

    log::info!(
        "Extracted data for {} messages for resend-only operation",
        messages_data.len()
    );

    Ok(messages_data)
}

/// Gets the consumer for bulk operations
fn get_consumer_for_bulk_operation<T: TerminalAdapter>(
    model: &Model<T>,
) -> Result<std::sync::Arc<tokio::sync::Mutex<server::consumer::Consumer>>, Msg> {
    match model.queue_state.consumer.clone() {
        Some(consumer) => Ok(consumer),
        None => {
            log::error!("No consumer available for bulk operation");
            Err(Msg::Error(AppError::State(
                "No consumer available for bulk operation".to_string(),
            )))
        }
    }
}

/// Get the main queue name for DLQ to Main operation
fn get_main_queue_name_from_current_dlq<T: TerminalAdapter>(
    model: &Model<T>,
) -> Result<String, AppError> {
    let current_queue_name = model
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

/// Generic method to start bulk send operation with either message IDs or pre-fetched data
fn start_bulk_send_generic<T: TerminalAdapter>(
    model: &Model<T>,
    bulk_data: BulkSendData,
    operation_params: BulkSendOperationParams,
) -> Option<Msg> {
    let task_manager = BulkTaskManager::new(model.taskpool.clone(), model.tx_to_main.clone());

    // Create task parameters
    let task_params = BulkSendTaskParams::new(
        bulk_data,
        operation_params,
        model.service_bus_client.clone(),
        model.tx_to_main.clone(),
    );

    // Execute the task
    task_manager.execute_bulk_send_task(task_params);
    None
}

/// Method to start bulk send operation with message retrieval
fn start_bulk_send_operation<T: TerminalAdapter>(
    model: &Model<T>,
    message_ids: Vec<MessageIdentifier>,
    params: BulkSendOperationParams,
) -> Option<Msg> {
    start_bulk_send_generic(model, BulkSendData::MessageIds(message_ids), params)
}

/// Method to start bulk send operation with pre-fetched message data
fn start_bulk_send_with_data_operation<T: TerminalAdapter>(
    model: &Model<T>,
    messages_data: Vec<(MessageIdentifier, Vec<u8>)>,
    params: BulkSendOperationParams,
) -> Option<Msg> {
    start_bulk_send_generic(model, BulkSendData::MessageData(messages_data), params)
}

