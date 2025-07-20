//! Types and data structures for bulk operations.
//!
//! This module defines the core types used throughout the bulk operations system,
//! including result tracking, message identification, configuration, and operation contexts.

use crate::consumer::Consumer;
use azservicebus::ServiceBusClient;
use azservicebus::core::BasicRetryPolicy;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// Result of a bulk operation with detailed statistics and error tracking.
///
/// Provides comprehensive information about the outcome of bulk operations,
/// including success counts, failure details, and lists of processed messages.
/// Used to report operation results to callers and for UI feedback.
///
/// # Examples
///
/// ```no_run
/// use quetty_server::bulk_operations::BulkOperationResult;
///
/// let mut result = BulkOperationResult::new(100);
/// result.add_success();
/// result.add_failure("Connection timeout".to_string());
///
/// if result.is_complete_success() {
///     println!("All operations completed successfully");
/// } else {
///     println!("Partial success: {} of {} succeeded",
///              result.successful, result.total_requested);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct BulkOperationResult {
    /// Total number of operations requested
    pub total_requested: usize,
    /// Number of operations that completed successfully
    pub successful: usize,
    /// Number of operations that failed
    pub failed: usize,
    /// Number of target items that were not found
    pub not_found: usize,
    /// Detailed error messages for failed operations
    pub error_details: Vec<String>,
    /// Identifiers of messages that were processed successfully
    pub successful_message_ids: Vec<MessageIdentifier>,
}

impl BulkOperationResult {
    /// Creates a new BulkOperationResult for the specified number of operations.
    ///
    /// # Arguments
    ///
    /// * `total_requested` - The total number of operations that will be attempted
    ///
    /// # Returns
    ///
    /// A new result tracker with zero counts and empty collections
    pub fn new(total_requested: usize) -> Self {
        Self {
            total_requested,
            successful: 0,
            failed: 0,
            not_found: 0,
            error_details: Vec::new(),
            successful_message_ids: Vec::new(),
        }
    }

    pub fn add_success(&mut self) {
        self.successful += 1;
    }

    pub fn add_failure(&mut self, error: String) {
        self.failed += 1;
        self.error_details.push(error);
    }

    pub fn add_successful_message(&mut self, message_id: MessageIdentifier) {
        self.successful += 1;
        self.successful_message_ids.push(message_id.clone());
        log::debug!(
            "SUCCESS COUNT: Incremented to {} (added message: {})",
            self.successful,
            message_id.id
        );
    }

    pub fn add_not_found(&mut self) {
        self.not_found += 1;
    }

    /// Checks if all requested operations completed successfully.
    ///
    /// # Returns
    ///
    /// `true` if all operations succeeded with no failures or missing items
    pub fn is_complete_success(&self) -> bool {
        self.successful == self.total_requested && self.failed == 0 && self.not_found == 0
    }
}

/// Identifier for targeting specific messages in bulk operations.
///
/// Combines message ID and sequence number for precise message targeting.
/// The sequence number is used for optimization during bulk scanning operations
/// to minimize the number of messages that need to be processed.
///
/// # Examples
///
/// ```no_run
/// use quetty_server::bulk_operations::MessageIdentifier;
///
/// let msg_id = MessageIdentifier::new("msg-123".to_string(), 4567);
/// println!("Message: {} at sequence {}", msg_id.id, msg_id.sequence);
///
/// let composite = msg_id.composite_key();
/// println!("Composite key: {}", composite);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MessageIdentifier {
    /// The unique message identifier
    pub id: String,
    /// The message sequence number for optimization
    pub sequence: i64,
}

impl std::fmt::Display for MessageIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl MessageIdentifier {
    /// Creates a new MessageIdentifier with the specified ID and sequence.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique message identifier
    /// * `sequence` - The message sequence number
    ///
    /// # Returns
    ///
    /// A new MessageIdentifier instance
    pub fn new(id: String, sequence: i64) -> Self {
        Self { id, sequence }
    }

    pub fn from_message(message: &crate::model::MessageModel) -> Self {
        Self {
            id: message.id.clone(),
            sequence: message.sequence,
        }
    }

    pub fn from_string(id: String) -> Self {
        Self { id, sequence: 0 }
    }

    /// Creates a composite key for exact matching.
    ///
    /// Combines the message ID and sequence number into a single string
    /// that can be used for precise identification in hash maps or other
    /// data structures that require string keys.
    ///
    /// # Returns
    ///
    /// A string in the format "id:sequence"
    pub fn composite_key(&self) -> String {
        format!("{}:{}", self.id, self.sequence)
    }
}

