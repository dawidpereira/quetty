use crate::app::model::Model;
use crate::components::common::{Msg, QueueType};
use crate::error::AppError;
use server::bulk_operations::MessageIdentifier;
use tuirealm::terminal::TerminalAdapter;

/// Common validation trait for bulk operations
pub trait BulkOperationValidator {
    fn validate_message_ids(&self, message_ids: &[MessageIdentifier]) -> Result<(), Msg>;
    fn validate_queue_type(&self, expected_queue_type: QueueType) -> Result<(), Msg>;
    fn validate_batch_configuration(
        &self,
        message_ids: &[MessageIdentifier],
    ) -> Result<(), AppError>;
}

impl<T> BulkOperationValidator for Model<T>
where
    T: TerminalAdapter,
{
    /// Validates that message IDs are not empty
    fn validate_message_ids(&self, message_ids: &[MessageIdentifier]) -> Result<(), Msg> {
        if message_ids.is_empty() {
            log::warn!("No messages provided for bulk operation");
            return Err(Msg::Error(AppError::State(
                "No messages selected for bulk operation".to_string(),
            )));
        }
        Ok(())
    }

    /// Validates the current queue type matches the expected type
    fn validate_queue_type(&self, expected_queue_type: QueueType) -> Result<(), Msg> {
        if self.queue_state.current_queue_type != expected_queue_type {
            let current_type = match self.queue_state.current_queue_type {
                QueueType::Main => "main queue",
                QueueType::DeadLetter => "dead letter queue",
            };
            let expected_type = match expected_queue_type {
                QueueType::Main => "main queue",
                QueueType::DeadLetter => "dead letter queue",
            };

            log::warn!(
                "Invalid queue type for operation: expected {}, current {}",
                expected_type,
                current_type
            );

            return Err(Msg::Error(AppError::State(format!(
                "Operation not allowed: currently in {} but operation requires {}",
                current_type, expected_type
            ))));
        }
        Ok(())
    }

    /// Validates batch configuration for bulk operations
    fn validate_batch_configuration(
        &self,
        message_ids: &[MessageIdentifier],
    ) -> Result<(), AppError> {
        // Use the configured maximum batch size
        let max_batch_size = crate::config::CONFIG.batch().max_batch_size() as usize;
        if message_ids.len() > max_batch_size {
            return Err(AppError::State(format!(
                "Batch size {} exceeds maximum allowed size of {}",
                message_ids.len(),
                max_batch_size
            )));
        }

        // Log operation details
        log::info!(
            "Validated batch configuration for {} messages",
            message_ids.len()
        );

        Ok(())
    }
}

/// Validates that the bulk resend request is valid
pub fn validate_bulk_resend_request<T: TerminalAdapter>(
    model: &Model<T>,
    message_ids: &[MessageIdentifier],
) -> Result<(), Msg> {
    // Validate basic requirements
    model.validate_message_ids(message_ids)?;

    // Only allow resending from DLQ (not from main queue)
    model.validate_queue_type(QueueType::DeadLetter)?;

    // Always log warning about potential message order changes in bulk operations
    log::warn!(
        "Bulk operation for {} messages may affect message order. Messages may not be processed in their original sequence.",
        message_ids.len()
    );

    log::info!(
        "Validated bulk resend request for {} messages",
        message_ids.len()
    );

    Ok(())
}

/// Validates that the bulk send to DLQ request is valid
pub fn validate_bulk_send_to_dlq_request<T: TerminalAdapter>(
    model: &Model<T>,
    message_ids: &[MessageIdentifier],
) -> Result<(), Msg> {
    // Validate basic requirements
    model.validate_message_ids(message_ids)?;

    // Only allow sending to DLQ from main queue (not from DLQ itself)
    model.validate_queue_type(QueueType::Main)?;

    // Log warning about message order changes
    log::warn!(
        "Bulk send to DLQ for {} messages may affect message order in the main queue",
        message_ids.len()
    );

    log::info!(
        "Validated bulk send to DLQ request for {} messages",
        message_ids.len()
    );

    Ok(())
}

/// Validates that the bulk delete request is valid
pub fn validate_bulk_delete_request<T: TerminalAdapter>(
    model: &Model<T>,
    message_ids: &[MessageIdentifier],
) -> Result<(), Msg> {
    // Validate basic requirements
    model.validate_message_ids(message_ids)?;

    // Validate batch configuration
    if let Err(e) = model.validate_batch_configuration(message_ids) {
        return Err(Msg::Error(e));
    }

    // Log operation details
    log::info!(
        "Validated bulk delete request for {} messages from {:?}",
        message_ids.len(),
        model.queue_state.current_queue_type
    );

    Ok(())
}

