use super::consumer_manager::ConsumerManager;
use super::producer_manager::ProducerManager;
use super::queue_statistics_service::QueueStatisticsService;
use super::types::{QueueInfo, QueueType};

use crate::bulk_operations::BulkOperationResult;
use crate::bulk_operations::{BulkOperationHandler, MessageIdentifier, types::BatchConfig};
use crate::consumer::Consumer;
use crate::service_bus_manager::{
    errors::ServiceBusError, responses::ServiceBusResponse, types::MessageData,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Parameters for target message processing in bulk send
#[derive(Debug)]
struct TargetMessageParams<'a> {
    consumer: &'a mut Consumer,
    msg: &'a azservicebus::ServiceBusReceivedMessage,
    is_dlq_operation: bool,
    should_delete_source: bool,
    message_bytes: &'a mut Vec<Vec<u8>>,
    successful_count: &'a mut usize,
    failed_count: &'a mut usize,
}

/// Parameters for bulk send result finalization
#[derive(Debug)]
struct BulkSendResultParams {
    message_ids: Vec<MessageIdentifier>,
    successful_count: usize,
    failed_count: usize,
}

/// Result type for service bus operations
type ServiceBusResult<T> = Result<T, ServiceBusError>;

// Error message constants
const ERROR_INDIVIDUAL_MSG_OPERATIONS: &str =
    "Individual message operations by ID require message to be received first";
const ERROR_BULK_OPERATIONS: &str = "Bulk operations require message to be received first";

