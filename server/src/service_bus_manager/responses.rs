use super::types::{OperationStats, QueueInfo, QueueType};
use crate::bulk_operations::{BulkOperationResult, MessageIdentifier};
use crate::model::MessageModel;

/// Response types for Service Bus operations.
///
/// This enum represents all possible responses from [`ServiceBusCommand`] operations
/// executed through the [`ServiceBusManager`]. Each response variant corresponds to
/// a specific command type and contains the relevant data or status information.
///
/// # Response Categories
///
/// - **Queue Management** - Information about queue switches, statistics, and current state
/// - **Message Retrieval** - Retrieved messages in various formats
/// - **Individual Operations** - Confirmations for single message operations
/// - **Bulk Operations** - Results from bulk processing with detailed statistics
/// - **Send Operations** - Confirmations and statistics for message sending
/// - **Status Operations** - Connection health and queue status information
/// - **Resource Management** - Confirmations for cleanup operations
/// - **Generic Results** - Success confirmations and error responses
///
/// # Examples
///
/// ```no_run
/// use server::service_bus_manager::{ServiceBusResponse, ServiceBusManager};
///
/// match manager.execute_command(command).await {
///     ServiceBusResponse::MessagesReceived { messages } => {
///         println!("Received {} messages", messages.len());
///         for message in messages {
///             println!("Message: {}", message.id);
///         }
///     }
///     ServiceBusResponse::QueueSwitched { queue_info } => {
///         println!("Switched to queue: {}", queue_info.name);
///     }
///     ServiceBusResponse::Error { error } => {
///         eprintln!("Operation failed: {}", error);
///     }
///     _ => println!("Operation completed successfully"),
/// }
/// ```
#[derive(Debug)]
pub enum ServiceBusResponse {
    /// Successful queue switch operation.
    ///
    /// Returned when a [`SwitchQueue`] command completes successfully.
    QueueSwitched {
        /// Information about the newly active queue
        queue_info: QueueInfo,
    },

    /// Information about the currently active queue.
    ///
    /// Returned by [`GetCurrentQueue`] command.
    CurrentQueue {
        /// Current queue information, or None if no queue is active
        queue_info: Option<QueueInfo>,
    },

    /// Detailed statistics for a specific queue.
    ///
    /// Returned by [`GetQueueStatistics`] command with comprehensive metrics.
    QueueStatistics {
        /// Name of the queue
        queue_name: String,
        /// Type of the queue (Queue or Topic)
        queue_type: QueueType,
        /// Number of active messages in the queue
        active_message_count: Option<u64>,
        /// Number of messages in the dead letter queue
        dead_letter_message_count: Option<u64>,
        /// Timestamp when statistics were retrieved
        retrieved_at: chrono::DateTime<chrono::Utc>,
    },

    /// Messages retrieved via peek operations.
    ///
    /// Contains parsed message models from [`PeekMessages`] command.
    MessagesReceived {
        /// List of parsed message models
        messages: Vec<MessageModel>,
    },

    /// Raw messages received with locks for processing.
    ///
    /// Contains native Service Bus message objects from [`ReceiveMessages`] command.
    ReceivedMessages {
        /// List of raw Service Bus messages with locks
        messages: Vec<azservicebus::ServiceBusReceivedMessage>,
    },

    /// Confirmation that a message was completed successfully.
    MessageCompleted {
        /// ID of the completed message
        message_id: String,
    },

    /// Confirmation that a message was abandoned.
    MessageAbandoned {
        /// ID of the abandoned message
        message_id: String,
    },

    /// Confirmation that a message was moved to dead letter queue.
    MessageDeadLettered {
        /// ID of the dead lettered message
        message_id: String,
    },

    /// Result of a bulk operation with comprehensive statistics.
    ///
    /// Used for complex bulk operations like delete, send, etc.
    BulkOperationCompleted {
        /// Detailed operation results and statistics
        result: BulkOperationResult,
    },

    /// Result of bulk message completion operation.
    BulkMessagesCompleted {
        /// List of successfully completed message identifiers
        successful_ids: Vec<MessageIdentifier>,
        /// List of failed message identifiers
        failed_ids: Vec<MessageIdentifier>,
        /// Operation timing and performance statistics
        stats: OperationStats,
    },

    /// Result of bulk message abandon operation.
    BulkMessagesAbandoned {
        /// List of successfully abandoned message identifiers
        successful_ids: Vec<MessageIdentifier>,
        /// List of failed message identifiers
        failed_ids: Vec<MessageIdentifier>,
        /// Operation timing and performance statistics
        stats: OperationStats,
    },

    /// Result of bulk dead letter operation.
    BulkMessagesDeadLettered {
        /// List of successfully dead lettered message identifiers
        successful_ids: Vec<MessageIdentifier>,
        /// List of failed message identifiers
        failed_ids: Vec<MessageIdentifier>,
        /// Operation timing and performance statistics
        stats: OperationStats,
    },

    /// Confirmation that a single message was sent successfully.
    MessageSent {
        /// Name of the target queue where message was sent
        queue_name: String,
    },

    /// Confirmation that multiple messages were sent successfully.
    MessagesSent {
        /// Name of the target queue where messages were sent
        queue_name: String,
        /// Number of messages sent
        count: usize,
        /// Operation timing and performance statistics
        stats: OperationStats,
    },

    /// Current connection status and health information.
    ConnectionStatus {
        /// Whether the connection is currently active
        connected: bool,
        /// Information about the currently active queue
        current_queue: Option<QueueInfo>,
        /// Last error message, if any
        last_error: Option<String>,
    },

    /// Basic statistics for a specific queue.
    QueueStats {
        /// Name of the queue
        queue_name: String,
        /// Number of messages in the queue
        message_count: Option<u64>,
        /// Whether there is an active consumer for this queue
        active_consumer: bool,
    },

    /// Confirmation that the consumer was disposed successfully.
    ConsumerDisposed,

    /// Confirmation that all resources were disposed successfully.
    AllResourcesDisposed,

    /// Confirmation that resources were disposed successfully.
    ResourcesDisposed,

    /// Confirmation that the connection was reset successfully.
    ConnectionReset,

    /// Generic success response for operations without specific data.
    Success,

    /// Error response containing detailed error information.
    ///
    /// Returned when any operation fails with comprehensive error details.
    Error {
        /// The specific error that occurred
        error: super::errors::ServiceBusError,
    },
}
