use super::consumer_manager::ConsumerManager;
use super::producer_manager::ProducerManager;
use super::types::{QueueInfo, QueueType};
use crate::bulk_operations::{BulkOperationHandler, MessageIdentifier};
use crate::service_bus_manager::{
    errors::ServiceBusError, responses::ServiceBusResponse, types::MessageData,
};
use azservicebus::{ServiceBusClient, core::BasicRetryPolicy};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Result type for service bus operations
type ServiceBusResult<T> = Result<T, ServiceBusError>;

// Error message constants
const ERROR_INDIVIDUAL_MSG_OPERATIONS: &str =
    "Individual message operations by ID require message to be received first";
const ERROR_BULK_OPERATIONS: &str = "Bulk operations require message to be received first";
const ERROR_PEEKED_TO_REGULAR_QUEUE: &str =
    "Sending peeked messages to regular queues (non-DLQ) is not supported";

/// Handles queue-related commands
pub struct QueueCommandHandler {
    consumer_manager: Arc<Mutex<ConsumerManager>>,
}

impl QueueCommandHandler {
    pub fn new(consumer_manager: Arc<Mutex<ConsumerManager>>) -> Self {
        Self { consumer_manager }
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
        log::debug!("Getting statistics for queue: {} (type: {:?})", queue_name, queue_type);
        
        let retrieved_at = chrono::Utc::now();
        
        // Get real queue statistics from the current queue
        let (active_count, dlq_count) = self.get_current_queue_statistics(&queue_name, &queue_type).await;
        
        log::debug!(
            "Retrieved stats for queue '{}' ({:?}): active={:?}, dlq={:?}",
            queue_name, queue_type, active_count, dlq_count
        );
        
        Ok(ServiceBusResponse::QueueStatistics {
            queue_name,
            queue_type,
            active_message_count: active_count,
            dead_letter_message_count: dlq_count,
            retrieved_at,
        })
    }

    /// Get statistics for the current queue by analyzing loaded messages
    async fn get_current_queue_statistics(&self, queue_name: &str, queue_type: &QueueType) -> (Option<u64>, Option<u64>) {
        let manager = self.consumer_manager.lock().await;
        
        // Only provide statistics if we're currently connected to the requested queue
        if let Some(current_queue) = manager.current_queue() {
            if current_queue.name == queue_name && current_queue.queue_type == *queue_type {
                // We're connected to the requested queue, try to get real statistics
                match self.estimate_current_queue_size(&manager).await {
                    Some(active_count) => {
                        match queue_type {
                            QueueType::Main => {
                                // For main queue, we can't easily get DLQ count without switching
                                // Return the active count and indicate DLQ count is unknown
                                (Some(active_count), None)
                            }
                            QueueType::DeadLetter => {
                                // For DLQ, there's no sub-DLQ
                                (Some(active_count), Some(0))
                            }
                        }
                    }
                    None => {
                        // Fall back to basic estimation
                        match queue_type {
                            QueueType::Main => (None, None),
                            QueueType::DeadLetter => (None, Some(0)),
                        }
                    }
                }
            } else {
                // Not connected to the requested queue, can't provide real statistics
                log::debug!("Cannot provide statistics for '{}' - currently connected to '{}'", 
                          queue_name, current_queue.name);
                (None, None)
            }
        } else {
            // No queue connected
            log::debug!("Cannot provide statistics - no queue currently connected");
            (None, None)
        }
    }

