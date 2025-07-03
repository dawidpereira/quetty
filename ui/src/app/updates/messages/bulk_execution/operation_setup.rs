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
    /// Maximum number of messages allowed for operation
    pub max_count: usize,
    /// Auto-reload threshold for UI refresh
    pub auto_reload_threshold: usize,
}

impl BulkOperationConfig {
    /// Create configuration from app config
    pub fn from_app_config() -> Self {
        let config = crate::config::get_config_or_panic();
        Self {
            max_count: config.batch().max_messages_to_process(),
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
        self.validate_operation_size()?; // Added back validation against max_messages_to_process

        // Validate link credit usage to prevent receiver credit exhaustion
        self.validate_link_credit_limit(model)?;

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

        if count > self.config.max_count {
            return Err(AppError::State(format!(
                "Too many messages selected: {} (maximum: {})",
                count, self.config.max_count
            )));
        }

        Ok(())
    }

    /// Validate operation size is within processing limits (max_messages_to_process)
    fn validate_operation_size(&self) -> Result<(), AppError> {
        let config = crate::config::get_config_or_panic();
        let max_messages_to_process = config.batch().max_messages_to_process();

        if self.message_ids.len() > max_messages_to_process {
            return Err(AppError::State(format!(
                "Operation size {} exceeds maximum allowed processing limit of {}",
                self.message_ids.len(),
                max_messages_to_process
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

        // Validate that we have messages loaded to prevent operations on stale cache
        let messages = model.queue_manager.queue_state.messages.as_ref();
        if messages.is_none() || messages.map(|m| m.is_empty()).unwrap_or(true) {
            return Err(AppError::State(
                "No messages available for operation. Please wait for messages to load after queue switch.".to_string()
            ));
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

    /// Ensure the operation will not exceed Service Bus link credit (2048)
    fn validate_link_credit_limit(&self, model: &Model<T>) -> Result<(), AppError> {
        // Get the highest position index of selected messages (1-based)
        let max_position = model
            .queue_state()
            .bulk_selection
            .get_highest_selected_index()
            .unwrap_or(0);

        if max_position == 0 {
            // Should not happen, but skip check if we can't determine
            return Ok(());
        }

        // Calculate non-target messages that will remain locked during the scan
        // This is: max_position - selected_count
        let locked_non_targets = max_position.saturating_sub(self.message_ids.len());

        // Use exact Azure Service Bus link credit limit
        const LINK_CREDIT_LIMIT: usize = 2048;

        if locked_non_targets > LINK_CREDIT_LIMIT {
            return Err(AppError::State(format!(
                "Operation would lock {} non-target messages (max position: {}, selected: {}), which exceeds the Azure Service Bus link-credit limit of {}. This would cause the bulk operation to get stuck.\n\nTo fix this:\n• Select messages in a smaller range (closer together)\n• Split into multiple smaller operations\n• Select messages from earlier pages",
                locked_non_targets,
                max_position,
                self.message_ids.len(),
                LINK_CREDIT_LIMIT
            )));
        }

        log::debug!(
            "Link credit validation passed: max_position={}, selected={}, locked_non_targets={}, limit={}",
            max_position,
            self.message_ids.len(),
            locked_non_targets,
            LINK_CREDIT_LIMIT
        );

        Ok(())
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
            .get_current_page_messages(crate::config::get_current_page_size())
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

        // Use actual highest selected index if available, otherwise get current message index
        let max_position = if let Some(highest_index) = model
            .queue_state()
            .bulk_selection
            .get_highest_selected_index()
        {
            // get_highest_selected_index now returns 1-based position
            highest_index
        } else if self.message_ids.len() == 1 {
            // Single message operation - get current message index from UI state
            if let Ok(tuirealm::State::One(tuirealm::StateValue::Usize(selected_index))) = model
                .app
                .state(&crate::components::common::ComponentId::Messages)
            {
                selected_index + 1 // Convert to 1-based position
            } else {
                // Fallback to page-based estimation
                let page_size = crate::config::get_config_or_panic().max_messages() as usize;
                let current_page = model.queue_state().message_pagination.current_page;
                self.calculate_max_position(model, page_size, current_page)
            }
        } else {
            // Fallback to the old calculation method
            let page_size = crate::config::get_config_or_panic().max_messages() as usize;
            let current_page = model.queue_state().message_pagination.current_page;
            self.calculate_max_position(model, page_size, current_page)
        };

        BulkOperationContext {
            auto_reload_threshold: self.config.auto_reload_threshold,
            current_message_count,
            selected_from_current_page,
            max_position,
        }
    }

    /// Calculate estimated maximum position of selected messages
    fn calculate_max_position(
        &self,
        model: &Model<T>,
        page_size: usize,
        current_page: usize,
    ) -> usize {
        let all_loaded_messages = &model.queue_state().message_pagination.all_loaded_messages;

        // Find the highest position among selected messages in loaded data
        let mut max_loaded_position = 0;
        for (index, loaded_msg) in all_loaded_messages.iter().enumerate() {
            if self
                .message_ids
                .iter()
                .any(|msg_id| msg_id.id == loaded_msg.id)
            {
                max_loaded_position = std::cmp::max(max_loaded_position, index + 1);
            }
        }

        // If we found positions in loaded data, use that
        if max_loaded_position > 0 {
            log::info!(
                "Found selected messages in loaded data, highest position: {}",
                max_loaded_position
            );
            return max_loaded_position;
        }

        // Fallback: estimate based on current page
        // If user is on page 30 selecting messages, those messages are likely around position 30 * page_size
        let page_based_estimate = (current_page + 1) * page_size;

        log::info!(
            "Using page-based estimation: page {} * page_size {} = estimated position {}",
            current_page + 1,
            page_size,
            page_based_estimate
        );

        page_based_estimate
    }
}

/// Context information for bulk operation post-processing
#[derive(Debug, Clone)]
pub struct BulkOperationContext {
    pub auto_reload_threshold: usize,
    pub current_message_count: usize,
    pub selected_from_current_page: usize,
    pub max_position: usize,
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
