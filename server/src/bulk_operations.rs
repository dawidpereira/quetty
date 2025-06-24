use crate::consumer::Consumer;
use crate::producer::ServiceBusClientProducerExt;
use azservicebus::core::BasicRetryPolicy;
use azservicebus::{ServiceBusClient, ServiceBusMessage, ServiceBusSenderOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;

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
    /// Timeout for bulk operations (default: 300 seconds)
    operation_timeout_secs: Option<u64>,
    /// Buffer percentage for batch size calculation (default: 0.15 = 15%)
    buffer_percentage: Option<f64>,
    /// Minimum buffer size (default: 30)
    min_buffer_size: Option<usize>,
    /// Chunk size for bulk processing operations (default: 100)
    bulk_chunk_size: Option<usize>,
    /// Processing time limit for bulk operations in seconds (default: 30)
    bulk_processing_time_secs: Option<u64>,
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
        }
    }

    /// Get the maximum batch size for bulk operations
    pub fn max_batch_size(&self) -> u32 {
        // Note: We use 2048 as default here since server doesn't depend on ui
        // The ui module validates this against limits::AZURE_SERVICE_BUS_MAX_BATCH_SIZE
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
}

/// Context for Service Bus operations containing shared resources
#[derive(Debug, Clone)]
pub struct ServiceBusOperationContext {
    pub consumer: Arc<Mutex<Consumer>>,
    pub service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
    pub main_queue_name: String,
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
        }
    }
}

/// Handles bulk operations on Azure Service Bus queues
/// Parameters for processing a single batch of messages
struct BatchProcessingContext<'a> {
    consumer: Arc<Mutex<Consumer>>,
    batch_size: usize,
    target_messages_found: usize,
    target_map: &'a HashMap<String, MessageIdentifier>,
    messages_processed: usize,
    remaining_targets: &'a mut HashMap<String, MessageIdentifier>,
    target_messages_vec: &'a mut Vec<azservicebus::ServiceBusReceivedMessage>,
    non_target_messages: &'a mut Vec<azservicebus::ServiceBusReceivedMessage>,
}

pub struct BulkOperationHandler {
    config: BatchConfig,
}

impl BulkOperationHandler {
    pub fn new(config: BatchConfig) -> Self {
        Self { config }
    }

    /// Main entry point for bulk send operations
    pub async fn bulk_send(
        &self,
        context: BulkOperationContext,
        params: BulkSendParams,
    ) -> Result<BulkOperationResult, Box<dyn Error + Send + Sync>> {
        let total_requested = params.message_identifiers.len();
        let mut result = BulkOperationResult::new(total_requested);

        log::info!(
            "Starting bulk send operation for {} messages to queue: {}",
            total_requested,
            context.target_queue
        );

        if total_requested == 0 {
            log::warn!("No messages provided for bulk send operation");
            return Ok(result);
        }

        // Validate batch size and warn if necessary for order-sensitive operations
        if params.should_delete {
            log::warn!(
                "Bulk operation with delete enabled for {} messages. Messages will be permanently removed from source queue after successful processing.",
                total_requested
            );
        }

        // Execute the operation based on available data
        self.execute_bulk_send_operation(context, &params, &mut result)
            .await
    }

    /// Core implementation of bulk send operation
    async fn execute_bulk_send_operation(
        &self,
        context: BulkOperationContext,
        params: &BulkSendParams,
        result: &mut BulkOperationResult,
    ) -> Result<BulkOperationResult, Box<dyn Error + Send + Sync>> {
        match &params.messages_data {
            Some(messages_data) => {
                self.execute_bulk_send_with_data(context, params, messages_data.clone(), result)
                    .await
            }
            None => {
                self.execute_bulk_send_with_retrieval(context, params, result)
                    .await
            }
        }
    }

