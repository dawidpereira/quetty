use crate::consumer::Consumer;
use crate::producer::ServiceBusClientProducerExt;
use azservicebus::core::BasicRetryPolicy;
use azservicebus::{ServiceBusClient, ServiceBusMessage, ServiceBusSenderOptions};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;

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
        self.successful_message_ids.push(message_id);
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
}

/// Configuration for bulk operations
#[derive(Debug, Clone)]
pub struct BulkOperationConfig {
    pub max_batch_size: Option<u32>,
    pub operation_timeout_secs: Option<u64>,
    pub order_warning_threshold: Option<u32>,
    pub batch_size_multiplier: Option<usize>,
}

impl BulkOperationConfig {
    /// Create a new BulkOperationConfig with specified values
    pub fn new(
        max_batch_size: u32,
        operation_timeout_secs: u64,
        order_warning_threshold: u32,
        batch_size_multiplier: usize,
    ) -> Self {
        Self {
            max_batch_size: Some(max_batch_size),
            operation_timeout_secs: Some(operation_timeout_secs),
            order_warning_threshold: Some(order_warning_threshold),
            batch_size_multiplier: Some(batch_size_multiplier),
        }
    }

    /// Get the maximum batch size for bulk operations (default: 2048)
    pub fn max_batch_size(&self) -> u32 {
        self.max_batch_size.unwrap_or(2048)
    }

    /// Get the timeout for bulk operations (default: 300 seconds)
    pub fn operation_timeout_secs(&self) -> u64 {
        self.operation_timeout_secs.unwrap_or(300)
    }

    /// Get the warning threshold for message order preservation (default: 2048)
    pub fn order_warning_threshold(&self) -> u32 {
        self.order_warning_threshold.unwrap_or(2048)
    }

    /// Get the batch size multiplier for target estimation (default: 2)
    pub fn batch_size_multiplier(&self) -> usize {
        self.batch_size_multiplier.unwrap_or(2)
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
    config: BulkOperationConfig,
}

impl BulkOperationHandler {
    pub fn new(config: BulkOperationConfig) -> Self {
        Self { config }
    }

    /// Resend multiple messages from DLQ to main queue efficiently
    /// This is the main entry point for bulk resend operations
    pub async fn bulk_resend_from_dlq(
        &self,
        context: ServiceBusOperationContext,
        target_messages: Vec<MessageIdentifier>,
    ) -> Result<BulkOperationResult, Box<dyn std::error::Error>> {
        log::info!(
            "Starting bulk resend operation for {} messages to queue {}",
            target_messages.len(),
            context.main_queue_name
        );

        let mut result = BulkOperationResult::new(target_messages.len());
        if target_messages.is_empty() {
            log::warn!("No messages provided for bulk resend operation");
            return Ok(result);
        }
        // Use a multiplier for batch size to efficiently retrieve messages
        // We use configurable multiplier to reduce round trips while staying within limits
        let batch_size = std::cmp::min(
            target_messages.len() * self.config.batch_size_multiplier(),
            self.config.max_batch_size() as usize,
        );

        log::info!(
            "Processing bulk resend for {} selected messages using batch size {}",
            target_messages.len(),
            batch_size
        );

        // Create a lookup map for quick message identification
        let target_map: HashMap<String, MessageIdentifier> = target_messages
            .iter()
            .map(|m| (m.id.clone(), m.clone()))
            .collect();

        // Execute the bulk resend operation with timeout
        let operation_timeout = Duration::from_secs(self.config.operation_timeout_secs());
        let bulk_result = timeout(
            operation_timeout,
            self.execute_bulk_resend_operation(context, target_map, batch_size),
        )
        .await;

        match bulk_result {
            Ok(operation_result) => operation_result,
            Err(_) => {
                let timeout_error = format!(
                    "Bulk resend operation timed out after {} seconds",
                    self.config.operation_timeout_secs()
                );
                log::error!("{}", timeout_error);
                result.add_failure(timeout_error);
                Ok(result)
            }
        }
    }

