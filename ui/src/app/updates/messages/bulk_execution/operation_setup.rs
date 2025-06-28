//! Bulk Operation Setup and Builder Patterns
//!
//! This module provides simplified, reusable patterns for setting up bulk operations,
//! reducing code duplication and complex parameter lists.

use crate::app::model::Model;
use crate::error::AppError;
use server::bulk_operations::MessageIdentifier;
use server::service_bus_manager::QueueType;
use tuirealm::terminal::TerminalAdapter;

/// Common bulk operation configuration
#[derive(Debug, Clone)]
pub struct BulkOperationConfig {
    /// Minimum number of messages required for operation
    pub min_count: usize,
    /// Maximum number of messages allowed for operation
    pub max_count: usize,
    /// Maximum batch size for processing
    pub max_batch_size: usize,
    /// Auto-reload threshold for UI refresh
    pub auto_reload_threshold: usize,
}

impl BulkOperationConfig {
    /// Create configuration from app config
    pub fn from_app_config() -> Self {
        let config = crate::config::get_config_or_panic();
        Self {
            min_count: config.batch().bulk_operation_min_count(),
            max_count: config.batch().bulk_operation_max_count(),
            max_batch_size: config.batch().max_batch_size() as usize,
            auto_reload_threshold: config.batch().auto_reload_threshold(),
        }
    }
}

/// Simplified builder for bulk operations with common validation
///
/// # Safety
/// This struct uses a raw pointer to the Model to avoid lifetime complications
/// during bulk operation setup. The safety guarantees are:
///
/// 1. The pointer is only valid during the scope of the bulk operation setup
/// 2. The Model reference must outlive the BulkOperationSetup instance
/// 3. The pointer is converted back to a reference only during validation and execution
/// 4. No mutable access through the pointer - only immutable access for validation
///
/// ## Usage Pattern
/// ```ignore
/// let setup = BulkOperationSetup::new(&model, message_ids)
///     .operation_type(BulkOperationType::Delete)
///     .validate_and_build()?; // Raw pointer used only here
/// // setup is consumed, no further pointer access
/// ```
pub struct BulkOperationSetup<T: TerminalAdapter> {
    /// Raw pointer to avoid lifetime parameters - see safety documentation above
    model: *const Model<T>,
    config: BulkOperationConfig,
    message_ids: Vec<MessageIdentifier>,
    operation_type: BulkOperationType,
}

#[derive(Debug, Clone)]
pub enum BulkOperationType {
    /// Delete messages from current queue
    Delete,
    /// Resend from DLQ to main queue (with deletion)
    ResendFromDlq { delete_source: bool },
    /// Send to DLQ from main queue (with deletion)
    SendToDlq { delete_source: bool },
}

impl<T: TerminalAdapter> BulkOperationSetup<T> {
    /// Create a new bulk operation setup
    pub fn new(model: &Model<T>, message_ids: Vec<MessageIdentifier>) -> Self {
        Self {
            model,
            config: BulkOperationConfig::from_app_config(),
            message_ids,
            operation_type: BulkOperationType::Delete, // Default
        }
    }

    /// Set the operation type
    pub fn operation_type(mut self, op_type: BulkOperationType) -> Self {
        self.operation_type = op_type;
        self
    }

    /// Validate the operation setup and return prepared context
    pub fn validate_and_build(self) -> Result<ValidatedBulkOperation<T>, AppError> {
        let model = unsafe { &*self.model };

        // Validate basic requirements
        self.validate_message_count()?;
        self.validate_queue_type_for_operation(model)?;
        self.validate_batch_size()?;

        // Log validation success
        log::info!(
            "Validated bulk {:?} operation for {} messages",
            self.operation_type,
            self.message_ids.len()
        );

        Ok(ValidatedBulkOperation {
            model: self.model,
            config: self.config,
            message_ids: self.message_ids,
            operation_type: self.operation_type,
        })
    }

