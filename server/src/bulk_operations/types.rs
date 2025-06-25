use crate::consumer::Consumer;
use azservicebus::core::BasicRetryPolicy;
use azservicebus::ServiceBusClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// Result of a bulk operation with detailed statistics
#[derive(Debug, Clone)]
pub struct BulkOperationResult {
    pub total_requested: usize,
    pub successful: usize,
    pub failed: usize,
    pub not_found: usize,
    pub error_details: Vec<String>,
    pub successful_message_ids: Vec<MessageIdentifier>,
}

impl BulkOperationResult {
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

    pub fn is_complete_success(&self) -> bool {
        self.successful == self.total_requested && self.failed == 0 && self.not_found == 0
    }
}

/// Identifier for targeting specific messages
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MessageIdentifier {
    pub id: String,
    pub sequence: i64,
}

impl std::fmt::Display for MessageIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl MessageIdentifier {
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

/// Configuration for batch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Maximum batch size for bulk operations (default: 2048, Azure Service Bus limit)
    max_batch_size: Option<u32>,
    /// Timeout for bulk operations in seconds (default: 300)
    operation_timeout_secs: Option<u64>,
    /// Buffer percentage for batch size calculation (default: 0.15 = 15%)
    buffer_percentage: Option<f64>,
    /// Minimum buffer size (default: 30)
    min_buffer_size: Option<usize>,
    /// Chunk size for bulk processing operations (default: 100)
    bulk_chunk_size: Option<usize>,
    /// Processing time limit for bulk operations in seconds (default: 30)
    bulk_processing_time_secs: Option<u64>,
    /// Timeout for lock operations in seconds (default: 5)
    lock_timeout_secs: Option<u64>,
    /// Multiplier for calculating max messages to process (default: 3)
    max_messages_multiplier: Option<usize>,
    /// Minimum messages to process in bulk operations (default: 100)
    min_messages_to_process: Option<usize>,
    /// Maximum messages to process in bulk operations (default: 1000)
    max_messages_to_process: Option<usize>,
    /// Maximum number of messages for bulk operations (default: 100)
    bulk_operation_max_count: Option<usize>,
    /// Threshold for triggering auto-reload after bulk operations (default: 10)
    auto_reload_threshold: Option<usize>,
    /// Small deletion threshold for backfill operations (default: 5)
    small_deletion_threshold: Option<usize>,
}

impl BatchConfig {
    /// Create a new BatchConfig
    pub fn new(max_batch_size: u32, operation_timeout_secs: u64) -> Self {
        Self {
            max_batch_size: Some(max_batch_size),
            operation_timeout_secs: Some(operation_timeout_secs),
            buffer_percentage: None,
            min_buffer_size: None,
            bulk_chunk_size: None,
            bulk_processing_time_secs: None,
            lock_timeout_secs: None,
            max_messages_multiplier: None,
            min_messages_to_process: None,
            max_messages_to_process: None,
            bulk_operation_max_count: None,
            auto_reload_threshold: None,
            small_deletion_threshold: None,
        }
    }

    /// Get the maximum batch size for bulk operations
    pub fn max_batch_size(&self) -> u32 {
        self.max_batch_size.unwrap_or(2048)
    }

    /// Get the timeout for bulk operations
    pub fn operation_timeout_secs(&self) -> u64 {
        self.operation_timeout_secs.unwrap_or(300)
    }

    /// Get the buffer percentage for batch size calculation
    pub fn buffer_percentage(&self) -> f64 {
        self.buffer_percentage.unwrap_or(0.15)
    }

    /// Get the minimum buffer size
    pub fn min_buffer_size(&self) -> usize {
        self.min_buffer_size.unwrap_or(30)
    }

    /// Get the chunk size for bulk processing operations
    pub fn bulk_chunk_size(&self) -> usize {
        self.bulk_chunk_size.unwrap_or(100)
    }

    /// Get the processing time limit for bulk operations in seconds
    pub fn bulk_processing_time_secs(&self) -> u64 {
        self.bulk_processing_time_secs.unwrap_or(30)
    }

    /// Get the timeout for lock operations in seconds
    pub fn lock_timeout_secs(&self) -> u64 {
        self.lock_timeout_secs.unwrap_or(5)
    }

