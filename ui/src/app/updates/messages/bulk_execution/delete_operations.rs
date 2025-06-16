use super::display_helpers::format_bulk_delete_success_message;
use super::message_collector::{BatchDeleteContext, MessageCollector};
use super::task_manager::BulkTaskManager;
use crate::app::model::Model;
use crate::components::common::{
    LoadingActivityMsg, MessageActivityMsg, Msg, PopupActivityMsg, QueueType,
};
use crate::error::AppError;
use server::bulk_operations::MessageIdentifier;
use server::consumer::Consumer;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tuirealm::terminal::TerminalAdapter;

/// Execute bulk delete operation
pub fn handle_bulk_delete_execution<T: TerminalAdapter>(
    model: &mut Model<T>,
    message_ids: Vec<MessageIdentifier>,
) -> Option<Msg> {
    if message_ids.is_empty() {
        log::warn!("No message IDs provided for bulk delete");
        return None;
    }

    let consumer = match model.queue_state.consumer.clone() {
        Some(consumer) => consumer,
        None => {
            log::error!("No consumer available for bulk delete operation");
            model.error_reporter.report_simple(
                AppError::State("No consumer available for bulk delete operation".to_string()),
                "BulkDeleteHandler",
                "handle_bulk_delete_execution",
            );
            return None;
        }
    };

    start_bulk_delete_operation(model, message_ids, consumer)
}

/// Start bulk delete operation with proper task management
fn start_bulk_delete_operation<T: TerminalAdapter>(
    model: &Model<T>,
    message_ids: Vec<MessageIdentifier>,
    consumer: Arc<Mutex<Consumer>>,
) -> Option<Msg> {
    let taskpool = &model.taskpool;
    let tx_to_main = model.tx_to_main.clone();
    let queue_type = model.queue_state.current_queue_type.clone();

    // Start loading indicator
    BulkTaskManager::send_message_or_log_error(
        &tx_to_main,
        Msg::LoadingActivity(LoadingActivityMsg::Start(format!(
            "Deleting {} messages...",
            message_ids.len()
        ))),
        "loading start",
    );

    // Clone necessary data for the async task
    let consumer_clone = consumer.clone();
    let message_ids_clone = message_ids.clone();
    let tx_to_main_clone = tx_to_main.clone();

    // Spawn delete task
    taskpool.execute(async move {
        match execute_bulk_delete_operation(consumer_clone, message_ids_clone).await {
            Ok(actually_deleted_ids) => {
                handle_bulk_delete_success(
                    &tx_to_main,
                    &message_ids,
                    &actually_deleted_ids,
                    queue_type,
                );
            }
            Err(error) => {
                handle_bulk_delete_error(&tx_to_main, &tx_to_main_clone, error, queue_type);
            }
        }
    });

    None
}

/// Execute the actual bulk delete operation
async fn execute_bulk_delete_operation(
    consumer: Arc<Mutex<Consumer>>,
    message_ids: Vec<MessageIdentifier>,
) -> Result<Vec<MessageIdentifier>, AppError> {
    let operation_start = std::time::Instant::now();
    log::info!(
        "Starting bulk delete operation for {} messages",
        message_ids.len()
    );

    // Setup batch delete context
    let context = setup_batch_delete_context(&message_ids)?;

    // Collect target message
    let (target_messages, non_target_messages) =
        collect_messages_for_deletion(consumer.clone(), &context).await?;

    // Abandon non-target messages first
    abandon_non_target_messages(consumer.clone(), non_target_messages).await;

    // Perform batch deletion
    let successfully_deleted_ids =
        perform_batch_deletion(consumer, target_messages, &context.target_map).await?;

    // Log results
    log_deletion_results(
        &successfully_deleted_ids,
        &message_ids,
        context.target_count(),
    );

    let duration = operation_start.elapsed();
    log::info!(
        "Bulk delete operation completed in {:?}: {} out of {} messages deleted",
        duration,
        successfully_deleted_ids.len(),
        message_ids.len()
    );

    Ok(successfully_deleted_ids)
}

/// Setup batch delete context with proper configuration
fn setup_batch_delete_context(
    message_ids: &[MessageIdentifier],
) -> Result<BatchDeleteContext, AppError> {
    // Use the application's configured DLQ batch size
    let batch_size = crate::config::CONFIG.dlq().batch_size() as usize;

    BatchDeleteContext::new(message_ids, batch_size)
}

/// Collect messages for deletion using the message collector
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
    let mut collector = MessageCollector::new(context);
    let mut consumer_guard = consumer.lock().await;

    log::info!(
        "Starting message collection for {} target messages with batch size {}",
        collector.target_count(),
        collector.batch_size()
    );

    while !collector.is_complete() && !collector.should_stop() {
        match consumer_guard
            .receive_messages(collector.batch_size())
            .await
        {
            Ok(messages) => {
                let received_count = messages.len();
                log::debug!("Received {} messages in batch", received_count);

                if collector.process_received_messages(messages) {
                    // Collection completed
                    break;
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
    target_map: &HashMap<String, MessageIdentifier>,
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
        track_deleted_messages(&target_messages, target_map, &mut successfully_deleted_ids);
    } else {
        log::warn!("Batch delete failed, falling back to individual deletion");
        perform_individual_deletion(
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
    target_map: &HashMap<String, MessageIdentifier>,
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
    target_map: &HashMap<String, MessageIdentifier>,
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

/// Handle successful bulk delete operation
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
    BulkTaskManager::send_message_or_log_error(
        tx_to_main,
        Msg::LoadingActivity(LoadingActivityMsg::Stop),
        "loading stop",
    );

    // Remove only the messages that were actually deleted from local state
    BulkTaskManager::send_message_or_log_error(
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

    let success_message = format_bulk_delete_success_message(
        actually_deleted_count,
        originally_selected_count,
        queue_name,
    );

    BulkTaskManager::send_message_or_log_error(
        tx_to_main,
        Msg::PopupActivity(PopupActivityMsg::ShowSuccess(success_message)),
        "success popup",
    );
}

/// Handle bulk delete operation errors
fn handle_bulk_delete_error(
    tx_to_main: &Sender<Msg>,
    tx_to_main_err: &Sender<Msg>,
    error: AppError,
    queue_type: QueueType,
) {
    log::error!("Error in bulk delete operation: {}", error);

    // Stop loading indicator
    BulkTaskManager::send_message_or_log_error(
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

    BulkTaskManager::send_message_or_log_error(tx_to_main_err, Msg::Error(enhanced_error), "error");
}
