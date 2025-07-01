use super::operation_setup::{BulkOperationSetup, BulkOperationType, BulkOperationValidation};
use crate::app::bulk_operation_processor::BulkOperationPostProcessor;
use crate::app::model::Model;
use crate::app::task_manager::ProgressReporter;
use crate::error::AppError;
use server::bulk_operations::MessageIdentifier;
use server::service_bus_manager::{ServiceBusCommand, ServiceBusResponse};
use tuirealm::terminal::TerminalAdapter;

/// Execute bulk delete operation using simplified setup pattern
pub fn handle_bulk_delete_execution<T: TerminalAdapter>(
    model: &mut Model<T>,
    message_ids: Vec<MessageIdentifier>,
) -> Option<crate::components::common::Msg> {
    // Quick validation for empty list
    if Model::<T>::validate_not_empty(&message_ids).is_err() {
        return None;
    }

    // Use BulkOperationSetup for validation and configuration
    let validated_operation = match BulkOperationSetup::new(model, message_ids)
        .operation_type(BulkOperationType::Delete)
        .validate_and_build()
    {
        Ok(op) => op,
        Err(e) => {
            model
                .error_reporter
                .report_simple(e, "BulkDelete", "validation");
            return None;
        }
    };

    // Get pre-calculated context for post-processing
    let context = validated_operation.calculate_post_processing_context();

    let service_bus_manager = model.service_bus_manager.clone();
    let loading_message = validated_operation.get_loading_message();
    let tx_to_main = model.tx_to_main().clone();
    let error_reporter = model.error_reporter.clone();
    let message_ids = validated_operation.message_ids().to_vec();

    let max_position = context.max_position;
    Model::<T>::log_message_order_warning(message_ids.len(), "delete");

    // Generate unique operation ID for cancellation support
    let operation_id = format!(
        "bulk_delete_{}",
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
                    "Starting enhanced bulk delete operation for {} messages",
                    message_ids.len()
                );

                // Report initial progress
                progress.report_progress("Initializing...");

                // Execute bulk delete using service bus manager with max position
                let command = ServiceBusCommand::BulkDelete {
                    message_ids: message_ids.clone(),
                    max_position,
                };

                progress.report_progress("Executing delete operation...");

                let response = service_bus_manager
                    .lock()
                    .await
                    .execute_command(command)
                    .await;

                let delete_result = match response {
                    ServiceBusResponse::BulkOperationCompleted { result } => {
                        progress.report_progress(format!(
                            "Completed: {} successful, {} failed",
                            result.successful, result.failed
                        ));
                        result
                    }
                    ServiceBusResponse::Error { error } => {
                        log::error!("Bulk delete operation failed: {}", error);
                        return Err(AppError::ServiceBus(error.to_string()));
                    }
                    _ => {
                        return Err(AppError::ServiceBus(
                            "Unexpected response for bulk delete".to_string(),
                        ));
                    }
                };

                log::info!(
                    "Bulk delete completed: {} successful, {} failed",
                    delete_result.successful,
                    delete_result.failed
                );

                progress.report_progress("Finalizing...");

                // Create context for centralized post-processing
                let message_ids_str: Vec<String> =
                    message_ids.iter().map(|id| id.to_string()).collect();
                let post_context = BulkOperationPostProcessor::create_delete_context(
                    &delete_result,
                    message_ids_str,
                    context.auto_reload_threshold,
                    context.current_message_count,
                    context.selected_from_current_page,
                );

                // Use centralized post-processor to handle completion
                BulkOperationPostProcessor::handle_completion(
                    &post_context,
                    &tx_to_main,
                    &error_reporter,
                )?;

                Ok(())
            })
        },
    );

    None
}
