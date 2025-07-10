use super::deleter::BulkDeleter;
use super::types::{
    BatchConfig, BulkOperationContext, BulkOperationResult, BulkSendParams, MessageIdentifier,
};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// High-level handler for bulk operations on Azure Service Bus messages.
///
/// The BulkOperationHandler provides a simplified interface for performing
/// bulk operations like deleting multiple messages, sending messages to dead letter queues,
/// and resending messages from dead letter queues. It orchestrates the underlying
/// bulk deletion and processing logic.
///
/// # Features
///
/// - **Bulk Delete** - Efficiently delete multiple messages from queues
/// - **Dead Letter Operations** - Move messages to/from dead letter queues
/// - **Batch Processing** - Configurable batch sizes for optimal performance
/// - **Error Handling** - Comprehensive error reporting with operation results
/// - **Cancellation Support** - Graceful cancellation of long-running operations
///
/// # Examples
///
/// ```no_run
/// use server::bulk_operations::{BulkOperationHandler, BatchConfig, MessageIdentifier};
/// use server::consumer::Consumer;
/// use std::sync::Arc;
/// use tokio::sync::Mutex;
///
/// async fn example(consumer: Arc<Mutex<Consumer>>) -> Result<(), Box<dyn std::error::Error>> {
///     let config = BatchConfig::default();
///     let handler = BulkOperationHandler::new(config);
///
///     let message_ids = vec![
///         MessageIdentifier::SequenceNumber(12345),
///         MessageIdentifier::SequenceNumber(12346),
///     ];
///
///     let result = handler.delete_messages(
///         consumer,
///         "my-queue".to_string(),
///         message_ids,
///         100, // max_position
///     ).await?;
///
///     println!("Deleted {} messages", result.successful_count);
///     Ok(())
/// }
/// ```
pub struct BulkOperationHandler {
    deleter: BulkDeleter,
}

impl BulkOperationHandler {
    /// Creates a new BulkOperationHandler with the specified configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Batch configuration controlling operation behavior
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use server::bulk_operations::{BulkOperationHandler, BatchConfig};
    ///
    /// let config = BatchConfig {
    ///     batch_size: 50,
    ///     timeout: std::time::Duration::from_secs(30),
    ///     ..Default::default()
    /// };
    /// let handler = BulkOperationHandler::new(config);
    /// ```
    pub fn new(config: BatchConfig) -> Self {
        Self {
            deleter: BulkDeleter::new(config),
        }
    }

    /// Executes a bulk delete operation on the specified messages.
    ///
    /// This method deletes multiple messages from a Service Bus queue efficiently
    /// by processing them in batches. It provides comprehensive error reporting
    /// and handles partial failures gracefully.
    ///
    /// # Arguments
    ///
    /// * `consumer` - Service Bus consumer for message operations
    /// * `queue_name` - Name of the queue containing the messages
    /// * `targets` - List of message identifiers to delete
    /// * `max_position` - Maximum position limit for message processing
    ///
    /// # Returns
    ///
    /// [`BulkOperationResult`] containing the count of successful and failed operations
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The consumer is unavailable or disposed
    /// - Service Bus operations fail
    /// - The operation is cancelled
    /// - Invalid message identifiers are provided
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use server::bulk_operations::{BulkOperationHandler, MessageIdentifier};
    /// use server::consumer::Consumer;
    /// use std::sync::Arc;
    /// use tokio::sync::Mutex;
    ///
    /// async fn delete_example(
    ///     handler: &BulkOperationHandler,
    ///     consumer: Arc<Mutex<Consumer>>
    /// ) -> Result<(), Box<dyn std::error::Error>> {
    ///     let messages_to_delete = vec![
    ///         MessageIdentifier::SequenceNumber(100),
    ///         MessageIdentifier::SequenceNumber(101),
    ///         MessageIdentifier::SequenceNumber(102),
    ///     ];
    ///
    ///     let result = handler.delete_messages(
    ///         consumer,
    ///         "orders-queue".to_string(),
    ///         messages_to_delete,
    ///         1000,
    ///     ).await?;
    ///
    ///     println!("Successfully deleted: {}", result.successful_count);
    ///     println!("Failed to delete: {}", result.failed_count);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn delete_messages(
        &self,
        consumer: Arc<Mutex<crate::consumer::Consumer>>,
        queue_name: String,
        targets: Vec<MessageIdentifier>,
        max_position: usize,
    ) -> Result<BulkOperationResult, Box<dyn Error + Send + Sync>> {
        let context = BulkOperationContext {
            consumer,
            cancel_token: CancellationToken::new(),
            queue_name: queue_name.clone(),
        };

        // Create BulkSendParams with max position
        let params = BulkSendParams {
            target_queue: queue_name,
            should_delete: true,
            message_identifiers: targets,
            messages_data: None,
            max_position,
        };

        self.deleter.delete_messages(context, params).await
    }
}

impl Default for BulkOperationHandler {
    /// Creates a BulkOperationHandler with default configuration.
    ///
    /// Uses the default [`BatchConfig`] settings for batch size, timeouts,
    /// and other operation parameters.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use server::bulk_operations::BulkOperationHandler;
    ///
    /// let handler = BulkOperationHandler::default();
    /// ```
    fn default() -> Self {
        Self::new(BatchConfig::default())
    }
}
