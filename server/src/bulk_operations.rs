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
}

impl BulkOperationResult {
    pub fn new(total_requested: usize) -> Self {
        Self {
            total_requested,
            successful: 0,
            failed: 0,
            not_found: 0,
            error_details: Vec::new(),
        }
    }

    pub fn add_success(&mut self) {
        self.successful += 1;
    }

    pub fn add_failure(&mut self, error: String) {
        self.failed += 1;
        self.error_details.push(error);
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
    pub max_batch_size: u32,
    pub operation_timeout_secs: u64,
    pub order_warning_threshold: u32,
}

impl Default for BulkOperationConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 2048,
            operation_timeout_secs: 300,
            order_warning_threshold: 2048,
        }
    }
}

impl BulkOperationConfig {
    /// Create a new BulkOperationConfig with specified values
    pub fn new(
        max_batch_size: u32,
        operation_timeout_secs: u64,
        order_warning_threshold: u32,
    ) -> Self {
        Self {
            max_batch_size,
            operation_timeout_secs,
            order_warning_threshold,
        }
    }
}

/// Handles bulk operations on Azure Service Bus queues
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
        consumer: Arc<Mutex<Consumer>>,
        target_messages: Vec<MessageIdentifier>,
        main_queue_name: String,
        service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
    ) -> Result<BulkOperationResult, Box<dyn std::error::Error>> {
        log::info!(
            "Starting bulk resend operation for {} messages to queue {}",
            target_messages.len(),
            main_queue_name
        );

        let mut result = BulkOperationResult::new(target_messages.len());
        if target_messages.is_empty() {
            log::warn!("No messages provided for bulk resend operation");
            return Ok(result);
        }
        let batch_size = std::cmp::min(
            target_messages.len() * 2,
            self.config.max_batch_size as usize,
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
        let operation_timeout = Duration::from_secs(self.config.operation_timeout_secs);
        let bulk_result = timeout(
            operation_timeout,
            self.execute_bulk_resend_operation(
                consumer,
                target_map,
                batch_size,
                main_queue_name,
                service_bus_client,
            ),
        )
        .await;

        match bulk_result {
            Ok(operation_result) => operation_result,
            Err(_) => {
                let timeout_error = format!(
                    "Bulk resend operation timed out after {} seconds",
                    self.config.operation_timeout_secs
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
        consumer: Arc<Mutex<Consumer>>,
        target_map: HashMap<String, MessageIdentifier>,
        batch_size: usize,
        main_queue_name: String,
        service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
    ) -> Result<BulkOperationResult, Box<dyn std::error::Error>> {
        let mut result = BulkOperationResult::new(target_map.len());

        // Phase 1: Collect target and non-target messages
        let (target_messages, non_target_messages) = self
            .collect_target_messages(consumer.clone(), &target_map, batch_size)
            .await?;

        // Phase 2: Process target messages (resend them)
        if !target_messages.is_empty() {
            match self
                .process_target_messages(
                    target_messages,
                    &main_queue_name,
                    service_bus_client,
                    consumer.clone(),
                )
                .await
            {
                Ok(processed_count) => {
                    result.successful = processed_count;
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
        self.abandon_non_target_messages(consumer, non_target_messages, &mut result)
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

        log::debug!(
            "Starting message collection phase - searching for {} target messages using batch size {}",
            target_map.len(),
            batch_size
        );

        // Keep processing batches until we find all target messages or no more messages available
        while !remaining_targets.is_empty() {
            match self
                .receive_message_batch(
                    consumer.clone(),
                    batch_size,
                    &target_messages,
                    target_map,
                    messages_processed,
                )
                .await?
            {
                Some(received_messages) => {
                    let batch_processed = self.process_message_batch(
                        received_messages,
                        &mut remaining_targets,
                        &mut target_messages,
                        &mut non_target_messages,
                    );
                    messages_processed += batch_processed;
                }
                None => {
                    // No more messages available
                    log::warn!(
                        "No more messages available in queue after processing {} messages. Found {}/{} target messages.",
                        messages_processed,
                        target_messages.len(),
                        target_map.len()
                    );
                    break;
                }
            }
        }

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

        Ok((target_messages, non_target_messages))
    }

    /// Receive a batch of messages from the consumer
    async fn receive_message_batch(
        &self,
        consumer: Arc<Mutex<Consumer>>,
        batch_size: usize,
        target_messages: &[azservicebus::ServiceBusReceivedMessage],
        target_map: &HashMap<String, MessageIdentifier>,
        messages_processed: usize,
    ) -> Result<Option<Vec<azservicebus::ServiceBusReceivedMessage>>, Box<dyn std::error::Error>>
    {
        log::debug!(
            "Receiving batch of {} messages (found {}/{} targets so far, {} messages processed total)",
            batch_size,
            target_messages.len(),
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
        main_queue_name: &str,
        service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
        consumer: Arc<Mutex<Consumer>>,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        if messages.is_empty() {
            return Ok(0);
        }

        log::debug!("Processing {} target messages", messages.len());

        // Create new messages for the main queue
        let mut new_messages = Vec::new();
        for message in &messages {
            let body = message.body()?;
            let new_message = ServiceBusMessage::new(body.to_vec());
            new_messages.push(new_message);
        }

        // Send messages to main queue
        log::debug!(
            "Sending {} messages to main queue {}",
            new_messages.len(),
            main_queue_name
        );
        self.send_messages_to_main_queue(main_queue_name, new_messages, service_bus_client)
            .await?;

        // Complete messages in DLQ (remove them)
        log::debug!("Completing {} messages in DLQ", messages.len());
        let mut consumer_guard = consumer.lock().await;
        consumer_guard.complete_messages(&messages).await?;
        drop(consumer_guard);

        log::info!("Successfully processed {} messages", messages.len());
        Ok(messages.len())
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
        max_message_index > self.config.order_warning_threshold as usize
    }
}