    /// Validate message count is within limits
    fn validate_message_count(&self) -> Result<(), AppError> {
        let count = self.message_ids.len();

        if count == 0 {
            return Err(AppError::State(
                "No messages selected for bulk operation".to_string(),
            ));
        }

        if count < self.config.min_count {
            return Err(AppError::State(format!(
                "Insufficient messages for bulk operation: {} (minimum: {})",
                count, self.config.min_count
            )));
        }

        if count > self.config.max_count {
            return Err(AppError::State(format!(
                "Too many messages selected: {} (maximum: {})",
                count, self.config.max_count
            )));
        }

        Ok(())
    }

    /// Validate queue type matches operation requirements
    fn validate_queue_type_for_operation(&self, model: &Model<T>) -> Result<(), AppError> {
        let current_type = model.queue_manager.queue_state.current_queue_type.clone();
        let required_type = self.get_required_queue_type();

        if current_type != required_type {
            let current_name = queue_type_display_name(current_type);
            let required_name = queue_type_display_name(required_type);

            return Err(AppError::State(format!(
                "Operation not allowed: currently in {} but operation requires {}",
                current_name, required_name
            )));
        }

        Ok(())
    }

    /// Validate batch size is within limits
    fn validate_batch_size(&self) -> Result<(), AppError> {
        if self.message_ids.len() > self.config.max_batch_size {
            return Err(AppError::State(format!(
                "Batch size {} exceeds maximum allowed size of {}",
                self.message_ids.len(),
                self.config.max_batch_size
            )));
        }
        Ok(())
    }

    /// Get required queue type for the operation
    fn get_required_queue_type(&self) -> QueueType {
        match self.operation_type {
            BulkOperationType::Delete => {
                // Delete can work from either queue type
                // We'll use the current queue type (no restriction)
                unsafe { &*self.model }
                    .queue_manager
                    .queue_state
                    .current_queue_type
                    .clone()
            }
            BulkOperationType::ResendFromDlq { .. } => QueueType::DeadLetter,
            BulkOperationType::SendToDlq { .. } => QueueType::Main,
        }
    }
}

/// Validated bulk operation ready for execution
///
/// # Safety
/// Contains a validated raw pointer from BulkOperationSetup. The same safety
/// guarantees apply - the Model reference must outlive this struct's usage.
/// Access is controlled through safe methods that convert the pointer to references.
pub struct ValidatedBulkOperation<T: TerminalAdapter> {
    /// Raw pointer inherited from BulkOperationSetup - see safety documentation
    model: *const Model<T>,
    config: BulkOperationConfig,
    message_ids: Vec<MessageIdentifier>,
    operation_type: BulkOperationType,
}

impl<T: TerminalAdapter> ValidatedBulkOperation<T> {
    /// Get the message IDs for this operation
    pub fn message_ids(&self) -> &[MessageIdentifier] {
        &self.message_ids
    }

    /// Get the model reference (unsafe but controlled)
    pub fn model(&self) -> &Model<T> {
        unsafe { &*self.model }
    }

    /// Get loading message for this operation
    pub fn get_loading_message(&self) -> String {
        let count = self.message_ids.len();
        match &self.operation_type {
            BulkOperationType::Delete => format!("Deleting {} messages...", count),
            BulkOperationType::ResendFromDlq {
                delete_source: true,
            } => {
                format!(
                    "Bulk resending {} messages from DLQ to main queue...",
                    count
                )
            }
            BulkOperationType::ResendFromDlq {
                delete_source: false,
            } => {
                format!(
                    "Bulk copying {} messages from DLQ to main queue (keeping in DLQ)...",
                    count
                )
            }
            BulkOperationType::SendToDlq {
                delete_source: true,
            } => {
                format!("Bulk moving {} messages from main queue to DLQ...", count)
            }
            BulkOperationType::SendToDlq {
                delete_source: false,
            } => {
                format!("Bulk copying {} messages from main queue to DLQ...", count)
            }
        }
    }

