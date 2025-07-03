use super::types::{MessageData, QueueType};
use crate::bulk_operations::MessageIdentifier;

#[derive(Debug, Clone)]
pub enum ServiceBusCommand {
    // Connection and queue management
    SwitchQueue {
        queue_name: String,
        queue_type: QueueType,
    },
    GetCurrentQueue,
    GetQueueStatistics {
        queue_name: String,
        queue_type: QueueType,
    },

    // Message retrieval operations
    PeekMessages {
        max_count: u32,
        from_sequence: Option<i64>,
    },
    ReceiveMessages {
        max_count: u32,
    },

    // Individual message operations
    CompleteMessage {
        message_id: String,
    },
    AbandonMessage {
        message_id: String,
    },
    DeadLetterMessage {
        message_id: String,
        reason: Option<String>,
        error_description: Option<String>,
    },

    // Bulk message operations
    BulkComplete {
        message_ids: Vec<MessageIdentifier>,
    },
    BulkDelete {
        message_ids: Vec<MessageIdentifier>,
        max_position: usize,
    },
    BulkAbandon {
        message_ids: Vec<MessageIdentifier>,
    },
    BulkDeadLetter {
        message_ids: Vec<MessageIdentifier>,
        reason: Option<String>,
        error_description: Option<String>,
    },

    // Bulk send operations
    BulkSend {
        message_ids: Vec<MessageIdentifier>,
        target_queue: String,
        should_delete_source: bool,
        repeat_count: usize,
        max_position: usize,
    },
    BulkSendPeeked {
        messages_data: Vec<(MessageIdentifier, Vec<u8>)>,
        target_queue: String,
        repeat_count: usize,
    },

    // Send operations
    SendMessage {
        queue_name: String,
        message: MessageData,
    },
    SendMessages {
        queue_name: String,
        messages: Vec<MessageData>,
    },

    // Health and status
    GetConnectionStatus,
    GetQueueStats {
        queue_name: String,
    },

    // Resource management
    DisposeConsumer,
    DisposeAllResources,
    ResetConnection,
}