/// Handles queue-related commands including queue switching and statistics.
///
/// Provides functionality for managing queue connections, retrieving queue
/// information, and gathering comprehensive statistics from Azure Service Bus.
///
/// # Examples
///
/// ```no_run
/// use quetty_server::service_bus_manager::command_handlers::QueueCommandHandler;
///
/// let handler = QueueCommandHandler::new(consumer_manager, statistics_service);
///
/// // Switch to a different queue
/// let response = handler.handle_switch_queue(
///     "orders".to_string(),
///     QueueType::Main
/// ).await?;
///
/// // Get current queue information
/// let response = handler.handle_get_current_queue().await?;
/// ```
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
        log::debug!("Getting real statistics for queue: {queue_name} (type: {queue_type:?})");

        let retrieved_at = chrono::Utc::now();

        // Get both active and dead letter counts from Azure Management API
        let (active_count, dlq_count) = self
            .statistics_service
            .get_both_queue_counts(&queue_name)
            .await;

        log::debug!(
            "Retrieved stats for queue '{queue_name}': active={active_count:?}, dlq={dlq_count:?}"
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

/// Handles message retrieval commands including peek and receive operations.
///
/// Provides functionality for retrieving messages from Service Bus queues
/// in different modes - peeking (non-destructive) and receiving (with locks).
/// Individual message operations are handled differently and require special setup.
///
/// # Examples
///
/// ```no_run
/// use quetty_server::service_bus_manager::command_handlers::MessageCommandHandler;
///
/// let handler = MessageCommandHandler::new(consumer_manager);
///
/// // Peek at messages without removing them
/// let response = handler.handle_peek_messages(10, None).await?;
///
/// // Receive messages with locks for processing
/// let response = handler.handle_receive_messages(5).await?;
/// ```
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

/// Handles bulk operation commands for efficient processing of multiple messages.
///
/// Provides functionality for bulk message operations including delete, send,
/// complete, abandon, and dead letter operations. Uses optimized batching
/// strategies to handle large numbers of messages efficiently.
///
/// # Features
///
/// - **Bulk Delete** - Efficient deletion of multiple messages
/// - **Bulk Send** - Send multiple messages to target queues
/// - **Bulk Complete/Abandon** - Process multiple received messages
/// - **Batch Optimization** - Smart batching based on operation size
///
/// # Examples
///
/// ```no_run
/// use quetty_server::service_bus_manager::command_handlers::BulkCommandHandler;
///
/// let handler = BulkCommandHandler::new(
///     bulk_handler,
///     consumer_manager,
///     producer_manager,
///     batch_config
/// );
///
/// // Bulk delete messages
/// let response = handler.handle_bulk_delete(
///     message_ids,
///     1000  // max position
/// ).await?;
/// ```
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
        log::info!("Bulk delete operating on queue: {queue_name}");

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
                log::error!("Bulk delete failed: {e}");
                Err(ServiceBusError::BulkOperationFailed(format!(
                    "Bulk delete failed: {e}"
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
        _max_position: usize,
    ) -> ServiceBusResult<ServiceBusResponse> {
        // Wrap the entire operation in a timeout
        let operation_timeout = Duration::from_secs(self.batch_config.operation_timeout_secs());

        match tokio::time::timeout(
            operation_timeout,
            self.handle_bulk_send_internal(
                message_ids,
                target_queue,
                should_delete_source,
                repeat_count,
                _max_position,
            ),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => {
                log::error!(
                    "Bulk send operation timed out after {} seconds",
                    self.batch_config.operation_timeout_secs()
                );
                Err(ServiceBusError::OperationTimeout(format!(
                    "Bulk send operation timed out after {} seconds",
                    self.batch_config.operation_timeout_secs()
                )))
            }
        }
    }

    async fn handle_bulk_send_internal(
        &self,
        message_ids: Vec<MessageIdentifier>,
        target_queue: String,
        should_delete_source: bool,
        repeat_count: usize,
        _max_position: usize,
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

        // Setup operation state
        let (
            consumer_arc,
            mut remaining,
            mut message_bytes,
            mut successful_count,
            mut failed_count,
        ) = self.setup_bulk_send_operation(&message_ids).await?;

        // Main processing loop
        let batch_size = self.batch_config.bulk_chunk_size() as u32;
        let mut processed_count = 0;
        let mut highest_sequence_seen = 0i64;
        let target_max_sequence = message_ids
            .iter()
            .map(|msg_id| msg_id.sequence)
            .max()
            .unwrap_or(0);
        let mut pending_non_targets: Vec<azservicebus::ServiceBusReceivedMessage> = Vec::new();
        let mut consecutive_empty_batches = 0;
        let max_empty_batches = 3; // Exit after 3 consecutive empty batches

        while self.should_continue_bulk_send(
            &remaining,
            target_max_sequence,
            highest_sequence_seen,
            consecutive_empty_batches,
            max_empty_batches,
        ) {
            // Acquire consumer lock per batch instead of holding it for entire operation
            let mut consumer = consumer_arc.lock().await;
            let batch = match consumer
                .receive_messages_with_timeout(
                    batch_size,
                    Duration::from_secs(self.batch_config.receive_timeout_secs()),
                )
                .await
            {
                Ok(msgs) => msgs,
                Err(e) => {
                    log::error!("Receive error during bulk send: {e}");
                    drop(consumer); // Release lock before breaking
                    break;
                }
            };

            if batch.is_empty() {
                consecutive_empty_batches += 1;
                log::debug!(
                    "Receive batch empty after processing {processed_count} messages (highest_sequence: {highest_sequence_seen}), consecutive empty: {consecutive_empty_batches}"
                );
                drop(consumer); // Release lock before continuing
                continue;
            } else {
                consecutive_empty_batches = 0; // Reset counter on successful batch
            }

            let batch_len = batch.len();
            for msg in batch.into_iter() {
                let msg_id = msg.message_id().map(|s| s.to_string()).unwrap_or_default();
                let msg_sequence = msg.sequence_number();
                if msg_sequence > highest_sequence_seen {
                    highest_sequence_seen = msg_sequence;
                }
                if remaining.remove(&msg_id).is_some() {
                    let params = TargetMessageParams {
                        consumer: &mut consumer,
                        msg: &msg,
                        is_dlq_operation,
                        should_delete_source,
                        message_bytes: &mut message_bytes,
                        successful_count: &mut successful_count,
                        failed_count: &mut failed_count,
                    };
                    self.process_target_message(params).await;
                } else {
                    pending_non_targets.push(msg);
                }
            }
            processed_count += batch_len;

            // Release consumer lock after processing batch
            drop(consumer);

            if processed_count % (batch_size as usize * 10) == 0 {
                log::info!(
                    "Bulk send progress: processed {} messages, highest_sequence: {}, remaining targets: {}",
                    processed_count,
                    highest_sequence_seen,
                    remaining.len()
                );
            }
            if target_max_sequence > 0
                && highest_sequence_seen > target_max_sequence + 1000
                && !remaining.is_empty()
            {
                log::warn!(
                    "Safety break: highest_sequence {} exceeds target {} by 1000+, {} targets still remaining",
                    highest_sequence_seen,
                    target_max_sequence,
                    remaining.len()
                );
                break;
            }
        }

        log::info!(
            "Bulk send scan completed: processed {} messages, highest_sequence: {}, targets found: {}, remaining: {}",
            processed_count,
            highest_sequence_seen,
            successful_count,
            remaining.len()
        );

        // Abandon non-target messages (acquire lock one final time)
        if !pending_non_targets.is_empty() {
            let mut consumer = consumer_arc.lock().await;
            self.abandon_pending_non_targets(&mut consumer, pending_non_targets)
                .await;
        }
        let params = BulkSendResultParams {
            message_ids,
            successful_count,
            failed_count,
        };
        self.finalize_bulk_send_result(params)
    }

    async fn setup_bulk_send_operation(
        &self,
        message_ids: &[MessageIdentifier],
    ) -> ServiceBusResult<(
        Arc<Mutex<Consumer>>,
        HashMap<String, MessageIdentifier>,
        Vec<Vec<u8>>,
        usize,
        usize,
    )> {
        let consumer_arc = {
            let manager = self.consumer_manager.lock().await;
            manager
                .get_raw_consumer()
                .ok_or(ServiceBusError::ConsumerNotFound)?
                .clone()
        };
        let remaining: HashMap<String, MessageIdentifier> = message_ids
            .iter()
            .map(|m| (m.id.clone(), m.clone()))
            .collect();
        let message_bytes: Vec<Vec<u8>> = Vec::new();
        let successful_count: usize = 0;
        let failed_count: usize = 0;
        Ok((
            consumer_arc,
            remaining,
            message_bytes,
            successful_count,
            failed_count,
        ))
    }

    fn should_continue_bulk_send(
        &self,
        remaining: &HashMap<String, MessageIdentifier>,
        target_max_sequence: i64,
        highest_sequence_seen: i64,
        consecutive_empty_batches: u32,
        max_empty_batches: u32,
    ) -> bool {
        // Exit if no more targets remain
        if remaining.is_empty() {
            return false;
        }

        // Exit if too many consecutive empty batches (likely no more messages available)
        if consecutive_empty_batches >= max_empty_batches {
            log::warn!(
                "Stopping bulk send after {} consecutive empty batches, {} targets still remaining",
                consecutive_empty_batches,
                remaining.len()
            );
            return false;
        }

        // Exit if we've gone far beyond the target sequence range
        if target_max_sequence > 0 && highest_sequence_seen > target_max_sequence {
            return false;
        }

        true
    }

    async fn process_target_message(&self, params: TargetMessageParams<'_>) {
        if params.is_dlq_operation {
            if let Err(e) = params
                .consumer
                .dead_letter_message(params.msg, Some("Bulk moved to DLQ".to_string()), None)
                .await
            {
                *params.failed_count += 1;
                log::error!(
                    "Failed to dead letter message {:?}: {}",
                    params.msg.message_id(),
                    e
                );
                return;
            }
            *params.successful_count += 1;
        } else {
            if let Ok(body) = params.msg.body() {
                params.message_bytes.push(body.to_vec());
            }
            let res = if params.should_delete_source {
                params.consumer.complete_message(params.msg).await
            } else {
                params.consumer.abandon_message(params.msg).await
            };
            if let Err(e) = res {
                *params.failed_count += 1;
                log::error!(
                    "Failed to finalise original message {:?}: {}",
                    params.msg.message_id(),
                    e
                );
                return;
            }
            *params.successful_count += 1;
        }
    }

    async fn abandon_pending_non_targets(
        &self,
        consumer: &mut Consumer,
        pending_non_targets: Vec<azservicebus::ServiceBusReceivedMessage>,
    ) {
        if !pending_non_targets.is_empty() {
            log::info!(
                "Abandoning {} non-target messages accumulated during scan",
                pending_non_targets.len()
            );
            for msg in pending_non_targets.into_iter() {
                if let Err(e) = consumer.abandon_message(&msg).await {
                    log::warn!("Failed to abandon non-target message after scan: {e}");
                }
            }
        }
    }

    fn finalize_bulk_send_result(
        &self,
        params: BulkSendResultParams,
    ) -> ServiceBusResult<ServiceBusResponse> {
        let mut result = BulkOperationResult::new(params.message_ids.len());
        result.successful = params.successful_count;
        result.failed = params.failed_count;
        result.not_found = params
            .message_ids
            .len()
            .saturating_sub(params.successful_count + params.failed_count);
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
            .map_err(|e| ServiceBusError::BulkOperationFailed(format!("Bulk send failed: {e}")))?;

        Ok(ServiceBusResponse::MessagesSent {
            queue_name: target_queue,
            count: stats.total,
            stats,
        })
    }
}

/// Handles message sending commands for single and multiple message operations.
///
/// Provides functionality for sending messages to Service Bus queues using
/// the producer manager. Supports both single message sends and batch sending
/// operations with comprehensive error handling and statistics tracking.
///
/// # Examples
///
/// ```no_run
/// use quetty_server::service_bus_manager::command_handlers::SendCommandHandler;
///
/// let handler = SendCommandHandler::new(producer_manager);
///
/// // Send a single message
/// let response = handler.handle_send_message(
///     "target-queue".to_string(),
///     message_data
/// ).await?;
///
/// // Send multiple messages
/// let response = handler.handle_send_messages(
///     "target-queue".to_string(),
///     vec![message1, message2, message3]
/// ).await?;
/// ```
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

/// Handles status and health check commands for monitoring Service Bus connections.
///
/// Provides functionality for checking connection health, queue status, and
/// overall system state. Monitors both consumer and producer managers to
/// provide comprehensive status information.
///
/// # Examples
///
/// ```no_run
/// use quetty_server::service_bus_manager::command_handlers::StatusCommandHandler;
///
/// let handler = StatusCommandHandler::new(consumer_manager, producer_manager);
///
/// // Check overall connection status
/// let response = handler.handle_get_connection_status().await?;
///
/// // Get basic queue statistics
/// let response = handler.handle_get_queue_stats(
///     "orders".to_string()
/// ).await?;
/// ```
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

/// Handles resource management commands for cleanup and connection management.
///
/// Provides functionality for disposing of Service Bus resources, resetting
/// connections, and performing cleanup operations. Essential for proper
/// resource lifecycle management and error recovery.
///
/// # Examples
///
/// ```no_run
/// use quetty_server::service_bus_manager::command_handlers::ResourceCommandHandler;
///
/// let handler = ResourceCommandHandler::new(consumer_manager, producer_manager);
///
/// // Dispose of current consumer
/// let response = handler.handle_dispose_consumer().await?;
///
/// // Clean up all resources
/// let response = handler.handle_dispose_all_resources().await?;
///
/// // Reset connection
/// let response = handler.handle_reset_connection().await?;
/// ```
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
