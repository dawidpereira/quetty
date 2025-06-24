use crate::components::common::{MessageActivityMsg, Msg, PopupActivityMsg};
use crate::error::AppError;
use server::bulk_operations::BulkOperationResult;
use std::sync::mpsc::Sender;

/// Context for bulk operation completion handling
#[derive(Debug, Clone)]
pub struct BulkOperationContext {
    pub operation_type: BulkOperationType,
    pub successful_count: usize,
    pub failed_count: usize,
    pub total_count: usize,
    pub message_ids: Vec<String>,
    pub should_remove_from_state: bool,
    pub reload_threshold: usize,
    pub current_message_count: usize,
    pub selected_from_current_page: usize,
}

/// Type of bulk operation being performed
#[derive(Debug, Clone)]
pub enum BulkOperationType {
    Delete,
    Send {
        from_queue_display: String,
        to_queue_display: String,
        should_delete: bool,
    },
}

/// Strategy for handling completion of bulk operations
#[derive(Debug, Clone)]
pub enum ReloadStrategy {
    /// Force reload and show completion message after
    ForceReload { reason: String },
    /// Remove from local state and show completion message
    LocalRemoval,
    /// Only show completion message (no state changes)
    CompletionOnly,
}

/// Centralized bulk operation post-processor
pub struct BulkOperationPostProcessor;

impl BulkOperationPostProcessor {
    /// Determine the appropriate reload strategy for a bulk operation
    pub fn determine_reload_strategy(context: &BulkOperationContext) -> ReloadStrategy {
        let large_operation = context.successful_count >= context.reload_threshold;
        
        match &context.operation_type {
            BulkOperationType::Delete => {
                let all_current_deleted = context.selected_from_current_page >= context.current_message_count;
                
                if large_operation || all_current_deleted {
                    let reason = if large_operation && all_current_deleted {
                        format!("Large deletion ({} messages) and all current messages deleted", context.successful_count)
                    } else if large_operation {
                        format!("Large deletion ({} messages >= threshold {})", context.successful_count, context.reload_threshold)
                    } else {
                        format!("All current messages deleted ({}/{})", context.selected_from_current_page, context.current_message_count)
                    };
                    ReloadStrategy::ForceReload { reason }
                } else if context.successful_count > 0 {
                    ReloadStrategy::LocalRemoval
                } else {
                    ReloadStrategy::CompletionOnly
                }
            }
            BulkOperationType::Send { should_delete, .. } => {
                if *should_delete && context.successful_count > 0 {
                    if large_operation {
                        let reason = format!("Large bulk send operation ({} messages >= threshold {})", context.successful_count, context.reload_threshold);
                        ReloadStrategy::ForceReload { reason }
                    } else {
                        ReloadStrategy::LocalRemoval
                    }
                } else {
                    ReloadStrategy::CompletionOnly
                }
            }
        }
    }

    /// Handle bulk operation completion with appropriate reload strategy
    pub fn handle_completion(
        context: &BulkOperationContext,
        tx_to_main: &Sender<Msg>,
    ) -> Result<(), AppError> {
        let strategy = Self::determine_reload_strategy(context);
        
        log::info!(
            "Processing bulk operation completion: type={:?}, strategy={:?}",
            context.operation_type, strategy
        );

        match strategy {
            ReloadStrategy::ForceReload { reason } => {
                log::info!("Forcing message reload: {}", reason);
                
                // Send reload first
                if let Err(e) = tx_to_main.send(Msg::MessageActivity(MessageActivityMsg::ForceReloadMessages)) {
                    log::error!("Failed to send force reload message: {}", e);
                    return Err(AppError::Component(e.to_string()));
                }
                
                // Send completion message after reload
                Self::send_completion_message(context, tx_to_main)?;
            }
            ReloadStrategy::LocalRemoval => {
                // Remove from local state first
                if context.should_remove_from_state && !context.message_ids.is_empty() {
                    log::info!(
                        "Removing {} messages from local state after {} successful operations",
                        context.message_ids.len(),
                        context.successful_count
                    );
                    
                    if let Err(e) = tx_to_main.send(Msg::MessageActivity(
                        MessageActivityMsg::BulkRemoveMessagesFromState(context.message_ids.clone()),
                    )) {
                        log::error!("Failed to send remove messages from state: {}", e);
                        return Err(AppError::Component(e.to_string()));
                    }
                }
                
                // Send completion message after removal
                Self::send_completion_message(context, tx_to_main)?;
            }
            ReloadStrategy::CompletionOnly => {
                // Only send completion message
                Self::send_completion_message(context, tx_to_main)?;
            }
        }

        Ok(())
    }

