use crate::app::model::Model;
use crate::components::common::QueueType;
use crate::error::AppError;
use server::bulk_operations::MessageIdentifier;
use tuirealm::terminal::TerminalAdapter;

/// Common validation trait for bulk operations
pub trait BulkOperationValidator {
    fn validate_message_ids(&self, message_ids: &[MessageIdentifier]) -> Result<(), bool>;
    fn validate_queue_type(&self, expected_queue_type: QueueType) -> Result<(), bool>;
    fn validate_batch_configuration(
        &self,
        message_ids: &[MessageIdentifier],
    ) -> Result<(), AppError>;
}

impl<T> BulkOperationValidator for Model<T>
where
    T: TerminalAdapter,
{
    /// Validates that message IDs are not empty and within limits
    fn validate_message_ids(&self, message_ids: &[MessageIdentifier]) -> Result<(), bool> {
        use crate::config::CONFIG;

        let count = message_ids.len();
        let min_count = CONFIG.bulk_operations().min_count();
        let max_count = CONFIG.bulk_operations().max_count();

        if count < min_count {
            log::warn!("Insufficient messages for bulk operation: {}", count);
            let error = AppError::State("No messages selected for bulk operation".to_string());
            self.error_reporter
                .report_simple(error, "BulkValidation", "count_check");
            return Err(true);
        }

        if count > max_count {
            log::warn!(
                "Too many messages for bulk operation: {} (max: {})",
                count,
                max_count
            );
            let error = AppError::State(format!(
                "Too many messages selected ({}). Maximum allowed: {} (configured limit)",
                count, max_count
            ));
            self.error_reporter
                .report_simple(error, "BulkValidation", "count_check");
            return Err(true);
        }

        log::debug!(
            "Validated {} messages for bulk operation (limits: {}-{})",
            count,
            min_count,
            max_count
        );
        Ok(())
    }

    /// Validates the current queue type matches the expected type
    fn validate_queue_type(&self, expected_queue_type: QueueType) -> Result<(), bool> {
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

            let error = AppError::State(format!(
                "Operation not allowed: currently in {} but operation requires {}",
                current_type, expected_type
            ));
            self.error_reporter
                .report_simple(error, "BulkValidation", "queue_type");
            return Err(true);
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
) -> Result<(), bool> {
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
) -> Result<(), bool> {
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
) -> Result<(), bool> {
    // Validate basic requirements
    model.validate_message_ids(message_ids)?;

    // Validate batch configuration
    if let Err(e) = model.validate_batch_configuration(message_ids) {
        model
            .error_reporter
            .report_simple(e, "BulkValidation", "batch_config");
        return Err(true);
    }

    // Log operation details
    log::info!(
        "Validated bulk delete request for {} messages from {:?}",
        message_ids.len(),
        model.queue_state.current_queue_type
    );

    Ok(())
}
