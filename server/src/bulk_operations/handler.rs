use super::deleter::BulkDeleter;
use super::types::{
    BatchConfig, BulkOperationContext, BulkOperationResult, BulkSendParams, MessageIdentifier,
};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// Modern bulk operations handler
pub struct BulkOperationHandler {
    deleter: BulkDeleter,
}

impl BulkOperationHandler {
    pub fn new(config: BatchConfig) -> Self {
        Self {
            deleter: BulkDeleter::new(config),
        }
    }

    /// Execute bulk delete operation
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
    fn default() -> Self {
        Self::new(BatchConfig::default())
    }
}