    /// Estimate the size of the currently connected queue
    async fn estimate_current_queue_size(&self, manager: &crate::service_bus_manager::consumer_manager::ConsumerManager) -> Option<u64> {
        // Try to peek messages to estimate queue size
        match manager.peek_messages(100, None).await {
            Ok(messages) if !messages.is_empty() => {
                let first_seq = messages.first().unwrap().sequence;
                let last_seq = messages.last().unwrap().sequence;
                
                // If we got a full batch (100 messages), try to estimate total by peeking further
                if messages.len() == 100 {
                    // Try to find the end of the queue by peeking with a high sequence number
                    match manager.peek_messages(1, Some(last_seq + 10000)).await {
                        Ok(end_messages) if !end_messages.is_empty() => {
                            let highest_seq = end_messages.first().unwrap().sequence;
                            let estimated_count = (highest_seq - first_seq + 1) as u64;
                            log::debug!("Estimated {} messages (seq range: {}-{})", estimated_count, first_seq, highest_seq);
                            Some(estimated_count)
                        }
                        _ => {
                            // Conservative estimate based on what we can see
                            let estimated_count = (last_seq - first_seq + 1) as u64;
                            log::debug!("Conservative estimate: {} messages (seq range: {}-{})", estimated_count, first_seq, last_seq);
                            Some(estimated_count)
                        }
                    }
                } else {
                    // Small queue, use actual count
                    let count = messages.len() as u64;
                    log::debug!("Small queue with {} messages", count);
                    Some(count)
                }
            }
            Ok(_) => {
                log::debug!("Queue appears to be empty");
                Some(0)
            }
            Err(e) => {
                log::warn!("Failed to peek messages for statistics: {}", e);
                None
            }
        }
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
    service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
}

impl BulkCommandHandler {
    pub fn new(
        bulk_handler: Arc<BulkOperationHandler>,
        consumer_manager: Arc<Mutex<ConsumerManager>>,
        service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
    ) -> Self {
        Self {
            bulk_handler,
            consumer_manager,
            service_bus_client,
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
    ) -> ServiceBusResult<ServiceBusResponse> {
        log::info!(
            "Starting bulk delete operation for {} messages",
            message_ids.len()
        );

        let consumer = {
            let manager = self.consumer_manager.lock().await;
            manager.get_raw_consumer()
                .ok_or(ServiceBusError::ConsumerNotFound)?
                .clone()
        };

        let context = crate::bulk_operations::BulkOperationContext::new(
            consumer,
            self.service_bus_client.clone(),
            String::new(),
        );

        let params = crate::bulk_operations::BulkSendParams::with_retrieval(
            String::new(),
            false,
            message_ids,
        );

        match self.bulk_handler.bulk_delete(context, &params).await {
            Ok(result) => {
                log::info!(
                    "Bulk delete completed: {} successful, {} failed",
                    result.successful,
                    result.failed
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
    ) -> ServiceBusResult<ServiceBusResponse> {
        log::info!(
            "Starting bulk send operation for {} messages to queue '{}' (delete_source: {}, repeat: {})",
            message_ids.len(),
            target_queue,
            should_delete_source,
            repeat_count
        );

        let consumer = {
            let manager = self.consumer_manager.lock().await;
            manager.get_raw_consumer()
                .ok_or(ServiceBusError::ConsumerNotFound)?
                .clone()
        };

        let operation_type =
            crate::bulk_operations::QueueOperationType::from_queue_name(&target_queue);
        log::debug!(
            "Determined operation type: {:?} for target queue: {}",
            operation_type,
            target_queue
        );

        let context = crate::bulk_operations::BulkOperationContext::new(
            consumer,
            self.service_bus_client.clone(),
            target_queue.clone(),
        );

        let params = crate::bulk_operations::BulkSendParams::with_retrieval(
            target_queue,
            should_delete_source,
            message_ids.clone(),
        );

        match self.bulk_handler.bulk_send(context, params).await {
            Ok(result) => {
                log::info!(
                    "Bulk send completed: {} successful, {} failed",
                    result.successful,
                    result.failed
                );
                Ok(ServiceBusResponse::BulkOperationCompleted { result })
            }
            Err(e) => {
                log::error!("Bulk send failed: {}", e);
                Err(ServiceBusError::BulkOperationFailed(format!(
                    "Bulk send failed: {}",
                    e
                )))
            }
        }
    }

    pub async fn handle_bulk_send_peeked(
        &self,
        messages_data: Vec<(MessageIdentifier, Vec<u8>)>,
        target_queue: String,
        should_delete_source: bool,
        _repeat_count: usize,
    ) -> ServiceBusResult<ServiceBusResponse> {
        log::info!(
            "Starting bulk send peeked operation for {} messages to queue '{}'",
            messages_data.len(),
            target_queue
        );

        let operation_type =
            crate::bulk_operations::QueueOperationType::from_queue_name(&target_queue);
        log::debug!(
            "Determined operation type: {:?} for target queue: {}",
            operation_type,
            target_queue
        );

        if let crate::bulk_operations::QueueOperationType::SendToDLQ = operation_type {
            let consumer = {
                let manager = self.consumer_manager.lock().await;
                manager.get_raw_consumer()
                    .ok_or(ServiceBusError::ConsumerNotFound)?
                    .clone()
            };

            let context = crate::bulk_operations::BulkOperationContext::new(
                consumer,
                self.service_bus_client.clone(),
                target_queue.clone(),
            );

            let params = crate::bulk_operations::BulkSendParams::with_message_data(
                target_queue.clone(),
                should_delete_source,
                messages_data,
            );

            match self.bulk_handler.bulk_send(context, params).await {
                Ok(result) => {
                    log::info!(
                        "Bulk send peeked completed: {} successful, {} failed",
                        result.successful,
                        result.failed
                    );
                    Ok(ServiceBusResponse::BulkOperationCompleted { result })
                }
                Err(e) => {
                    log::error!("Bulk send peeked failed: {}", e);
                    Err(ServiceBusError::BulkOperationFailed(format!(
                        "Bulk send peeked failed: {}",
                        e
                    )))
                }
            }
        } else {
            Err(ServiceBusError::InternalError(
                ERROR_PEEKED_TO_REGULAR_QUEUE.to_string(),
            ))
        }
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
        // Test that our error constants are not empty
        assert!(!ERROR_INDIVIDUAL_MSG_OPERATIONS.is_empty());
        assert!(!ERROR_BULK_OPERATIONS.is_empty());
        assert!(!ERROR_PEEKED_TO_REGULAR_QUEUE.is_empty());

        // Test that error messages are descriptive
        assert!(ERROR_INDIVIDUAL_MSG_OPERATIONS.contains("require message to be received"));
        assert!(ERROR_BULK_OPERATIONS.contains("require message to be received"));
        assert!(ERROR_PEEKED_TO_REGULAR_QUEUE.contains("not supported"));
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
        assert_ne!(ERROR_BULK_OPERATIONS, ERROR_PEEKED_TO_REGULAR_QUEUE);

        // Ensure all error messages provide helpful context
        assert!(ERROR_INDIVIDUAL_MSG_OPERATIONS.len() > 10);
        assert!(ERROR_BULK_OPERATIONS.len() > 10);
        assert!(ERROR_PEEKED_TO_REGULAR_QUEUE.len() > 10);
    }
}