    /// Get the multiplier for calculating max messages to process
    pub fn max_messages_multiplier(&self) -> usize {
        self.max_messages_multiplier.unwrap_or(3)
    }

    /// Get the minimum messages to process in bulk operations
    pub fn min_messages_to_process(&self) -> usize {
        self.min_messages_to_process.unwrap_or(100)
    }

    /// Get the maximum messages to process in bulk operations
    pub fn max_messages_to_process(&self) -> usize {
        self.max_messages_to_process.unwrap_or(1000)
    }

    /// Get the maximum number of messages for bulk operations
    pub fn bulk_operation_max_count(&self) -> usize {
        self.bulk_operation_max_count.unwrap_or(100)
    }

    /// Get the minimum number of messages for bulk operations
    pub fn bulk_operation_min_count(&self) -> usize {
        1
    }

    /// Get the threshold for triggering auto-reload after bulk operations
    pub fn auto_reload_threshold(&self) -> usize {
        self.auto_reload_threshold.unwrap_or(10)
    }

    /// Get the small deletion threshold for backfill operations
    pub fn small_deletion_threshold(&self) -> usize {
        self.small_deletion_threshold.unwrap_or(5)
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

/// Parameters for processing a single batch of messages
pub(crate) struct BatchProcessingContext<'a> {
    pub consumer: Arc<Mutex<Consumer>>,
    pub batch_size: usize,
    pub target_messages_found: usize,
    pub target_map: &'a HashMap<String, MessageIdentifier>,
    pub messages_processed: usize,
    pub remaining_targets: &'a mut HashMap<String, MessageIdentifier>,
    pub target_messages_vec: &'a mut Vec<azservicebus::ServiceBusReceivedMessage>,
    pub non_target_messages: &'a mut Vec<azservicebus::ServiceBusReceivedMessage>,
}

/// Parameters for bulk send operations
pub struct BulkSendParams {
    pub target_queue: String,
    pub should_delete: bool,
    pub message_identifiers: Vec<MessageIdentifier>,
    pub messages_data: Option<Vec<(MessageIdentifier, Vec<u8>)>>, // For peek-based operations
}

impl BulkSendParams {
    /// Create parameters for operations that retrieve messages from the queue
    pub fn with_retrieval(
        target_queue: String,
        should_delete: bool,
        message_identifiers: Vec<MessageIdentifier>,
    ) -> Self {
        Self {
            target_queue,
            should_delete,
            message_identifiers,
            messages_data: None,
        }
    }

    /// Create parameters for operations with pre-fetched message data
    pub fn with_message_data(
        target_queue: String,
        should_delete: bool,
        messages_data: Vec<(MessageIdentifier, Vec<u8>)>,
    ) -> Self {
        // Extract identifiers from the data
        let message_identifiers = messages_data.iter().map(|(id, _)| id.clone()).collect();

        Self {
            target_queue,
            should_delete,
            message_identifiers,
            messages_data: Some(messages_data),
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

/// Context for bulk operations
pub struct BulkOperationContext {
    pub consumer: Arc<Mutex<Consumer>>,
    pub service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
    pub target_queue: String,
    pub operation_type: QueueOperationType,
    pub cancel_token: CancellationToken,
}

impl BulkOperationContext {
    /// Create a new operation context with automatic operation type detection
    pub fn new(
        consumer: Arc<Mutex<Consumer>>,
        service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
        target_queue: String,
    ) -> Self {
        let operation_type = QueueOperationType::from_queue_name(&target_queue);
        Self {
            consumer,
            service_bus_client,
            target_queue,
            operation_type,
            cancel_token: CancellationToken::new(),
        }
    }

    /// Create a new context with a specific cancellation token
    pub fn with_cancel_token(
        consumer: Arc<Mutex<Consumer>>,
        service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
        target_queue: String,
        cancel_token: CancellationToken,
    ) -> Self {
        let operation_type = QueueOperationType::from_queue_name(&target_queue);
        Self {
            consumer,
            service_bus_client,
            target_queue,
            operation_type,
            cancel_token,
        }
    }

    /// Cancel the operation
    pub fn cancel(&self) {
        self.cancel_token.cancel();
        log::info!("Bulk operation cancelled for queue: {}", self.target_queue);
    }

    /// Check if the operation has been cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }
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