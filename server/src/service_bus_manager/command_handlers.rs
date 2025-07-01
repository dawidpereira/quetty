use super::consumer_manager::ConsumerManager;
use super::producer_manager::ProducerManager;
use super::queue_statistics_service::QueueStatisticsService;
use super::types::{QueueInfo, QueueType};

use crate::bulk_operations::BulkOperationResult;
use crate::bulk_operations::{BulkOperationHandler, MessageIdentifier, types::BatchConfig};
use crate::service_bus_manager::{
    errors::ServiceBusError, responses::ServiceBusResponse, types::MessageData,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Result type for service bus operations
type ServiceBusResult<T> = Result<T, ServiceBusError>;

// Error message constants
const ERROR_INDIVIDUAL_MSG_OPERATIONS: &str =
    "Individual message operations by ID require message to be received first";
const ERROR_BULK_OPERATIONS: &str = "Bulk operations require message to be received first";

/// Handles queue-related commands
pub struct QueueCommandHandler {
    consumer_manager: Arc<Mutex<ConsumerManager>>,
    statistics_service: Arc<QueueStatisticsService>,
}

impl QueueCommandHandler {
    pub fn new(
        consumer_manager: Arc<Mutex<ConsumerManager>>,
        statistics_service: Arc<QueueStatisticsService>,
    ) -> Self {
        Self {
            consumer_manager,
            statistics_service,
        }
    }

    pub async fn handle_switch_queue(
        &self,
        queue_name: String,
        queue_type: QueueType,
    ) -> ServiceBusResult<ServiceBusResponse> {
        let queue_info = QueueInfo::new(queue_name, queue_type);
        let mut manager = self.consumer_manager.lock().await;
        manager.switch_queue(queue_info.clone()).await?;
        Ok(ServiceBusResponse::QueueSwitched { queue_info })
    }

    pub async fn handle_get_current_queue(&self) -> ServiceBusResult<ServiceBusResponse> {
        let manager = self.consumer_manager.lock().await;
        let queue_info = manager.current_queue().cloned();
        Ok(ServiceBusResponse::CurrentQueue { queue_info })
    }

    pub async fn handle_get_queue_statistics(
        &self,
        queue_name: String,
        queue_type: QueueType,
    ) -> ServiceBusResult<ServiceBusResponse> {
        log::debug!(
            "Getting real statistics for queue: {} (type: {:?})",
            queue_name,
            queue_type
        );

        let retrieved_at = chrono::Utc::now();

        // Get both active and dead letter counts from Azure Management API
        let (active_count, dlq_count) = self
            .statistics_service
            .get_both_queue_counts(&queue_name)
            .await;

        log::debug!(
            "Retrieved stats for queue '{}': active={:?}, dlq={:?}",
            queue_name,
            active_count,
            dlq_count
        );

        Ok(ServiceBusResponse::QueueStatistics {
            queue_name,
            queue_type,
            active_message_count: active_count,
            dead_letter_message_count: dlq_count,
            retrieved_at,
        })
    }
}

/// Handles message retrieval commands
pub struct MessageCommandHandler {
    consumer_manager: Arc<Mutex<ConsumerManager>>,
}

impl MessageCommandHandler {
    pub fn new(consumer_manager: Arc<Mutex<ConsumerManager>>) -> Self {
        Self { consumer_manager }
    }

    pub async fn handle_peek_messages(
        &self,
        max_count: u32,
        from_sequence: Option<i64>,
    ) -> ServiceBusResult<ServiceBusResponse> {
        let manager = self.consumer_manager.lock().await;
        let messages = manager.peek_messages(max_count, from_sequence).await?;
        Ok(ServiceBusResponse::MessagesReceived { messages })
    }

    pub async fn handle_receive_messages(
        &self,
        max_count: u32,
    ) -> ServiceBusResult<ServiceBusResponse> {
        let manager = self.consumer_manager.lock().await;
        let messages = manager.receive_messages(max_count).await?;
        Ok(ServiceBusResponse::ReceivedMessages { messages })
    }

    pub async fn handle_complete_message(
        &self,
        _message_id: String,
    ) -> ServiceBusResult<ServiceBusResponse> {
        Err(ServiceBusError::InternalError(
            ERROR_INDIVIDUAL_MSG_OPERATIONS.to_string(),
        ))
    }

    pub async fn handle_abandon_message(
        &self,
        _message_id: String,
    ) -> ServiceBusResult<ServiceBusResponse> {
        Err(ServiceBusError::InternalError(
            ERROR_INDIVIDUAL_MSG_OPERATIONS.to_string(),
        ))
    }

    pub async fn handle_dead_letter_message(
        &self,
        _message_id: String,
        _reason: Option<String>,
        _error_description: Option<String>,
    ) -> ServiceBusResult<ServiceBusResponse> {
        Err(ServiceBusError::InternalError(
            ERROR_INDIVIDUAL_MSG_OPERATIONS.to_string(),
        ))
    }
}

/// Handles bulk operation commands
pub struct BulkCommandHandler {
    bulk_handler: Arc<BulkOperationHandler>,
    consumer_manager: Arc<Mutex<ConsumerManager>>,
    producer_manager: Arc<Mutex<ProducerManager>>,
    batch_config: BatchConfig,
}

impl BulkCommandHandler {
    pub fn new(
        bulk_handler: Arc<BulkOperationHandler>,
        consumer_manager: Arc<Mutex<ConsumerManager>>,
        producer_manager: Arc<Mutex<ProducerManager>>,
        batch_config: BatchConfig,
    ) -> Self {
        Self {
            bulk_handler,
            consumer_manager,
            producer_manager,
            batch_config,
        }
    }

    pub async fn handle_bulk_complete(
        &self,
        _message_ids: Vec<MessageIdentifier>,
    ) -> ServiceBusResult<ServiceBusResponse> {
        Err(ServiceBusError::InternalError(
            ERROR_BULK_OPERATIONS.to_string(),
        ))
    }

    pub async fn handle_bulk_delete(
        &self,
        message_ids: Vec<MessageIdentifier>,
        max_position: usize,
    ) -> ServiceBusResult<ServiceBusResponse> {
        log::info!(
            "Starting bulk delete operation for {} messages",
            message_ids.len()
        );

        let (consumer, queue_name) = {
            let manager = self.consumer_manager.lock().await;
            let consumer_arc = manager
                .get_raw_consumer()
                .ok_or(ServiceBusError::ConsumerNotFound)?
                .clone();
            let queue = manager
                .current_queue()
                .ok_or(ServiceBusError::ConsumerNotFound)?
                .name
                .clone();
            (consumer_arc, queue)
        };

        // Validate that we have messages to work with
        if message_ids.is_empty() {
            log::warn!("Bulk delete called with no message IDs");
            let result = BulkOperationResult::new(0);
            return Ok(ServiceBusResponse::BulkOperationCompleted { result });
        }

        // Log which queue we're deleting from for debugging
        log::info!("Bulk delete operating on queue: {}", queue_name);

        match self
            .bulk_handler
            .delete_messages(consumer, queue_name, message_ids, max_position)
            .await
        {
            Ok(result) => {
                log::info!(
                    "Bulk delete completed: {} successful, {} failed, {} not found",
                    result.successful,
                    result.failed,
                    result.not_found
                );
                Ok(ServiceBusResponse::BulkOperationCompleted { result })
            }
            Err(e) => {
                log::error!("Bulk delete failed: {}", e);
                Err(ServiceBusError::BulkOperationFailed(format!(
                    "Bulk delete failed: {}",
                    e
                )))
            }
        }
    }

    pub async fn handle_bulk_abandon(
        &self,
        _message_ids: Vec<MessageIdentifier>,
    ) -> ServiceBusResult<ServiceBusResponse> {
        Err(ServiceBusError::InternalError(
            ERROR_BULK_OPERATIONS.to_string(),
        ))
    }

    pub async fn handle_bulk_dead_letter(
        &self,
        _message_ids: Vec<MessageIdentifier>,
        _reason: Option<String>,
        _error_description: Option<String>,
    ) -> ServiceBusResult<ServiceBusResponse> {
        Err(ServiceBusError::InternalError(
            ERROR_BULK_OPERATIONS.to_string(),
        ))
    }

    pub async fn handle_bulk_send(
        &self,
        message_ids: Vec<MessageIdentifier>,
        target_queue: String,
        should_delete_source: bool,
        repeat_count: usize,
        max_position: usize,
    ) -> ServiceBusResult<ServiceBusResponse> {
        log::info!(
            "Starting bulk send: {} -> {}, delete_source={}, repeat={}",
            message_ids.len(),
            target_queue,
            should_delete_source,
            repeat_count
        );

        // Check if this is a DLQ operation
        let is_dlq_operation = target_queue.ends_with("/$deadletterqueue");

        // Acquire consumer for receiving original messages
        let consumer_arc = {
            let manager = self.consumer_manager.lock().await;
            manager
                .get_raw_consumer()
                .ok_or(ServiceBusError::ConsumerNotFound)?
                .clone()
        };

        // Create lookup for quick match
        let mut remaining: HashMap<String, MessageIdentifier> = message_ids
            .iter()
            .map(|m| (m.id.clone(), m.clone()))
            .collect();

        let mut matched_messages = Vec::<azservicebus::ServiceBusReceivedMessage>::new();
        let mut message_bytes = Vec::<Vec<u8>>::new();

        {
            // Lock consumer for receive operations
            let mut consumer = consumer_arc.lock().await;

            let batch_size = self.batch_config.bulk_chunk_size() as u32;
            let mut processed_count = 0;
            while !remaining.is_empty() && processed_count < max_position {
                let batch = match consumer
                    .receive_messages_with_timeout(
                        batch_size,
                        Duration::from_secs(self.batch_config.operation_timeout_secs()),
                    )
                    .await
                {
                    Ok(msgs) => msgs,
                    Err(e) => {
                        log::error!("Receive error during bulk send: {}", e);
                        break;
                    }
                };

                if batch.is_empty() {
                    log::debug!(
                        "Receive batch empty after processing {} messages",
                        processed_count
                    );
                    break;
                }

                let batch_len = batch.len();
                for msg in batch.into_iter() {
                    let msg_id = msg.message_id().map(|s| s.to_string()).unwrap_or_default();

                    if remaining.remove(&msg_id).is_some() {
                        if !is_dlq_operation {
                            // For regular send operations, extract body bytes
                            let body_bytes = match msg.body() {
                                Ok(bytes) => bytes.to_vec(),
                                Err(_) => Vec::new(),
                            };
                            message_bytes.push(body_bytes);
                        }
                        matched_messages.push(msg);
                    } else {
                        // Not a target, abandon it immediately
                        if let Err(e) = consumer.abandon_message(&msg).await {
                            log::warn!("Failed to abandon non-target message: {}", e);
                        }
                    }
                }

                processed_count += batch_len;
            }
        }

        if is_dlq_operation {
            // For DLQ operations, use dead_letter_message on each matched message
            let mut successful_count = 0;
            let mut failed_count = 0;

            {
                let mut consumer = consumer_arc.lock().await;
                for msg in &matched_messages {
                    match consumer
                        .dead_letter_message(msg, Some("Bulk moved to DLQ".to_string()), None)
                        .await
                    {
                        Ok(()) => {
                            successful_count += 1;
                            log::debug!(
                                "Successfully dead lettered message: {:?}",
                                msg.message_id()
                            );
                        }
                        Err(e) => {
                            failed_count += 1;
                            log::error!("Failed to dead letter message: {}", e);
                        }
                    }
                }
            }

            // Build BulkOperationResult for DLQ operation
            let mut result = BulkOperationResult::new(message_ids.len());
            result.successful = successful_count;
            result.not_found = remaining.len();
            result.failed = failed_count;

            return Ok(ServiceBusResponse::BulkOperationCompleted { result });
        }

        // For regular send operations (not DLQ), use producer manager
        let mut producer_mgr = self.producer_manager.lock().await;
        let stats = producer_mgr
            .send_raw_messages(&target_queue, message_bytes.clone(), repeat_count)
            .await
            .map_err(|e| {
                ServiceBusError::BulkOperationFailed(format!("Bulk send failed: {}", e))
            })?;

        // Complete or abandon original messages as required
        {
            let mut consumer = consumer_arc.lock().await;
            for msg in &matched_messages {
                let res = if should_delete_source {
                    consumer.complete_message(msg).await
                } else {
                    consumer.abandon_message(msg).await
                };
                if let Err(e) = res {
                    log::warn!(
                        "Failed to finalise message {}: {}",
                        msg.sequence_number(),
                        e
                    );
                }
            }
        }

        // Build BulkOperationResult
        let mut result = BulkOperationResult::new(message_ids.len());
        result.successful = matched_messages.len();
        result.not_found = remaining.len();
        result.failed = stats.failed / repeat_count; // approximate failures per original message

        Ok(ServiceBusResponse::BulkOperationCompleted { result })
    }

    pub async fn handle_bulk_send_peeked(
        &self,
        messages_data: Vec<(MessageIdentifier, Vec<u8>)>,
        target_queue: String,
        repeat_count: usize,
    ) -> ServiceBusResult<ServiceBusResponse> {
        log::info!(
            "Bulk send (peeked) {} messages to {} (repeat={})",
            messages_data.len(),
            target_queue,
            repeat_count
        );

        // Extract raw bytes
        let raw_vec: Vec<Vec<u8>> = messages_data
            .iter()
            .map(|(_id, data)| data.clone())
            .collect();

        let mut producer_mgr = self.producer_manager.lock().await;
        let stats = producer_mgr
            .send_raw_messages(&target_queue, raw_vec, repeat_count)
            .await
            .map_err(|e| {
                ServiceBusError::BulkOperationFailed(format!("Bulk send failed: {}", e))
            })?;

        Ok(ServiceBusResponse::MessagesSent {
            queue_name: target_queue,
            count: stats.total,
            stats,
        })
    }
}

/// Handles sending operation commands
pub struct SendCommandHandler {
    producer_manager: Arc<Mutex<ProducerManager>>,
}

impl SendCommandHandler {
    pub fn new(producer_manager: Arc<Mutex<ProducerManager>>) -> Self {
        Self { producer_manager }
    }

    pub async fn handle_send_message(
        &self,
        queue_name: String,
        message: MessageData,
    ) -> ServiceBusResult<ServiceBusResponse> {
        let mut manager = self.producer_manager.lock().await;
        manager.send_message(&queue_name, message).await?;
        Ok(ServiceBusResponse::MessageSent {
            queue_name: queue_name.clone(),
        })
    }

    pub async fn handle_send_messages(
        &self,
        queue_name: String,
        messages: Vec<MessageData>,
    ) -> ServiceBusResult<ServiceBusResponse> {
        let count = messages.len();
        let mut manager = self.producer_manager.lock().await;
        manager.send_messages(&queue_name, messages).await?;

        let mut stats = super::types::OperationStats::new();
        for _ in 0..count {
            stats.add_success();
        }

        Ok(ServiceBusResponse::MessagesSent {
            queue_name: queue_name.clone(),
            count,
            stats,
        })
    }
}

/// Handles status and health check commands
pub struct StatusCommandHandler {
    consumer_manager: Arc<Mutex<ConsumerManager>>,
    producer_manager: Arc<Mutex<ProducerManager>>,
}

impl StatusCommandHandler {
    pub fn new(
        consumer_manager: Arc<Mutex<ConsumerManager>>,
        producer_manager: Arc<Mutex<ProducerManager>>,
    ) -> Self {
        Self {
            consumer_manager,
            producer_manager,
        }
    }

    pub async fn handle_get_connection_status(&self) -> ServiceBusResult<ServiceBusResponse> {
        let consumer = self.consumer_manager.lock().await;
        let producer = self.producer_manager.lock().await;

        let connected = consumer.is_consumer_ready() || producer.producer_count() > 0;
        let current_queue = consumer.current_queue().cloned();

        Ok(ServiceBusResponse::ConnectionStatus {
            connected,
            current_queue,
            last_error: None,
        })
    }

    pub async fn handle_get_queue_stats(
        &self,
        queue_name: String,
    ) -> ServiceBusResult<ServiceBusResponse> {
        let consumer = self.consumer_manager.lock().await;
        Ok(ServiceBusResponse::QueueStats {
            queue_name: queue_name.clone(),
            message_count: None,
            active_consumer: consumer.is_consumer_ready(),
        })
    }
}

/// Handles resource management commands
pub struct ResourceCommandHandler {
    consumer_manager: Arc<Mutex<ConsumerManager>>,
    producer_manager: Arc<Mutex<ProducerManager>>,
}

impl ResourceCommandHandler {
    pub fn new(
        consumer_manager: Arc<Mutex<ConsumerManager>>,
        producer_manager: Arc<Mutex<ProducerManager>>,
    ) -> Self {
        Self {
            consumer_manager,
            producer_manager,
        }
    }

    pub async fn handle_dispose_consumer(&self) -> ServiceBusResult<ServiceBusResponse> {
        let mut manager = self.consumer_manager.lock().await;
        manager.dispose_consumer().await?;
        Ok(ServiceBusResponse::ConsumerDisposed)
    }

    pub async fn handle_dispose_all_resources(&self) -> ServiceBusResult<ServiceBusResponse> {
        let mut consumer = self.consumer_manager.lock().await;
        let mut producer = self.producer_manager.lock().await;
        consumer.dispose_consumer().await?;
        producer.dispose_all_producers().await?;
        Ok(ServiceBusResponse::AllResourcesDisposed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service_bus_manager::types::QueueType;

    #[test]
    fn test_error_constants() {
        // Test that error messages are descriptive
        assert!(ERROR_INDIVIDUAL_MSG_OPERATIONS.contains("require message to be received"));
        assert!(ERROR_BULK_OPERATIONS.contains("require message to be received"));
    }

    #[test]
    fn test_queue_info_creation() {
        let queue_info = QueueInfo::new("test_queue".to_string(), QueueType::Main);
        assert_eq!(queue_info.name, "test_queue");
        assert_eq!(queue_info.queue_type, QueueType::Main);
    }

    #[test]
    fn test_message_identifier_creation() {
        use crate::bulk_operations::MessageIdentifier;

        let msg_id = MessageIdentifier::new("test_id".to_string(), 123);
        assert_eq!(msg_id.id, "test_id");
        assert_eq!(msg_id.sequence, 123);
    }

    #[test]
    fn test_error_message_consistency() {
        // Test that error constants are used consistently
        assert_ne!(ERROR_INDIVIDUAL_MSG_OPERATIONS, ERROR_BULK_OPERATIONS);

        // Ensure all error messages provide helpful context
        assert!(ERROR_INDIVIDUAL_MSG_OPERATIONS.len() > 10);
        assert!(ERROR_BULK_OPERATIONS.len() > 10);
    }
}