impl From<String> for MessageIdentifier {
    fn from(id: String) -> Self {
        Self::from_string(id)
    }
}

impl From<&str> for MessageIdentifier {
    fn from(id: &str) -> Self {
        Self::from_string(id.to_string())
    }
}

impl From<MessageIdentifier> for String {
    fn from(val: MessageIdentifier) -> Self {
        val.id
    }
}

impl PartialEq<String> for MessageIdentifier {
    fn eq(&self, other: &String) -> bool {
        &self.id == other
    }
}

impl PartialEq<MessageIdentifier> for String {
    fn eq(&self, other: &MessageIdentifier) -> bool {
        self == &other.id
    }
}

/// Configuration for bulk operation batching and limits.
///
/// Controls various aspects of bulk operations including batch sizes,
/// timeouts, processing limits, and UI behavior. Provides sensible defaults
/// for all configuration values.
///
/// # Examples
///
/// ```no_run
/// use quetty_server::bulk_operations::BatchConfig;
///
/// // Use default configuration
/// let config = BatchConfig::default();
///
/// // Create custom configuration
/// let config = BatchConfig::new(100, 600);
///
/// // Access configuration values
/// println!("Max batch size: {}", config.max_batch_size());
/// println!("Timeout: {}s", config.operation_timeout_secs());
/// ```
#[derive(Debug, Deserialize, Default, Clone)]
pub struct BatchConfig {
    /// Maximum batch size for bulk operations (default: 200)
    max_batch_size: Option<u32>,
    /// Timeout for bulk operations in seconds (default: 300)
    operation_timeout_secs: Option<u64>,
    /// Chunk size for bulk processing operations (default: 200, same as max_batch_size)
    bulk_chunk_size: Option<usize>,
    /// Processing time limit for bulk operations in seconds (default: 30)
    bulk_processing_time_secs: Option<u64>,
    /// Timeout for lock operations in seconds (default: 10)
    lock_timeout_secs: Option<u64>,
    /// Maximum messages to process in bulk operations (default: 10,000)
    max_messages_to_process: Option<usize>,
    /// Auto-reload threshold for UI refresh after bulk operations (default: 50)
    auto_reload_threshold: Option<usize>,
    /// Timeout for individual receive message operations in seconds (default: 5)
    receive_timeout_secs: Option<u64>,
}

impl BatchConfig {
    /// Creates a new BatchConfig with specified batch size and timeout.
    ///
    /// Other configuration values will use their defaults when accessed.
    ///
    /// # Arguments
    ///
    /// * `max_batch_size` - Maximum number of messages per batch
    /// * `operation_timeout_secs` - Timeout for bulk operations in seconds
    ///
    /// # Returns
    ///
    /// A new BatchConfig with the specified values
    pub fn new(max_batch_size: u32, operation_timeout_secs: u64) -> Self {
        Self {
            max_batch_size: Some(max_batch_size),
            operation_timeout_secs: Some(operation_timeout_secs),
            bulk_chunk_size: None,
            bulk_processing_time_secs: None,
            lock_timeout_secs: None,
            max_messages_to_process: None,
            auto_reload_threshold: None,
            receive_timeout_secs: None,
        }
    }

    /// Get the maximum batch size for bulk operations
    pub fn max_batch_size(&self) -> u32 {
        self.max_batch_size.unwrap_or(500)
    }

    /// Get the timeout for bulk operations
    pub fn operation_timeout_secs(&self) -> u64 {
        self.operation_timeout_secs.unwrap_or(600)
    }

    /// Get the chunk size for bulk processing operations
    pub fn bulk_chunk_size(&self) -> usize {
        self.bulk_chunk_size.unwrap_or(500)
    }

    /// Get the processing time limit for bulk operations in seconds
    pub fn bulk_processing_time_secs(&self) -> u64 {
        self.bulk_processing_time_secs.unwrap_or(300)
    }

    /// Get the timeout for lock operations in seconds
    pub fn lock_timeout_secs(&self) -> u64 {
        self.lock_timeout_secs.unwrap_or(10)
    }

    /// Get the maximum messages to process in bulk operations
    pub fn max_messages_to_process(&self) -> usize {
        self.max_messages_to_process.unwrap_or(10_000)
    }

