use crate::app::bulk_operation_processor::BulkOperationPostProcessor;
use crate::app::model::Model;
use crate::error::AppError;
use server::bulk_operations::MessageIdentifier;
use server::service_bus_manager::{ServiceBusCommand, ServiceBusResponse};
use tuirealm::terminal::TerminalAdapter;

/// Execute bulk delete operation using a simplified approach
pub fn handle_bulk_delete_execution<T: TerminalAdapter>(
    model: &mut Model<T>,
    message_ids: Vec<MessageIdentifier>,
) -> Option<crate::components::common::Msg> {
    if message_ids.is_empty() {
        log::warn!("No message IDs provided for bulk delete");
        return None;
    }

    // Validate the bulk delete request
    if super::validation::validate_bulk_delete_request(model, &message_ids).is_err() {
        return None;
    }

    // Calculate if this will delete all current messages (for auto-reload logic)
    let current_message_count = model
        .queue_state
        .message_pagination
        .get_current_page_messages(crate::config::get_config_or_panic().max_messages())
        .len();
    let selected_from_current_page = message_ids
        .iter()
        .filter(|msg_id| {
            model
                .queue_state
                .message_pagination
                .all_loaded_messages
                .iter()
                .any(|loaded_msg| loaded_msg.id == **msg_id)
        })
        .count();

    let service_bus_manager = model.service_bus_manager.clone();
    let loading_message = format!("Deleting {} messages...", message_ids.len());
    let tx_to_main = model.tx_to_main.clone();
    let auto_reload_threshold = crate::config::get_config_or_panic()
        .batch()
        .auto_reload_threshold();

    // Use TaskManager for proper loading management
    model.task_manager.execute(loading_message, async move {
        log::info!(
            "Starting bulk delete operation for {} messages",
            message_ids.len()
        );

        // Execute bulk delete using service bus manager
        let command = ServiceBusCommand::BulkDelete {
            message_ids: message_ids.clone(),
        };

        let response = service_bus_manager
            .lock()
            .await
            .execute_command(command)
            .await;

        let delete_result = match response {
            ServiceBusResponse::BulkOperationCompleted { result } => result,
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

        // Create context for centralized post-processing
        let message_ids_str: Vec<String> = message_ids.iter().map(|id| id.to_string()).collect();
        let context = BulkOperationPostProcessor::create_delete_context(
            &delete_result,
            message_ids_str,
            auto_reload_threshold,
            current_message_count,
            selected_from_current_page,
        );

        // Use centralized post-processor to handle completion
        BulkOperationPostProcessor::handle_completion(&context, &tx_to_main)?;

        Ok(())
    });

    None
}