    /// Get target queue name for send operations
    pub fn get_target_queue(&self) -> Result<String, AppError> {
        let model = self.model();
        let current_queue_name = model
            .queue_state()
            .current_queue_name
            .as_ref()
            .ok_or_else(|| AppError::State("No current queue name available".to_string()))?;

        match &self.operation_type {
            BulkOperationType::Delete => Err(AppError::State(
                "Delete operations don't have target queues".to_string(),
            )),
            BulkOperationType::ResendFromDlq { .. } => {
                // Remove DLQ suffix to get main queue name
                if current_queue_name.ends_with("/$deadletterqueue") {
                    Ok(current_queue_name
                        .strip_suffix("/$deadletterqueue")
                        .unwrap_or(current_queue_name)
                        .to_string())
                } else {
                    Ok(current_queue_name.clone())
                }
            }
            BulkOperationType::SendToDlq { .. } => {
                // Add DLQ suffix to current queue name
                Ok(format!("{}/$deadletterqueue", current_queue_name))
            }
        }
    }

    /// Get display names for from/to queues
    pub fn get_queue_display_names(&self) -> (String, String) {
        match &self.operation_type {
            BulkOperationType::Delete => ("Current".to_string(), "Deleted".to_string()),
            BulkOperationType::ResendFromDlq { .. } => ("DLQ".to_string(), "Main".to_string()),
            BulkOperationType::SendToDlq { .. } => ("Main".to_string(), "DLQ".to_string()),
        }
    }

    /// Check if this operation should delete from source
    pub fn should_delete_source(&self) -> bool {
        match &self.operation_type {
            BulkOperationType::Delete => true, // Delete operations always delete
            BulkOperationType::ResendFromDlq { delete_source } => *delete_source,
            BulkOperationType::SendToDlq { delete_source } => *delete_source,
        }
    }

    /// Calculate context for post-processing
    pub fn calculate_post_processing_context(&self) -> BulkOperationContext {
        let model = self.model();
        let current_message_count = model
            .queue_state()
            .message_pagination
            .get_current_page_messages(crate::config::get_config_or_panic().max_messages())
            .len();

        let selected_from_current_page = self
            .message_ids
            .iter()
            .filter(|msg_id| {
                model
                    .queue_state()
                    .message_pagination
                    .all_loaded_messages
                    .iter()
                    .any(|loaded_msg| loaded_msg.id == **msg_id)
            })
            .count();

        BulkOperationContext {
            auto_reload_threshold: self.config.auto_reload_threshold,
            current_message_count,
            selected_from_current_page,
        }
    }
}

/// Context information for bulk operation post-processing
#[derive(Debug, Clone)]
pub struct BulkOperationContext {
    pub auto_reload_threshold: usize,
    pub current_message_count: usize,
    pub selected_from_current_page: usize,
}

/// Get human-readable queue type name
pub fn queue_type_display_name(queue_type: QueueType) -> &'static str {
    match queue_type {
        QueueType::Main => "main queue",
        QueueType::DeadLetter => "dead letter queue",
    }
}

/// Common validation patterns extracted as a trait
pub trait BulkOperationValidation<T: TerminalAdapter> {
    /// Quick validation for empty message lists
    fn validate_not_empty(message_ids: &[MessageIdentifier]) -> Result<(), AppError> {
        if message_ids.is_empty() {
            log::warn!("No messages provided for bulk operation");
            return Err(AppError::State(
                "No messages selected for bulk operation".to_string(),
            ));
        }
        Ok(())
    }

    /// Log operation warning about message order
    fn log_message_order_warning(message_count: usize, operation_name: &str) {
        log::warn!(
            "Bulk {} for {} messages may affect message order. Messages may not be processed in their original sequence.",
            operation_name,
            message_count
        );
    }
}

// Implement the validation trait for any TerminalAdapter type
impl<T: TerminalAdapter> BulkOperationValidation<T> for Model<T> {}