    /// Execute bulk send with pre-fetched message data
    async fn execute_bulk_send_with_data(
        &self,
        context: BulkOperationContext,
        params: &BulkSendParams,
        messages_data: Vec<(MessageIdentifier, Vec<u8>)>,
        result: &mut BulkOperationResult,
    ) -> Result<BulkOperationResult, Box<dyn Error + Send + Sync>> {
        log::info!(
            "Processing bulk send for {} messages with pre-fetched data",
            messages_data.len()
        );

        match context.operation_type {
            QueueOperationType::SendToQueue => {
                // Convert peeked message data to ServiceBusMessage objects
                let new_messages: Vec<ServiceBusMessage> = messages_data
                    .iter()
                    .map(|(_, data)| ServiceBusMessage::new(data.clone()))
                    .collect();

                // Send messages to target queue
                self.send_messages_to_queue(
                    &context.target_queue,
                    new_messages,
                    context.service_bus_client.clone(),
                )
                .await?;

                // Track all messages as successful since we can't selectively delete when using peek
                for (identifier, _) in messages_data {
                    result.add_successful_message(identifier);
                }
            }
            QueueOperationType::SendToDLQ => {
                // For DLQ operations with pre-fetched data, we need to convert data back to received messages
                // This is because dead_letter_message requires ServiceBusReceivedMessage objects
                log::info!("DLQ operation with pre-fetched data requires message retrieval");
                return self
                    .execute_bulk_send_with_retrieval(context, params, result)
                    .await;
            }
        }

        log::info!(
            "Bulk send with data completed: {} successful, {} failed",
            result.successful,
            result.failed
        );

        Ok(result.clone())
    }

    /// Execute bulk send with message retrieval from source queue
    async fn execute_bulk_send_with_retrieval(
        &self,
        context: BulkOperationContext,
        params: &BulkSendParams,
        result: &mut BulkOperationResult,
    ) -> Result<BulkOperationResult, Box<dyn Error + Send + Sync>> {
        let target_count = params.message_identifiers.len();

        // Calculate buffer size (percentage of targets or minimum)
        let buffer_size = std::cmp::max(
            (target_count as f64 * self.config.buffer_percentage()) as usize,
            self.config.min_buffer_size(),
        );

        let buffered_batch = target_count + buffer_size;
        let final_batch_size = std::cmp::min(buffered_batch, self.config.max_batch_size() as usize);

        log::info!(
            "Processing bulk send for {} selected messages using batch size {} ({}+{} buffer, capped at {})",
            target_count,
            final_batch_size,
            target_count,
            buffer_size,
            self.config.max_batch_size()
        );

        // Create a lookup map for quick message identification
        let target_map: HashMap<String, MessageIdentifier> = params
            .message_identifiers
            .iter()
            .map(|m| (m.id.clone(), m.clone()))
            .collect();

        // Phase 1: Collect target and non-target messages
        let (target_messages, non_target_messages) = self
            .collect_target_messages(context.consumer.clone(), &target_map, final_batch_size)
            .await?;

        // Phase 2: Process target messages based on operation type
        if !target_messages.is_empty() {
            let process_params = ProcessTargetMessagesParams::new(
                target_messages,
                &context,
                params,
                &target_map,
                result,
            );

            match self.process_target_messages(process_params).await {
                Ok(processed_count) => {
                    log::info!("Successfully processed {} target messages", processed_count);
                }
                Err(e) => {
                    let error_msg = format!("Failed to process target messages: {}", e);
                    log::error!("{}", error_msg);
                    result.add_failure(error_msg);
                }
            }
        }

        // Phase 3: Abandon non-target messages to make them available again
        self.abandon_non_target_messages(context.consumer, non_target_messages, result)
            .await?;

        // Calculate not found messages
        result.not_found = target_map.len() - result.successful;

        log::info!(
            "Bulk send operation completed: {} successful, {} failed, {} not found",
            result.successful,
            result.failed,
            result.not_found
        );

        Ok(result.clone())
    }