    /// Get the threshold for triggering auto-reload after bulk operations
    pub fn auto_reload_threshold(&self) -> usize {
        self.auto_reload_threshold.unwrap_or(50)
    }

    /// Get the timeout for individual receive message operations in seconds
    pub fn receive_timeout_secs(&self) -> u64 {
        self.receive_timeout_secs.unwrap_or(5)
    }
}

/// Context for Service Bus operations containing shared resources
#[derive(Debug, Clone)]
pub struct ServiceBusOperationContext {
    pub consumer: Arc<Mutex<Consumer>>,
    pub service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
    pub main_queue_name: String,
    pub cancel_token: CancellationToken,
}

impl ServiceBusOperationContext {
    /// Create a new ServiceBusOperationContext
    pub fn new(
        consumer: Arc<Mutex<Consumer>>,
        service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
        main_queue_name: String,
    ) -> Self {
        Self {
            consumer,
            service_bus_client,
            main_queue_name,
            cancel_token: CancellationToken::new(),
        }
    }
}

/// Parameters for bulk send operations
#[derive(Debug, Clone)]
pub struct BulkSendParams {
    pub target_queue: String,
    pub should_delete: bool,
    pub message_identifiers: Vec<MessageIdentifier>,
    pub messages_data: Option<Vec<(MessageIdentifier, Vec<u8>)>>, // For peek-based operations
    pub max_position: usize,                                      // For dynamic processing limits
}

impl BulkSendParams {
    /// Create parameters for operations that retrieve messages from the queue
    pub fn with_retrieval(
        target_queue: String,
        should_delete: bool,
        message_identifiers: Vec<MessageIdentifier>,
        max_position: usize,
    ) -> Self {
        Self {
            target_queue,
            should_delete,
            message_identifiers,
            messages_data: None,
            max_position,
        }
    }

    /// Create parameters for operations with pre-fetched message data
    pub fn with_message_data(
        target_queue: String,
        should_delete: bool,
        messages_data: Vec<(MessageIdentifier, Vec<u8>)>,
        max_position: usize,
    ) -> Self {
        // Extract identifiers from the data
        let message_identifiers = messages_data.iter().map(|(id, _)| id.clone()).collect();

        Self {
            target_queue,
            should_delete,
            message_identifiers,
            messages_data: Some(messages_data),
            max_position,
        }
    }

    /// Create parameters with max position for better processing limits
    pub fn with_max_position(
        target_queue: String,
        should_delete: bool,
        message_identifiers: Vec<MessageIdentifier>,
        max_position: usize,
    ) -> Self {
        Self {
            target_queue,
            should_delete,
            message_identifiers,
            messages_data: None,
            max_position,
        }
    }
}

/// Queue operation type determination
#[derive(Debug, Clone)]
pub enum QueueOperationType {
    /// Send to regular queue (copy message content)
    SendToQueue,
    /// Send to dead letter queue (use dead_letter_message operation)
    SendToDLQ,
}

impl QueueOperationType {
    /// Determine operation type based on target queue name
    pub fn from_queue_name(queue_name: &str) -> Self {
        if queue_name.ends_with("/$deadletterqueue") {
            Self::SendToDLQ
        } else {
            Self::SendToQueue
        }
    }
}

/// Bulk operation context containing shared resources
#[derive(Debug, Clone)]
pub struct BulkOperationContext {
    pub consumer: Arc<Mutex<crate::consumer::Consumer>>,
    pub cancel_token: CancellationToken,
    /// Name of the queue this operation is targeting (used for deferred message persistence)
    pub queue_name: String,
}

/// Parameters for process_target_messages method
pub struct ProcessTargetMessagesParams<'a> {
    pub messages: Vec<azservicebus::ServiceBusReceivedMessage>,
    pub context: &'a BulkOperationContext,
    pub params: &'a BulkSendParams,
    pub target_map: &'a HashMap<String, MessageIdentifier>,
    pub result: &'a mut BulkOperationResult,
}

impl<'a> ProcessTargetMessagesParams<'a> {
    pub fn new(
        messages: Vec<azservicebus::ServiceBusReceivedMessage>,
        context: &'a BulkOperationContext,
        params: &'a BulkSendParams,
        target_map: &'a HashMap<String, MessageIdentifier>,
        result: &'a mut BulkOperationResult,
    ) -> Self {
        Self {
            messages,
            context,
            params,
            target_map,
            result,
        }
    }
}