    /// Core implementation of bulk resend operation
    async fn execute_bulk_resend_operation(
        &self,
        context: ServiceBusOperationContext,
        target_map: HashMap<String, MessageIdentifier>,
        batch_size: usize,
    ) -> Result<BulkOperationResult, Box<dyn std::error::Error>> {
        let mut result = BulkOperationResult::new(target_map.len());

        // Phase 1: Collect target and non-target messages
        let (target_messages, non_target_messages) = self
            .collect_target_messages(context.consumer.clone(), &target_map, batch_size)
            .await?;

        // Phase 2: Process target messages (resend them)
        if !target_messages.is_empty() {
            match self
                .process_target_messages(target_messages, &context, &target_map, &mut result)
                .await
            {
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

        // Phase 3: Abandon non-target messages to make them available in DLQ again
        self.abandon_non_target_messages(context.consumer, non_target_messages, &mut result)
            .await?;

        // Calculate not found messages
        result.not_found = target_map.len() - result.successful;

        log::info!(
            "Bulk resend operation completed: {} successful, {} failed, {} not found",
            result.successful,
            result.failed,
            result.not_found
        );

        Ok(result)
    }

    /// Resend messages from peeked message data to main queue without deleting from DLQ
    /// This method uses message data already available from peek operations
    pub async fn bulk_resend_from_dlq_only(
        &self,
        context: ServiceBusOperationContext,
        messages_data: Vec<(MessageIdentifier, Vec<u8>)>,
    ) -> Result<BulkOperationResult, Box<dyn std::error::Error>> {
        log::info!(
            "Starting bulk resend-only operation for {} messages to queue {} (without deleting from DLQ)",
            messages_data.len(),
            context.main_queue_name
        );

        let mut result = BulkOperationResult::new(messages_data.len());
        if messages_data.is_empty() {
            log::warn!("No message data provided for bulk resend-only operation");
            return Ok(result);
        }

        // Execute the bulk resend-only operation with timeout
        let operation_timeout = Duration::from_secs(self.config.operation_timeout_secs());
        let bulk_result = timeout(
            operation_timeout,
            self.execute_bulk_resend_only_operation(context, messages_data, &mut result),
        )
        .await;

        match bulk_result {
            Ok(operation_result) => operation_result,
            Err(_) => {
                let timeout_error = format!(
                    "Bulk resend-only operation timed out after {} seconds",
                    self.config.operation_timeout_secs()
                );
                log::error!("{}", timeout_error);
                result.add_failure(timeout_error);
                Ok(result)
            }
        }
    }

    /// Core implementation of bulk resend-only operation
    async fn execute_bulk_resend_only_operation(
        &self,
        context: ServiceBusOperationContext,
        messages_data: Vec<(MessageIdentifier, Vec<u8>)>,
        result: &mut BulkOperationResult,
    ) -> Result<BulkOperationResult, Box<dyn std::error::Error>> {
        if messages_data.is_empty() {
            return Ok(result.clone());
        }

        // Convert message data to ServiceBusMessage objects
        let messages_to_send = self.convert_peeked_messages_for_sending(&messages_data)?;

        // Send all messages to the main queue
        match self
            .send_messages_to_main_queue(
                &context.main_queue_name,
                messages_to_send,
                context.service_bus_client,
            )
            .await
        {
            Ok(()) => {
                // Track all messages as successful
                for (identifier, _) in messages_data {
                    result.add_successful_message(identifier);
                }
                log::info!(
                    "Successfully sent {} messages to main queue",
                    result.successful
                );
            }
            Err(e) => {
                let error_msg = format!("Failed to send messages to main queue: {}", e);
                log::error!("{}", error_msg);
                result.add_failure(error_msg);
            }
        }

        log::info!(
            "Bulk resend-only operation completed: {} successful, {} failed",
            result.successful,
            result.failed
        );

        Ok(result.clone())
    }

    /// Convert peeked message data to ServiceBusMessage objects for sending
    fn convert_peeked_messages_for_sending(
        &self,
        messages_data: &[(MessageIdentifier, Vec<u8>)],
    ) -> Result<Vec<ServiceBusMessage>, Box<dyn std::error::Error>> {
        let mut converted_messages = Vec::new();

        for (identifier, body) in messages_data {
            log::debug!("Converting peeked message {} for sending", identifier.id);

            // Create a new ServiceBusMessage with the body data
            let mut message = ServiceBusMessage::new(body.clone());

            // Set message ID for tracking (optional, but useful for debugging)
            if let Err(e) = message.set_message_id(&identifier.id) {
                log::warn!(
                    "Failed to set message ID for message {}: {}",
                    identifier.id,
                    e
                );
                // Continue anyway - this is not critical
            }

            converted_messages.push(message);
        }

        log::debug!(
            "Converted {} peeked messages for sending",
            converted_messages.len()
        );
        Ok(converted_messages)
    }

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
        Box<dyn std::error::Error>,
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
    ) -> Result<Option<usize>, Box<dyn std::error::Error>> {
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
    ) -> Result<Option<Vec<azservicebus::ServiceBusReceivedMessage>>, Box<dyn std::error::Error>>
    {
        log::debug!(
            "Receiving batch of {} messages (found {}/{} targets so far, {} messages processed total)",
            batch_size,
            target_messages_found,
            target_map.len(),
            messages_processed
        );

        let mut consumer_guard = consumer.lock().await;
        let received_messages = consumer_guard.receive_messages(batch_size as u32).await?;
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
        result: &mut BulkOperationResult,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if non_target_messages.is_empty() {
            return Ok(());
        }

        log::info!(
            "Abandoning {} non-target messages to make them available in DLQ again",
            non_target_messages.len()
        );

        let mut consumer_guard = consumer.lock().await;
        match consumer_guard.abandon_messages(&non_target_messages).await {
            Ok(()) => {
                log::info!("Successfully abandoned all non-target messages");
            }
            Err(e) => {
                let error_msg = format!("Failed to abandon non-target messages: {}", e);
                log::error!("{}", error_msg);
                result.add_failure(error_msg);
            }
        }
        drop(consumer_guard);

        Ok(())
    }

    /// Process target messages: send to main queue and complete from DLQ
    async fn process_target_messages(
        &self,
        messages: Vec<azservicebus::ServiceBusReceivedMessage>,
        context: &ServiceBusOperationContext,
        target_map: &HashMap<String, MessageIdentifier>,
        result: &mut BulkOperationResult,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        if messages.is_empty() {
            return Ok(0);
        }

        log::debug!("Processing {} target messages", messages.len());

        // Convert DLQ messages to new messages for the main queue
        let new_messages = self.convert_messages_for_sending(&messages)?;

        // Send messages to main queue
        self.send_converted_messages_to_queue(&new_messages, context)
            .await?;

        // Complete messages in DLQ (remove them)
        self.complete_processed_messages(&messages, context).await?;

        // Track successful message processing
        self.track_successful_messages(&messages, target_map, result);

        log::info!("Successfully processed {} messages", messages.len());
        Ok(messages.len())
    }

    /// Convert DLQ messages to new ServiceBusMessage objects for sending
    fn convert_messages_for_sending(
        &self,
        messages: &[azservicebus::ServiceBusReceivedMessage],
    ) -> Result<Vec<ServiceBusMessage>, Box<dyn std::error::Error>> {
        let mut new_messages = Vec::new();
        for message in messages {
            let body = message.body()?;
            let new_message = ServiceBusMessage::new(body.to_vec());
            new_messages.push(new_message);
        }
        Ok(new_messages)
    }

    /// Send converted messages to the main queue
    async fn send_converted_messages_to_queue(
        &self,
        new_messages: &[ServiceBusMessage],
        context: &ServiceBusOperationContext,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!(
            "Sending {} messages to main queue {}",
            new_messages.len(),
            context.main_queue_name
        );
        self.send_messages_to_main_queue(
            &context.main_queue_name,
            new_messages.to_vec(),
            context.service_bus_client.clone(),
        )
        .await
    }

    /// Complete processed messages in DLQ to remove them
    async fn complete_processed_messages(
        &self,
        messages: &[azservicebus::ServiceBusReceivedMessage],
        context: &ServiceBusOperationContext,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("Completing {} messages in DLQ", messages.len());
        let mut consumer_guard = context.consumer.lock().await;
        consumer_guard.complete_messages(messages).await?;
        drop(consumer_guard);
        Ok(())
    }

    /// Track which specific messages were successfully processed
    fn track_successful_messages(
        &self,
        messages: &[azservicebus::ServiceBusReceivedMessage],
        target_map: &HashMap<String, MessageIdentifier>,
        result: &mut BulkOperationResult,
    ) {
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

    /// Send multiple messages to the main queue using batch operations
    async fn send_messages_to_main_queue(
        &self,
        queue_name: &str,
        messages: Vec<ServiceBusMessage>,
        service_bus_client: Arc<Mutex<ServiceBusClient<azservicebus::core::BasicRetryPolicy>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if messages.is_empty() {
            return Ok(());
        }

        log::debug!("Creating producer for queue: {}", queue_name);

        let mut client = service_bus_client.lock().await;
        let mut producer = client
            .create_producer_for_queue(queue_name, ServiceBusSenderOptions::default())
            .await
            .map_err(|e| format!("Failed to create producer for queue {}: {}", queue_name, e))?;

        log::debug!(
            "Sending batch of {} messages to queue: {}",
            messages.len(),
            queue_name
        );

        // Send messages in batch for better performance
        producer
            .send_messages(messages)
            .await
            .map_err(|e| format!("Failed to send messages to queue {}: {}", queue_name, e))?;

        log::debug!("Disposing producer for queue: {}", queue_name);
        producer
            .dispose()
            .await
            .map_err(|e| format!("Failed to dispose producer for queue {}: {}", queue_name, e))?;

        log::info!("Successfully sent messages to queue: {}", queue_name);
        Ok(())
    }

    /// Check if the bulk operation should show a warning about message order
    pub fn should_warn_about_order(&self, max_message_index: usize) -> bool {
        max_message_index > self.config.order_warning_threshold() as usize
    }
}