    /// Process target messages based on operation type
    async fn process_target_messages(
        &self,
        process_params: ProcessTargetMessagesParams<'_>,
    ) -> Result<usize, Box<dyn Error + Send + Sync>> {
        if process_params.messages.is_empty() {
            return Ok(0);
        }

        log::debug!(
            "Processing {} target messages",
            process_params.messages.len()
        );

        match process_params.context.operation_type {
            QueueOperationType::SendToQueue => {
                // Convert messages to new ServiceBusMessage objects for sending
                let new_messages: Vec<ServiceBusMessage> = process_params
                    .messages
                    .iter()
                    .filter_map(|msg| match msg.body() {
                        Ok(body) => Some(ServiceBusMessage::new(body.to_vec())),
                        Err(e) => {
                            log::error!("Failed to get message body: {}", e);
                            None
                        }
                    })
                    .collect();

                // Send messages to target queue
                self.send_messages_to_queue(
                    &process_params.context.target_queue,
                    new_messages,
                    process_params.context.service_bus_client.clone(),
                )
                .await?;
            }
            QueueOperationType::SendToDLQ => {
                if process_params.params.should_delete {
                    // Move operation: Use dead_letter_message operation (deletes from source)
                    log::debug!(
                        "Using dead_letter_message for move operation (should_delete=true)"
                    );
                    self.dead_letter_messages(
                        &process_params.messages,
                        process_params.context.consumer.clone(),
                    )
                    .await?;
                } else {
                    // Copy operation to DLQ: Azure Service Bus limitation
                    log::warn!("Copy operation to DLQ is not supported by Azure Service Bus");

                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Unsupported,
                        "Copy operation to Dead Letter Queue is not supported by Azure Service Bus. \
                         DLQ can only be written to via dead_letter_message operation, which always \
                         removes messages from the source queue. Use move operation (S key) instead, \
                         or consider using a regular queue as copy destination.",
                    )));
                }
            }
        }

        // Handle source message cleanup based on should_delete and operation type
        match process_params.context.operation_type {
            QueueOperationType::SendToDLQ if process_params.params.should_delete => {
                // For move operations with dead_letter_message, messages are already deleted
                log::debug!("Messages already deleted by dead_letter_message operation");
            }
            _ => {
                // For all other cases, handle based on should_delete parameter
                if process_params.params.should_delete {
                    self.complete_processed_messages(
                        &process_params.messages,
                        process_params.context,
                    )
                    .await?;
                } else {
                    // Abandon messages to make them available again in source queue
                    self.abandon_processed_messages(
                        &process_params.messages,
                        process_params.context.consumer.clone(),
                    )
                    .await?;
                }
            }
        }

        // Track successful message processing
        self.track_successful_messages(
            &process_params.messages,
            process_params.target_map,
            process_params.result,
        );

        log::info!(
            "Successfully processed {} messages",
            process_params.messages.len()
        );
        Ok(process_params.messages.len())
    }

    /// Dead letter multiple messages using the native dead_letter_message operation
    async fn dead_letter_messages(
        &self,
        messages: &[azservicebus::ServiceBusReceivedMessage],
        consumer: Arc<Mutex<Consumer>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        log::debug!("Dead lettering {} messages", messages.len());

        let mut consumer_guard = consumer.lock().await;

        for message in messages {
            let message_id = message
                .message_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            log::debug!("Dead lettering message: {}", message_id);

            consumer_guard
                .dead_letter_message(
                    message,
                    Some("Bulk dead letter operation".to_string()),
                    Some("Message sent to DLQ via bulk operation".to_string()),
                )
                .await
                .map_err(|e| -> Box<dyn Error + Send + Sync> {
                    log::error!("Failed to dead letter message {}: {}", message_id, e);
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to dead letter message {}: {}", message_id, e),
                    ))
                })?;
        }

        drop(consumer_guard);
        log::info!("Successfully dead lettered {} messages", messages.len());
        Ok(())
    }

    /// Abandon processed messages (for operations where we don't want to delete from source)
    async fn abandon_processed_messages(
        &self,
        messages: &[azservicebus::ServiceBusReceivedMessage],
        consumer: Arc<Mutex<Consumer>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        log::debug!("Abandoning {} processed messages", messages.len());
        let mut consumer_guard = consumer.lock().await;
        consumer_guard.abandon_messages(messages).await.map_err(
            |e| -> Box<dyn Error + Send + Sync> {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to abandon messages: {}", e),
                ))
            },
        )?;
        drop(consumer_guard);
        Ok(())
    }

    /// Send multiple messages to a queue using batch operations
    async fn send_messages_to_queue(
        &self,
        queue_name: &str,
        messages: Vec<ServiceBusMessage>,
        service_bus_client: Arc<Mutex<ServiceBusClient<azservicebus::core::BasicRetryPolicy>>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        if messages.is_empty() {
            return Ok(());
        }

        log::debug!("Creating producer for queue: {}", queue_name);

        let mut client = service_bus_client.lock().await;
        let mut producer = client
            .create_producer_for_queue(queue_name, ServiceBusSenderOptions::default())
            .await
            .map_err(|e| -> Box<dyn Error + Send + Sync> {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create producer for queue {}: {}", queue_name, e),
                ))
            })?;

        // For large batches, chunk into smaller groups to prevent Azure Service Bus timeouts
        let max_chunk_size = self.config.bulk_chunk_size(); // Use configurable chunk size
        let total_messages = messages.len();

        if total_messages > max_chunk_size {
            log::info!(
                "Splitting {} messages into chunks of {} for queue: {}",
                total_messages,
                max_chunk_size,
                queue_name
            );

            // Send messages in chunks
            for (chunk_idx, chunk) in messages.chunks(max_chunk_size).enumerate() {
                log::debug!(
                    "Sending chunk {}/{} ({} messages) to queue: {}",
                    chunk_idx + 1,
                    total_messages.div_ceil(max_chunk_size),
                    chunk.len(),
                    queue_name
                );

                producer.send_messages(chunk.to_vec()).await.map_err(|e| {
                    format!(
                        "Failed to send chunk {} to queue {}: {}",
                        chunk_idx + 1,
                        queue_name,
                        e
                    )
                })?;
            }
        } else {
            log::debug!(
                "Sending batch of {} messages to queue: {}",
                messages.len(),
                queue_name
            );

            // Send all messages at once for smaller batches
            producer
                .send_messages(messages)
                .await
                .map_err(|e| format!("Failed to send messages to queue {}: {}", queue_name, e))?;
        }

        log::debug!("Disposing producer for queue: {}", queue_name);
        producer
            .dispose()
            .await
            .map_err(|e| format!("Failed to dispose producer for queue {}: {}", queue_name, e))?;

        log::info!(
            "Successfully sent {} messages to queue: {}",
            total_messages,
            queue_name
        );
        Ok(())
    }

    /// Update complete_processed_messages to work with BulkOperationContext
    async fn complete_processed_messages(
        &self,
        messages: &[azservicebus::ServiceBusReceivedMessage],
        context: &BulkOperationContext,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        log::debug!("Completing {} messages in source queue", messages.len());
        let mut consumer_guard = context.consumer.lock().await;
        consumer_guard.complete_messages(messages).await.map_err(
            |e| -> Box<dyn Error + Send + Sync> {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to complete messages: {}", e),
                ))
            },
        )?;
        drop(consumer_guard);
        Ok(())
    }

    // Removed unused convert_peeked_messages_for_sending method

    /// Collect target messages from the queue, separating them from non-target messages
    async fn collect_target_messages(
        &self,
        consumer: Arc<Mutex<Consumer>>,
        target_map: &HashMap<String, MessageIdentifier>,
        batch_size: usize,
    ) -> Result<
        (
            Vec<azservicebus::ServiceBusReceivedMessage>,
            Vec<azservicebus::ServiceBusReceivedMessage>,
        ),
        Box<dyn Error + Send + Sync>,
    > {
        let mut target_messages = Vec::new();
        let mut non_target_messages = Vec::new();
        let mut messages_processed = 0;
        let mut remaining_targets = target_map.clone();

        self.log_collection_start(target_map.len(), batch_size);

        // Keep processing batches until we find all target messages or no more messages available
        while !remaining_targets.is_empty() {
            let ctx = BatchProcessingContext {
                consumer: consumer.clone(),
                batch_size,
                target_messages_found: target_messages.len(),
                target_map,
                messages_processed,
                remaining_targets: &mut remaining_targets,
                target_messages_vec: &mut target_messages,
                non_target_messages: &mut non_target_messages,
            };

            match self.process_single_batch(ctx).await? {
                Some(batch_processed) => {
                    messages_processed += batch_processed;
                }
                None => {
                    self.log_no_more_messages(
                        messages_processed,
                        target_messages.len(),
                        target_map.len(),
                    );
                    break;
                }
            }
        }

        self.log_collection_complete(
            &target_messages,
            &non_target_messages,
            messages_processed,
            &remaining_targets,
        );

        Ok((target_messages, non_target_messages))
    }

    /// Log the start of the collection phase
    fn log_collection_start(&self, target_count: usize, batch_size: usize) {
        log::debug!(
            "Starting message collection phase - searching for {} target messages using batch size {}",
            target_count,
            batch_size
        );
    }

    /// Log when no more messages are available
    fn log_no_more_messages(
        &self,
        messages_processed: usize,
        targets_found: usize,
        total_targets: usize,
    ) {
        log::warn!(
            "No more messages available in queue after processing {} messages. Found {}/{} target messages.",
            messages_processed,
            targets_found,
            total_targets
        );
    }

    /// Log the completion of the collection phase
    fn log_collection_complete(
        &self,
        target_messages: &[azservicebus::ServiceBusReceivedMessage],
        non_target_messages: &[azservicebus::ServiceBusReceivedMessage],
        messages_processed: usize,
        remaining_targets: &HashMap<String, MessageIdentifier>,
    ) {
        log::info!(
            "Collection phase complete: {} target messages found, {} non-target messages collected, {} messages processed total",
            target_messages.len(),
            non_target_messages.len(),
            messages_processed
        );

        if !remaining_targets.is_empty() {
            log::warn!(
                "Could not find {} target messages: {:?}",
                remaining_targets.len(),
                remaining_targets.keys().collect::<Vec<_>>()
            );
        }
    }

    /// Process a single batch of messages
    async fn process_single_batch(
        &self,
        ctx: BatchProcessingContext<'_>,
    ) -> Result<Option<usize>, Box<dyn Error + Send + Sync>> {
        match self
            .receive_message_batch(
                ctx.consumer,
                ctx.batch_size,
                ctx.target_messages_found,
                ctx.target_map,
                ctx.messages_processed,
            )
            .await?
        {
            Some(received_messages) => {
                let batch_processed = self.process_message_batch(
                    received_messages,
                    ctx.remaining_targets,
                    ctx.target_messages_vec,
                    ctx.non_target_messages,
                );
                Ok(Some(batch_processed))
            }
            None => Ok(None),
        }
    }

    /// Receive a batch of messages from the consumer
    async fn receive_message_batch(
        &self,
        consumer: Arc<Mutex<Consumer>>,
        batch_size: usize,
        target_messages_found: usize,
        target_map: &HashMap<String, MessageIdentifier>,
        messages_processed: usize,
    ) -> Result<Option<Vec<azservicebus::ServiceBusReceivedMessage>>, Box<dyn Error + Send + Sync>>
    {
        log::debug!(
            "Receiving batch of {} messages (found {}/{} targets so far, {} messages processed total)",
            batch_size,
            target_messages_found,
            target_map.len(),
            messages_processed
        );

        let mut consumer_guard = consumer.lock().await;
        let received_messages = consumer_guard
            .receive_messages(batch_size as u32)
            .await
            .map_err(|e| -> Box<dyn Error + Send + Sync> {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to receive messages: {}", e),
                ))
            })?;
        drop(consumer_guard); // Release the lock early

        if received_messages.is_empty() {
            Ok(None)
        } else {
            Ok(Some(received_messages))
        }
    }

    /// Process a batch of messages, categorizing them as target or non-target
    fn process_message_batch(
        &self,
        received_messages: Vec<azservicebus::ServiceBusReceivedMessage>,
        remaining_targets: &mut HashMap<String, MessageIdentifier>,
        target_messages: &mut Vec<azservicebus::ServiceBusReceivedMessage>,
        non_target_messages: &mut Vec<azservicebus::ServiceBusReceivedMessage>,
    ) -> usize {
        let mut batch_processed = 0;

        // Process each message in the batch - keep them in memory (they are locked)
        for message in received_messages {
            let message_id = message
                .message_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            if remaining_targets.contains_key(&message_id) {
                log::debug!(
                    "Found target message: {} (sequence: {})",
                    message_id,
                    message.sequence_number()
                );
                remaining_targets.remove(&message_id);
                target_messages.push(message);
            } else {
                log::debug!(
                    "Keeping non-target message in memory: {} (sequence: {})",
                    message_id,
                    message.sequence_number()
                );
                non_target_messages.push(message);
            }

            batch_processed += 1;
        }

        batch_processed
    }

    /// Abandon non-target messages to make them available in DLQ again
    async fn abandon_non_target_messages(
        &self,
        consumer: Arc<Mutex<Consumer>>,
        non_target_messages: Vec<azservicebus::ServiceBusReceivedMessage>,
        _result: &mut BulkOperationResult,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        if non_target_messages.is_empty() {
            return Ok(());
        }

        log::info!(
            "Abandoning {} non-target messages to make them available in DLQ again",
            non_target_messages.len()
        );

        let mut consumer_guard = consumer.lock().await;
        consumer_guard
            .abandon_messages(&non_target_messages)
            .await
            .map_err(|e| -> Box<dyn Error + Send + Sync> {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to abandon messages: {}", e),
                ))
            })?;
        drop(consumer_guard);

        Ok(())
    }

    // Removed unused convert_messages_for_sending method

    /// Track which specific messages were successfully processed
    fn track_successful_messages(
        &self,
        messages: &[azservicebus::ServiceBusReceivedMessage],
        target_map: &HashMap<String, MessageIdentifier>,
        result: &mut BulkOperationResult,
    ) {
        log::debug!(
            "TRACKING: Starting to track {} successful messages (current count: {})",
            messages.len(),
            result.successful
        );
        for message in messages {
            let message_id = message
                .message_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            // Find the corresponding MessageIdentifier from the original target map
            if let Some(original_message_id) = target_map.get(&message_id) {
                result.add_successful_message(original_message_id.clone());
                log::debug!(
                    "Marked message {} (sequence: {}) as successfully processed",
                    original_message_id.id,
                    original_message_id.sequence
                );
            }
        }
    }

    /// Main entry point for bulk delete operations
    pub async fn bulk_delete(
        &self,
        context: BulkOperationContext,
        params: &BulkSendParams,
    ) -> Result<BulkOperationResult, Box<dyn Error + Send + Sync>> {
        let total_requested = params.message_identifiers.len();
        let mut result = BulkOperationResult::new(total_requested);

        log::info!(
            "Starting bulk delete operation for {} messages",
            total_requested
        );

        if total_requested == 0 {
            log::warn!("No messages provided for bulk delete operation");
            return Ok(result);
        }

        // Create a lookup map for quick message identification
        let target_map: HashMap<String, MessageIdentifier> = params
            .message_identifiers
            .iter()
            .map(|m| (m.id.clone(), m.clone()))
            .collect();

        // Log target message IDs and sequences for debugging
        log::info!("Target messages for deletion:");
        for (i, msg_id) in params.message_identifiers.iter().take(10).enumerate() {
            log::info!(
                "  [{}] ID: {}, Sequence: {}",
                i + 1,
                msg_id.id,
                msg_id.sequence
            );
        }
        if params.message_identifiers.len() > 10 {
            log::info!(
                "  ... and {} more messages",
                params.message_identifiers.len() - 10
            );
        }

        // Use a very small batch size to minimize lock expiration issues
        // Azure Service Bus message locks expire after 60 seconds, so we need to process quickly
        let batch_size = std::cmp::min(10, total_requested);

        // Limit maximum messages to process to prevent runaway operations
        // Process at most 5x the number of target messages, with a minimum of 500
        // This accounts for Azure Service Bus message ordering differences
        let max_messages_to_process = (total_requested * 5).max(500);

        log::info!(
            "Processing bulk delete for {} selected messages using batch size {} (max {} messages to process)",
            total_requested,
            batch_size,
            max_messages_to_process
        );

        log::info!(
            "Target sequence number range: {} to {}",
            target_map.values().map(|m| m.sequence).min().unwrap_or(0),
            target_map.values().map(|m| m.sequence).max().unwrap_or(0)
        );

        // Process messages in smaller batches to avoid lock expiration
        let mut remaining_targets = target_map.clone();
        let mut total_processed = 0;
        let start_time = std::time::Instant::now();
        let max_processing_time_secs = self.config.bulk_processing_time_secs(); // Stay well under 60-second lock timeout to prevent expiration

        while !remaining_targets.is_empty()
            && total_processed < max_messages_to_process
            && start_time.elapsed().as_secs() < max_processing_time_secs
        {
            let elapsed_time = start_time.elapsed().as_secs();
            log::debug!(
                "Processing batch - {} targets remaining, {} messages processed so far, elapsed time: {}s",
                remaining_targets.len(),
                total_processed,
                elapsed_time
            );

            // Check if we're approaching the timeout limit before starting a new batch
            if elapsed_time > (max_processing_time_secs - 10) {
                log::warn!(
                    "Approaching timeout limit ({}s), stopping processing to avoid lock expiration. {} targets remaining.",
                    elapsed_time,
                    remaining_targets.len()
                );
                break;
            }

            // Receive a small batch of messages
            let batch_start_time = std::time::Instant::now();
            let received_messages = {
                let mut consumer_guard = context.consumer.lock().await;
                consumer_guard
                    .receive_messages(batch_size as u32)
                    .await
                    .map_err(|e| -> Box<dyn Error + Send + Sync> {
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Failed to receive messages: {}", e),
                        ))
                    })?
            };

            let receive_time = batch_start_time.elapsed().as_millis();
            log::debug!(
                "Received {} messages in {}ms",
                received_messages.len(),
                receive_time
            );

            // Log sequence number range of received messages for debugging
            if !received_messages.is_empty() {
                let min_seq = received_messages
                    .iter()
                    .map(|m| m.sequence_number())
                    .min()
                    .unwrap_or(0);
                let max_seq = received_messages
                    .iter()
                    .map(|m| m.sequence_number())
                    .max()
                    .unwrap_or(0);
                log::debug!(
                    "Received messages sequence range: {} to {} (batch size: {})",
                    min_seq,
                    max_seq,
                    received_messages.len()
                );
            }

            if received_messages.is_empty() {
                log::info!("No more messages available in queue");
                break;
            }

            // Immediately process this batch to avoid lock expiration
            let mut target_messages = Vec::new();
            let mut non_target_messages = Vec::new();

            for message in received_messages {
                let message_id = message
                    .message_id()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let message_sequence = message.sequence_number();

                // Check if this message matches our targets (ID-only matching)
                if remaining_targets.contains_key(&message_id) {
                    log::info!(
                        "✓ Found target message: {} (sequence: {})",
                        message_id,
                        message_sequence
                    );
                    remaining_targets.remove(&message_id);
                    target_messages.push(message);
                } else {
                    log::debug!(
                        "Non-target message: {} (sequence: {})",
                        message_id,
                        message_sequence
                    );
                    non_target_messages.push(message);
                }
                total_processed += 1;
            }

            // Immediately complete target messages while locks are still valid
            if !target_messages.is_empty() {
                let complete_start_time = std::time::Instant::now();
                log::debug!(
                    "Immediately completing {} target messages",
                    target_messages.len()
                );

                match self
                    .complete_processed_messages(&target_messages, &context)
                    .await
                {
                    Ok(()) => {
                        let complete_time = complete_start_time.elapsed().as_millis();
                        self.track_successful_messages(&target_messages, &target_map, &mut result);
                        log::debug!(
                            "Successfully deleted {} messages in this batch (took {}ms)",
                            target_messages.len(),
                            complete_time
                        );
                    }
                    Err(e) => {
                        let complete_time = complete_start_time.elapsed().as_millis();
                        let error_msg = format!(
                            "Failed to delete target messages in batch (after {}ms): {}",
                            complete_time, e
                        );
                        log::error!("{}", error_msg);

                        // Check if this looks like a lock expiration error
                        let error_str = e.to_string().to_lowercase();
                        if error_str.contains("lock")
                            || error_str.contains("timeout")
                            || error_str.contains("expired")
                        {
                            log::error!(
                                "CRITICAL: Lock expiration detected! {} target messages may not be deleted",
                                target_messages.len()
                            );
                        }

                        result.add_failure(error_msg);

                        // Don't track these as successful since they failed
                        for msg in &target_messages {
                            let msg_id = msg
                                .message_id()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "unknown".to_string());
                            remaining_targets.insert(
                                msg_id.clone(),
                                target_map
                                    .get(&msg_id)
                                    .cloned()
                                    .unwrap_or_else(|| MessageIdentifier::from_string(msg_id)),
                            );
                        }
                    }
                }
            }

            // Abandon non-target messages immediately with detailed error handling
            if !non_target_messages.is_empty() {
                let abandon_start_time = std::time::Instant::now();
                log::debug!(
                    "Abandoning {} non-target messages",
                    non_target_messages.len()
                );
                let mut consumer_guard = context.consumer.lock().await;
                match consumer_guard.abandon_messages(&non_target_messages).await {
                    Ok(()) => {
                        let abandon_time = abandon_start_time.elapsed().as_millis();
                        log::debug!(
                            "Successfully abandoned {} non-target messages (took {}ms)",
                            non_target_messages.len(),
                            abandon_time
                        );
                    }
                    Err(e) => {
                        let abandon_time = abandon_start_time.elapsed().as_millis();
                        log::error!(
                            "CRITICAL: Failed to abandon {} non-target messages after {}ms: {}. These messages may be accidentally completed!",
                            non_target_messages.len(),
                            abandon_time,
                            e
                        );

                        // Check if this looks like a lock expiration error
                        let error_str = e.to_string().to_lowercase();
                        if error_str.contains("lock")
                            || error_str.contains("timeout")
                            || error_str.contains("expired")
                        {
                            log::error!(
                                "CRITICAL: Lock expiration detected during abandon! {} non-target messages may be accidentally deleted!",
                                non_target_messages.len()
                            );
                        }

                        // This is a critical error - non-target messages might get completed instead of abandoned
                        result.add_failure(format!(
                            "Failed to abandon {} non-target messages: {}",
                            non_target_messages.len(),
                            e
                        ));
                    }
                }
            }
        }

        // Calculate not found messages correctly - use remaining targets count
        result.not_found = remaining_targets.len();

        // Debug logging to track the calculation
        log::debug!(
            "Final calculation: successful={}, failed={}, remaining_targets={}, total_requested={}",
            result.successful,
            result.failed,
            remaining_targets.len(),
            total_requested
        );

        let processing_time = start_time.elapsed().as_secs();

        if total_processed >= max_messages_to_process && !remaining_targets.is_empty() {
            log::warn!(
                "Reached maximum message processing limit ({}). {} targets not found after processing {} messages in {} seconds.",
                max_messages_to_process,
                remaining_targets.len(),
                total_processed,
                processing_time
            );
        } else if processing_time >= max_processing_time_secs && !remaining_targets.is_empty() {
            log::warn!(
                "Reached maximum processing time ({}s). {} targets not found after processing {} messages.",
                max_processing_time_secs,
                remaining_targets.len(),
                total_processed
            );
        }

        log::info!(
            "Bulk delete operation completed: {} successful, {} failed, {} not found out of {} requested (processed {} messages total)",
            result.successful,
            result.failed,
            result.not_found,
            total_requested,
            total_processed
        );

        Ok(result)
    }
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
        }
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
