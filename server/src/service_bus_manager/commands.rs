use super::types::{MessageData, QueueType};
use crate::bulk_operations::MessageIdentifier;

/// Commands for Service Bus operations using the command pattern.
///
/// This enum defines all possible operations that can be performed through the
/// [`ServiceBusManager`]. Each command encapsulates the parameters needed for
/// a specific operation, providing a clean separation between the command
/// definition and its execution.
///
/// # Command Categories
///
/// - **Queue Management** - Switch queues, get statistics, and manage connections
/// - **Message Retrieval** - Peek and receive messages from queues
/// - **Individual Message Operations** - Complete, abandon, or dead letter single messages
/// - **Bulk Operations** - Efficient bulk processing of multiple messages
/// - **Send Operations** - Send single or multiple messages to queues
/// - **Status Operations** - Check connection health and queue statistics
/// - **Resource Management** - Clean up consumers and reset connections
///
/// # Examples
///
/// ```no_run
/// use server::service_bus_manager::{ServiceBusCommand, QueueType};
///
/// // Switch to a queue
/// let command = ServiceBusCommand::SwitchQueue {
///     queue_name: "my-queue".to_string(),
///     queue_type: QueueType::Queue,
/// };
///
/// // Peek messages
/// let command = ServiceBusCommand::PeekMessages {
///     max_count: 10,
///     from_sequence: None,
/// };
///
/// // Send a message
/// let command = ServiceBusCommand::SendMessage {
///     queue_name: "target-queue".to_string(),
///     message: message_data,
/// };
/// ```
#[derive(Debug, Clone)]
pub enum ServiceBusCommand {
    /// Switch to a different queue for message operations.
    ///
    /// Changes the active queue context for subsequent operations.
    SwitchQueue {
        /// Name of the queue to switch to
        queue_name: String,
        /// Type of queue (Queue or Topic)
        queue_type: QueueType,
    },

    /// Get information about the currently active queue.
    GetCurrentQueue,

    /// Retrieve detailed statistics for a specific queue.
    ///
    /// Returns message counts, size information, and other queue metrics.
    GetQueueStatistics {
        /// Name of the queue to get statistics for
        queue_name: String,
        /// Type of queue (Queue or Topic)
        queue_type: QueueType,
    },

    /// Peek at messages without removing them from the queue.
    ///
    /// Messages remain in the queue and can be retrieved again.
    PeekMessages {
        /// Maximum number of messages to peek
        max_count: u32,
        /// Optional sequence number to start peeking from
        from_sequence: Option<i64>,
    },

    /// Receive messages with a lock for processing.
    ///
    /// Messages are locked during processing and must be completed or abandoned.
    ReceiveMessages {
        /// Maximum number of messages to receive
        max_count: u32,
    },

    /// Complete (acknowledge) a message, removing it from the queue.
    CompleteMessage {
        /// ID of the message to complete
        message_id: String,
    },

    /// Abandon a message, returning it to the queue for redelivery.
    AbandonMessage {
        /// ID of the message to abandon
        message_id: String,
    },

    /// Move a message to the dead letter queue.
    ///
    /// Used for messages that cannot be processed successfully.
    DeadLetterMessage {
        /// ID of the message to dead letter
        message_id: String,
        /// Optional reason for dead lettering
        reason: Option<String>,
        /// Optional detailed error description
        error_description: Option<String>,
    },

    /// Complete multiple messages in a single bulk operation.
    BulkComplete {
        /// List of message identifiers to complete
        message_ids: Vec<MessageIdentifier>,
    },

    /// Delete multiple messages efficiently using bulk processing.
    ///
    /// Optimized for large-scale message deletion operations.
    BulkDelete {
        /// List of message identifiers to delete
        message_ids: Vec<MessageIdentifier>,
        /// Maximum position to scan when looking for messages
        max_position: usize,
    },

    /// Abandon multiple messages in a single bulk operation.
    BulkAbandon {
        /// List of message identifiers to abandon
        message_ids: Vec<MessageIdentifier>,
    },

    /// Move multiple messages to the dead letter queue.
    BulkDeadLetter {
        /// List of message identifiers to dead letter
        message_ids: Vec<MessageIdentifier>,
        /// Optional reason for dead lettering
        reason: Option<String>,
        /// Optional detailed error description
        error_description: Option<String>,
    },

    /// Send multiple messages to a target queue with optional source deletion.
    ///
    /// Can optionally delete source messages after successful send.
    BulkSend {
        /// List of message identifiers to send
        message_ids: Vec<MessageIdentifier>,
        /// Name of the target queue to send messages to
        target_queue: String,
        /// Whether to delete source messages after sending
        should_delete_source: bool,
        /// Number of times to repeat each message
        repeat_count: usize,
        /// Maximum position to scan when retrieving messages
        max_position: usize,
    },

    /// Send pre-fetched message data to a target queue.
    ///
    /// Used when message content has already been retrieved via peek operations.
    BulkSendPeeked {
        /// Pre-fetched message data (identifier and content)
        messages_data: Vec<(MessageIdentifier, Vec<u8>)>,
        /// Name of the target queue to send messages to
        target_queue: String,
        /// Number of times to repeat each message
        repeat_count: usize,
    },

    /// Send a single message to a specific queue.
    SendMessage {
        /// Name of the target queue
        queue_name: String,
        /// Message data to send
        message: MessageData,
    },

    /// Send multiple messages to a specific queue.
    SendMessages {
        /// Name of the target queue
        queue_name: String,
        /// List of messages to send
        messages: Vec<MessageData>,
    },

    /// Check the current connection status to Service Bus.
    GetConnectionStatus,

    /// Get basic statistics for a specific queue.
    GetQueueStats {
        /// Name of the queue to get stats for
        queue_name: String,
    },

    /// Dispose of the current message consumer.
    ///
    /// Cleans up consumer resources and releases locks.
    DisposeConsumer,

    /// Dispose of all Service Bus resources.
    ///
    /// Comprehensive cleanup of all consumers, producers, and connections.
    DisposeAllResources,

    /// Reset the Service Bus connection.
    ///
    /// Re-establishes connection using current configuration.
    ResetConnection,
}