    /// Send the appropriate completion message for the operation type
    fn send_completion_message(
        context: &BulkOperationContext,
        tx_to_main: &Sender<Msg>,
    ) -> Result<(), AppError> {
        match &context.operation_type {
            BulkOperationType::Delete => {
                if let Err(e) = tx_to_main.send(Msg::MessageActivity(
                    MessageActivityMsg::BulkDeleteCompleted {
                        successful_count: context.successful_count,
                        failed_count: context.failed_count,
                        total_count: context.total_count,
                    },
                )) {
                    log::error!("Failed to send bulk delete completion message: {}", e);
                    return Err(AppError::Component(e.to_string()));
                }
            }
            BulkOperationType::Send { from_queue_display, to_queue_display, should_delete } => {
                let success_message = Self::format_send_success_message(
                    context.successful_count,
                    context.failed_count,
                    context.total_count,
                    from_queue_display,
                    to_queue_display,
                    *should_delete,
                );
                
                if let Err(e) = tx_to_main.send(Msg::PopupActivity(
                    PopupActivityMsg::ShowSuccess(success_message),
                )) {
                    log::error!("Failed to send success popup message: {}", e);
                    return Err(AppError::Component(e.to_string()));
                }
            }
        }

        Ok(())
    }

    /// Format success message for bulk send operations
    fn format_send_success_message(
        successful_count: usize,
        failed_count: usize,
        total_count: usize,
        from_queue_display: &str,
        to_queue_display: &str,
        should_delete: bool,
    ) -> String {
        let not_found_count = total_count.saturating_sub(successful_count + failed_count);
        
        if failed_count > 0 || not_found_count > 0 {
            // Partial success case
            format!(
                "Bulk {} operation completed with mixed results:\n\n\
                ✅ Successfully processed: {} messages\n\
                ❌ Failed: {} messages\n\
                ⚠️  Not found: {} messages\n\n\
                Direction: {} → {}",
                if should_delete { "move" } else { "copy" },
                successful_count,
                failed_count,
                not_found_count,
                from_queue_display,
                to_queue_display
            )
        } else {
            // Full success case
            format!(
                "✅ Bulk {} operation completed successfully!\n\n\
                📦 {} messages processed from {} to {}\n\n\
                All messages were {} successfully.",
                if should_delete { "move" } else { "copy" },
                successful_count,
                from_queue_display,
                to_queue_display,
                if should_delete { "moved" } else { "copied" }
            )
        }
    }

    /// Create context from bulk operation result for delete operations
    pub fn create_delete_context(
        result: &BulkOperationResult,
        message_ids: Vec<String>,
        reload_threshold: usize,
        current_message_count: usize,
        selected_from_current_page: usize,
    ) -> BulkOperationContext {
        BulkOperationContext {
            operation_type: BulkOperationType::Delete,
            successful_count: result.successful,
            failed_count: result.failed,
            total_count: message_ids.len(),
            message_ids,
            should_remove_from_state: true,
            reload_threshold,
            current_message_count,
            selected_from_current_page,
        }
    }

    /// Create context from bulk operation result for send operations
    pub fn create_send_context(
        result: &BulkOperationResult,
        message_ids: Vec<String>,
        reload_threshold: usize,
        from_queue_display: String,
        to_queue_display: String,
        should_delete: bool,
    ) -> BulkOperationContext {
        BulkOperationContext {
            operation_type: BulkOperationType::Send {
                from_queue_display,
                to_queue_display,
                should_delete,
            },
            successful_count: result.successful,
            failed_count: result.failed,
            total_count: message_ids.len(),
            message_ids,
            should_remove_from_state: should_delete,
            reload_threshold,
            current_message_count: 0, // Not used for send operations
            selected_from_current_page: 0, // Not used for send operations
        }
    }

