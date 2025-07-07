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
                let all_current_deleted =
                    context.selected_from_current_page >= context.current_message_count;

                // Only force reload in extreme cases where UI state might be completely invalid
                if all_current_deleted && large_operation {
                    // Both conditions: all current page deleted AND large operation
                    let reason = format!(
                        "Complete current page deletion in large operation ({} messages) - ensuring UI consistency",
                        context.successful_count
                    );
                    ReloadStrategy::ForceReload { reason }
                } else if context.successful_count > 0 {
                    // For all other deletions, use smart local removal with backfill
                    ReloadStrategy::LocalRemoval
                } else {
                    ReloadStrategy::CompletionOnly
                }
            }
            BulkOperationType::Send { should_delete, .. } => {
                if *should_delete && context.successful_count > 0 {
                    // Always prefer local removal for send-with-delete to preserve messages in memory.
                    // The pagination logic will auto-adjust pages and auto-fill if genuinely needed.
                    ReloadStrategy::LocalRemoval
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
        error_reporter: &crate::error::ErrorReporter,
    ) -> Result<(), AppError> {
        let strategy = Self::determine_reload_strategy(context);

        log::info!(
            "Processing bulk operation completion: type={:?}, strategy={:?}",
            context.operation_type,
            strategy
        );

        match strategy {
            ReloadStrategy::ForceReload { reason } => {
                log::info!("Forcing message reload: {reason}");

                // Send reload first
                if let Err(e) = tx_to_main.send(Msg::MessageActivity(
                    MessageActivityMsg::ForceReloadMessages,
                )) {
                    error_reporter.report_send_error("force reload message", &e);
                    return Err(AppError::Component(e.to_string()));
                }

                // Refresh queue statistics after bulk operation
                if let Err(e) = tx_to_main.send(Msg::MessageActivity(
                    MessageActivityMsg::RefreshQueueStatistics,
                )) {
                    error_reporter.report_send_error("refresh queue statistics", &e);
                    // Don't fail the operation if statistics refresh fails
                }

                // Send completion message after reload
                Self::send_completion_message(context, tx_to_main, error_reporter)?;
            }
            ReloadStrategy::LocalRemoval => {
                // Remove from local state first - this preserves existing messages that weren't deleted
                if context.should_remove_from_state && !context.message_ids.is_empty() {
                    log::info!(
                        "Smart local removal: removing {} messages from state while preserving others",
                        context.message_ids.len()
                    );

                    if let Err(e) = tx_to_main.send(Msg::MessageActivity(
                        MessageActivityMsg::BulkRemoveMessagesFromState(
                            context.message_ids.clone(),
                        ),
                    )) {
                        error_reporter.report_send_error("remove messages from state", &e);
                        return Err(AppError::Component(e.to_string()));
                    }
                }

                // Refresh queue statistics after bulk operation
                log::info!("Refreshing queue statistics after smart local removal");
                if let Err(e) = tx_to_main.send(Msg::MessageActivity(
                    MessageActivityMsg::RefreshQueueStatistics,
                )) {
                    error_reporter.report_send_error("refresh queue statistics", &e);
                    // Don't fail the operation if statistics refresh fails
                }

                // Send completion message after local removal
                Self::send_completion_message(context, tx_to_main, error_reporter)?;
            }
            ReloadStrategy::CompletionOnly => {
                // Refresh queue statistics after bulk operation
                if let Err(e) = tx_to_main.send(Msg::MessageActivity(
                    MessageActivityMsg::RefreshQueueStatistics,
                )) {
                    error_reporter.report_send_error("refresh queue statistics", &e);
                    // Don't fail the operation if statistics refresh fails
                }

                // Only send completion message
                Self::send_completion_message(context, tx_to_main, error_reporter)?;
            }
        }

        // After operations that remove messages from the main queue (delete or send-with-delete), ensure selections are cleared
        if matches!(context.operation_type, BulkOperationType::Delete)
            || matches!(
                context.operation_type,
                BulkOperationType::Send {
                    should_delete: true,
                    ..
                }
            )
        {
            if let Err(e) =
                tx_to_main.send(Msg::MessageActivity(MessageActivityMsg::ClearAllSelections))
            {
                error_reporter.report_send_error("clear selections", &e);
            }
        }

        Ok(())
    }

    /// Shared: Format detailed result message for bulk operations (delete, send with delete)
    pub fn format_bulk_operation_result_message(
        operation: &str,
        queue_name: &str,
        successful_count: usize,
        failed_count: usize,
        not_found_count: usize,
        total_count: usize,
        is_delete: bool,
    ) -> String {
        if successful_count == 0 {
            if failed_count > 0 {
                format!(
                    "❌ Bulk {operation} failed: No messages were processed from {queue_name}\n\n\
                    📊 Results:\n\
                    • ❌ Failed: {failed_count} messages\n\
                    • ⚠️  Not found: {not_found_count} messages\n\
                    • 📦 Total requested: {total_count}\n\n\
                    💡 Messages may have been already processed, moved, or deleted by another process."
                )
            } else {
                let unavailable_hint = if is_delete {
                    format!(
                        "💡 The {not_found_count} messages you selected were not available for deletion.\n\
                        This typically happens when:\n\
                        • Messages were processed by another consumer\n\
                        • Messages were moved or deleted by another process\n\
                        • Selected messages are only visible in preview but not available for consumption\n\n\
                        🔄 Try refreshing the queue to see the current available messages."
                    )
                } else {
                    format!(
                        "💡 The {not_found_count} messages you selected were not available for moving.\n\
                        This typically happens when:\n\
                        • Messages were processed by another consumer\n\
                        • Messages were moved or deleted by another process\n\
                        • Selected messages are only visible in preview but not available for consumption\n\n\
                        🔄 Try refreshing the queue to see the current available messages."
                    )
                };
                format!(
                    "⚠️  No messages were processed from {queue_name}

📊 Results:
• ⚠️  Not found: {not_found_count} messages
• 📦 Total requested: {total_count}

{unavailable_hint}"
                )
            }
        } else if failed_count > 0 || not_found_count > 0 {
            // Partial success
            format!(
                "⚠️ Bulk {operation} operation completed with mixed results

{queue_name}


                📊 Results:

                • ✅ Successfully processed: {successful_count} messages

                • ❌ Failed: {failed_count} messages

                • ⚠️  Not found: {not_found_count} messages

                • 📦 Total requested: {total_count}



                💡 Some messages may have been processed by another process during the operation."
            )
        } else {
            // Complete success
            let operation_word = if is_delete { "move" } else { "copy" };
            let past_tense = if is_delete { "moved" } else { "copied" };

            // Convert arrow representation to 'to' wording for the processed line
            let queue_wording = if queue_name.contains('→') {
                queue_name.replace('→', "to")
            } else {
                queue_name.to_string()
            };

            format!(
                "✅ Bulk {op} operation completed successfully!\n\n{count} message{plural} processed from {queue_wording}\n\nAll messages {past_tense} successfully",
                op = operation_word,
                count = successful_count,
                plural = if successful_count == 1 { "" } else { "s" },
                queue_wording = queue_wording,
                past_tense = past_tense,
            )
        }
    }

    /// Send the appropriate completion message for the operation type
    fn send_completion_message(
        context: &BulkOperationContext,
        tx_to_main: &Sender<Msg>,
        error_reporter: &crate::error::ErrorReporter,
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
                    error_reporter.report_send_error("bulk delete completion message", &e);
                    return Err(AppError::Component(e.to_string()));
                }
            }
            BulkOperationType::Send {
                from_queue_display,
                to_queue_display,
                should_delete,
            } => {
                // Use detailed message if should_delete (move), else fallback to old summary
                let not_found_count = context
                    .total_count
                    .saturating_sub(context.successful_count + context.failed_count);
                let queue_name_combined = format!("{from_queue_display} → {to_queue_display}");
                let operation = if *should_delete { "move" } else { "copy" };
                let is_delete = *should_delete;
                let message = Self::format_bulk_operation_result_message(
                    operation,
                    &queue_name_combined,
                    context.successful_count,
                    context.failed_count,
                    not_found_count,
                    context.total_count,
                    is_delete,
                );
                if let Err(e) =
                    tx_to_main.send(Msg::PopupActivity(PopupActivityMsg::ShowSuccess(message)))
                {
                    error_reporter.report_send_error("success popup message", &e);
                    return Err(AppError::Component(e.to_string()));
                }
            }
        }
        Ok(())
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
    #[allow(clippy::too_many_arguments)]
    pub fn create_send_context(
        result: &BulkOperationResult,
        message_ids_to_remove: Vec<String>,
        reload_threshold: usize,
        from_queue_display: String,
        to_queue_display: String,
        should_delete: bool,
        current_message_count: usize,
        selected_from_current_page: usize,
    ) -> BulkOperationContext {
        BulkOperationContext {
            operation_type: BulkOperationType::Send {
                from_queue_display,
                to_queue_display,
                should_delete,
            },
            successful_count: result.successful,
            failed_count: result.failed,
            total_count: result.total_requested,
            message_ids: message_ids_to_remove,
            should_remove_from_state: should_delete,
            reload_threshold,
            current_message_count,
            selected_from_current_page,
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
                message_ids
                    .iter()
                    .take(successful_count)
                    .map(|id| id.id.clone())
                    .collect()
            }
            BulkSendData::MessageData(messages_data) => {
                // Extract message IDs from the message data
                messages_data
                    .iter()
                    .take(successful_count)
                    .map(|(id, _)| id.id.clone())
                    .collect()
            }
        }
    }

    /// Convenience wrapper retained for test compatibility.
    /// Generates a user-facing summary for bulk send operations (copy/move).
    #[allow(clippy::too_many_arguments)]
    #[allow(dead_code)]
    pub fn format_send_success_message(
        successful_count: usize,
        failed_count: usize,
        total_count: usize,
        from_queue: &str,
        to_queue: &str,
        is_delete: bool,
    ) -> String {
        let not_found_count = total_count.saturating_sub(successful_count + failed_count);
        let operation = if is_delete { "move" } else { "copy" };
        let combined_queue = format!("{from_queue} → {to_queue}");

        Self::format_bulk_operation_result_message(
            operation,
            &combined_queue,
            successful_count,
            failed_count,
            not_found_count,
            total_count,
            is_delete,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delete_strategy_large_operation_with_all_current_deleted() {
        // This represents the only case where ForceReload should be used for deletes:
        // Large operation AND all current page messages deleted
        let context = BulkOperationContext {
            operation_type: BulkOperationType::Delete,
            successful_count: 100,
            failed_count: 0,
            total_count: 100,
            message_ids: vec![],
            should_remove_from_state: true,
            reload_threshold: 50,
            current_message_count: 20,
            selected_from_current_page: 20, // All current messages deleted
        };

        match BulkOperationPostProcessor::determine_reload_strategy(&context) {
            ReloadStrategy::ForceReload { reason } => {
                assert!(reason.contains("Complete current page deletion in large operation"));
            }
            _ => panic!(
                "Expected ForceReload strategy for large operation that deletes all current page messages"
            ),
        }
    }

    #[test]
    fn test_delete_strategy_large_operation_partial_current() {
        // Large operation but not all current page deleted -> LocalRemoval (smart)
        let context = BulkOperationContext {
            operation_type: BulkOperationType::Delete,
            successful_count: 100,
            failed_count: 0,
            total_count: 100,
            message_ids: vec![],
            should_remove_from_state: true,
            reload_threshold: 50,
            current_message_count: 20,
            selected_from_current_page: 10, // Only partial current page deleted
        };

        match BulkOperationPostProcessor::determine_reload_strategy(&context) {
            ReloadStrategy::LocalRemoval => {}
            _ => panic!(
                "Expected LocalRemoval strategy for large operation that preserves some current page messages"
            ),
        }
    }

    #[test]
    fn test_delete_strategy_small_operation_all_current_deleted() {
        // Small operation but all current deleted -> LocalRemoval (smart)
        let context = BulkOperationContext {
            operation_type: BulkOperationType::Delete,
            successful_count: 5,
            failed_count: 0,
            total_count: 5,
            message_ids: vec![],
            should_remove_from_state: true,
            reload_threshold: 50,
            current_message_count: 5,
            selected_from_current_page: 5, // All current deleted but small operation
        };

        match BulkOperationPostProcessor::determine_reload_strategy(&context) {
            ReloadStrategy::LocalRemoval => {}
            _ => panic!(
                "Expected LocalRemoval strategy for small operation (even if all current deleted)"
            ),
        }
    }

    #[test]
    fn test_delete_strategy_small_local_removal() {
        // Typical small delete operation -> LocalRemoval
        let context = BulkOperationContext {
            operation_type: BulkOperationType::Delete,
            successful_count: 3,
            failed_count: 0,
            total_count: 3,
            message_ids: vec!["1".to_string(), "2".to_string(), "3".to_string()],
            should_remove_from_state: true,
            reload_threshold: 50,
            current_message_count: 20,
            selected_from_current_page: 3,
        };

        match BulkOperationPostProcessor::determine_reload_strategy(&context) {
            ReloadStrategy::LocalRemoval => {}
            _ => panic!("Expected LocalRemoval strategy for typical small operation"),
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
            successful_count: 2000,
            failed_count: 0,
            total_count: 2000,
            message_ids: vec![],
            should_remove_from_state: true,
            reload_threshold: 50,
            current_message_count: 1000,
            selected_from_current_page: 1000,
        };

        match BulkOperationPostProcessor::determine_reload_strategy(&context) {
            ReloadStrategy::LocalRemoval => {}
            _ => panic!("Expected LocalRemoval strategy for large send operation with delete"),
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
            current_message_count: 1000,
            selected_from_current_page: 50,
        };

        match BulkOperationPostProcessor::determine_reload_strategy(&context) {
            ReloadStrategy::CompletionOnly => {}
            _ => panic!("Expected CompletionOnly strategy for copy operation"),
        }
    }

    #[test]
    fn test_send_strategy_local_removal() {
        // This represents your scenario: 2000 messages deleted from mixed pages (not all current page)
        let context = BulkOperationContext {
            operation_type: BulkOperationType::Send {
                from_queue_display: "Main".to_string(),
                to_queue_display: "DLQ".to_string(),
                should_delete: true,
            },
            successful_count: 2000,
            failed_count: 0,
            total_count: 2000,
            message_ids: vec![],
            should_remove_from_state: true,
            reload_threshold: 50,
            current_message_count: 3000, // 3000 total messages loaded
            selected_from_current_page: 1000, // Only 1000 from current page (not all current page deleted)
        };

        match BulkOperationPostProcessor::determine_reload_strategy(&context) {
            ReloadStrategy::LocalRemoval => {}
            _ => panic!(
                "Expected LocalRemoval strategy for large send operation that doesn't move entire current page"
            ),
        }
    }

    #[test]
    fn test_format_send_success_message_full_success() {
        let message =
            BulkOperationPostProcessor::format_send_success_message(10, 0, 10, "Main", "DLQ", true);

        assert!(message.contains("✅ Bulk move operation completed successfully!"));
        assert!(message.contains("10 messages processed from Main to DLQ"));
        assert!(message.contains("moved successfully"));
    }

    #[test]
    fn test_format_send_success_message_partial_success() {
        let message =
            BulkOperationPostProcessor::format_send_success_message(7, 2, 10, "Main", "DLQ", false);

        assert!(message.contains("Bulk copy operation completed with mixed results"));
        assert!(message.contains("✅ Successfully processed: 7 messages"));
        assert!(message.contains("❌ Failed: 2 messages"));
        assert!(message.contains("⚠️  Not found: 1 messages"));
        assert!(message.contains("Main → DLQ"));
    }
}
