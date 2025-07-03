use server::bulk_operations::MessageIdentifier;

/// Parameters for bulk send operations
#[derive(Debug, Clone)]
pub struct BulkSendParams {
    pub target_queue: String,
    pub should_delete: bool,
    pub loading_message_template: String,
    pub from_queue_display: String,
    pub to_queue_display: String,
}

impl BulkSendParams {
    pub fn new(
        target_queue: String,
        should_delete: bool,
        loading_message_template: &str,
        from_queue_display: &str,
        to_queue_display: &str,
    ) -> Self {
        Self {
            target_queue,
            should_delete,
            loading_message_template: loading_message_template.to_string(),
            from_queue_display: from_queue_display.to_string(),
            to_queue_display: to_queue_display.to_string(),
        }
    }
}

/// Data types for bulk send operations
pub enum BulkSendData {
    MessageIds(Vec<MessageIdentifier>),
    MessageData(Vec<(MessageIdentifier, Vec<u8>)>),
}

impl BulkSendData {
    pub fn message_count(&self) -> usize {
        match self {
            BulkSendData::MessageIds(ids) => ids.len(),
            BulkSendData::MessageData(data) => data.len(),
        }
    }
}