    /// Extract message IDs that were successfully processed for removal from local state
    pub fn extract_successfully_processed_message_ids(
        bulk_data: &crate::app::updates::messages::bulk_execution::task_manager::BulkSendData,
        successful_count: usize,
    ) -> Vec<String> {
        use crate::app::updates::messages::bulk_execution::task_manager::BulkSendData;
        
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delete_strategy_large_operation() {
        let context = BulkOperationContext {
            operation_type: BulkOperationType::Delete,
            successful_count: 50,
            failed_count: 0,
            total_count: 50,
            message_ids: vec![],
            should_remove_from_state: true,
            reload_threshold: 10,
            current_message_count: 20,
            selected_from_current_page: 5,
        };
        
        match BulkOperationPostProcessor::determine_reload_strategy(&context) {
            ReloadStrategy::ForceReload { reason } => {
                assert!(reason.contains("Large deletion (50 messages >= threshold 10)"));
            }
            _ => panic!("Expected ForceReload strategy for large operation"),
        }
    }

    #[test]
    fn test_delete_strategy_all_current_deleted() {
        let context = BulkOperationContext {
            operation_type: BulkOperationType::Delete,
            successful_count: 5,
            failed_count: 0,
            total_count: 5,
            message_ids: vec![],
            should_remove_from_state: true,
            reload_threshold: 10,
            current_message_count: 5,
            selected_from_current_page: 5,
        };
        
        match BulkOperationPostProcessor::determine_reload_strategy(&context) {
            ReloadStrategy::ForceReload { reason } => {
                assert!(reason.contains("All current messages deleted (5/5)"));
            }
            _ => panic!("Expected ForceReload strategy when all current messages deleted"),
        }
    }

    #[test]
    fn test_delete_strategy_local_removal() {
        let context = BulkOperationContext {
            operation_type: BulkOperationType::Delete,
            successful_count: 3,
            failed_count: 0,
            total_count: 3,
            message_ids: vec!["1".to_string(), "2".to_string(), "3".to_string()],
            should_remove_from_state: true,
            reload_threshold: 10,
            current_message_count: 20,
            selected_from_current_page: 3,
        };
        
        match BulkOperationPostProcessor::determine_reload_strategy(&context) {
            ReloadStrategy::LocalRemoval => {}
            _ => panic!("Expected LocalRemoval strategy for small operation"),
        }
    }

    #[test]
    fn test_send_strategy_large_move() {
        let context = BulkOperationContext {
            operation_type: BulkOperationType::Send {
                from_queue_display: "Main".to_string(),
                to_queue_display: "DLQ".to_string(),
                should_delete: true,
            },
            successful_count: 50,
            failed_count: 0,
            total_count: 50,
            message_ids: vec![],
            should_remove_from_state: true,
            reload_threshold: 10,
            current_message_count: 0,
            selected_from_current_page: 0,
        };
        
        match BulkOperationPostProcessor::determine_reload_strategy(&context) {
            ReloadStrategy::ForceReload { reason } => {
                assert!(reason.contains("Large bulk send operation (50 messages >= threshold 10)"));
            }
            _ => panic!("Expected ForceReload strategy for large send operation"),
        }
    }

    #[test]
    fn test_send_strategy_copy_only() {
        let context = BulkOperationContext {
            operation_type: BulkOperationType::Send {
                from_queue_display: "Main".to_string(),
                to_queue_display: "Other".to_string(),
                should_delete: false, // Copy operation
            },
            successful_count: 50,
            failed_count: 0,
            total_count: 50,
            message_ids: vec![],
            should_remove_from_state: false,
            reload_threshold: 10,
            current_message_count: 0,
            selected_from_current_page: 0,
        };
        
        match BulkOperationPostProcessor::determine_reload_strategy(&context) {
            ReloadStrategy::CompletionOnly => {}
            _ => panic!("Expected CompletionOnly strategy for copy operation"),
        }
    }

    #[test]
    fn test_format_send_success_message_full_success() {
        let message = BulkOperationPostProcessor::format_send_success_message(
            10, 0, 10, "Main", "DLQ", true
        );
        
        assert!(message.contains("✅ Bulk move operation completed successfully!"));
        assert!(message.contains("10 messages processed from Main to DLQ"));
        assert!(message.contains("moved successfully"));
    }

    #[test]
    fn test_format_send_success_message_partial_success() {
        let message = BulkOperationPostProcessor::format_send_success_message(
            7, 2, 10, "Main", "DLQ", false
        );
        
        assert!(message.contains("Bulk copy operation completed with mixed results"));
        assert!(message.contains("✅ Successfully processed: 7 messages"));
        assert!(message.contains("❌ Failed: 2 messages"));
        assert!(message.contains("⚠️  Not found: 1 messages"));
        assert!(message.contains("Main → DLQ"));
    }
} 