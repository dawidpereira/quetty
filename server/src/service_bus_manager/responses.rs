use super::types::{OperationStats, QueueInfo};
use crate::bulk_operations::{BulkOperationResult, MessageIdentifier};
use crate::model::MessageModel;

#[derive(Debug)]
pub enum ServiceBusResponse {
    // Queue management responses
    QueueSwitched {
        queue_info: QueueInfo,
    },
    CurrentQueue {
        queue_info: Option<QueueInfo>,
    },

    // Message retrieval responses
    MessagesReceived {
        messages: Vec<MessageModel>,
    },
    ReceivedMessages {
        messages: Vec<azservicebus::ServiceBusReceivedMessage>,
    },

    // Individual message operation responses
    MessageCompleted {
        message_id: String,
    },
    MessageAbandoned {
        message_id: String,
    },
    MessageDeadLettered {
        message_id: String,
    },

    // Bulk operation responses
    BulkOperationCompleted {
        result: BulkOperationResult,
    },
    BulkMessagesCompleted {
        successful_ids: Vec<MessageIdentifier>,
        failed_ids: Vec<MessageIdentifier>,
        stats: OperationStats,
    },
    BulkMessagesAbandoned {
        successful_ids: Vec<MessageIdentifier>,
        failed_ids: Vec<MessageIdentifier>,
        stats: OperationStats,
    },
    BulkMessagesDeadLettered {
        successful_ids: Vec<MessageIdentifier>,
        failed_ids: Vec<MessageIdentifier>,
        stats: OperationStats,
    },

    // Send operation responses
    MessageSent {
        queue_name: String,
    },
    MessagesSent {
        queue_name: String,
        count: usize,
        stats: OperationStats,
    },

    // Status and health responses
    ConnectionStatus {
        connected: bool,
        current_queue: Option<QueueInfo>,
        last_error: Option<String>,
    },
    QueueStats {
        queue_name: String,
        message_count: Option<u64>,
        active_consumer: bool,
    },

    // Resource management responses
    ConsumerDisposed,
    AllResourcesDisposed,

    // Operation success (generic)
    Success,

    // Error response
    Error {
        error: super::errors::ServiceBusError,
    },
}
