use super::errors::{ServiceBusError, ServiceBusResult};
use super::types::QueueInfo;
use crate::bulk_operations::types::BatchConfig;
use crate::consumer::{Consumer, ServiceBusClientExt};
use crate::model::MessageModel;
use azservicebus::{ServiceBusClient, ServiceBusReceiverOptions, core::BasicRetryPolicy};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

pub struct ConsumerManager {
    current_consumer: Option<Arc<Mutex<Consumer>>>,
    current_queue: Option<QueueInfo>,
    service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
    batch_config: BatchConfig,
}

impl ConsumerManager {
    pub fn new(
        service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
        batch_config: BatchConfig,
    ) -> Self {
        Self {
            current_consumer: None,
            current_queue: None,
            service_bus_client,
            batch_config,
        }
    }

    /// Switch to a different queue, disposing current consumer if needed
    pub async fn switch_queue(&mut self, queue_info: QueueInfo) -> ServiceBusResult<()> {
        log::info!(
            "Switching to queue: {} (type: {:?})",
            queue_info.name,
            queue_info.queue_type
        );

        // Check if we're already connected to this queue
        if let Some(current_queue) = &self.current_queue {
            if current_queue.name == queue_info.name
                && current_queue.queue_type == queue_info.queue_type
            {
                log::debug!("Already connected to queue: {}", queue_info.name);
                return Ok(());
            }
        }

        // Dispose current consumer if exists
        if let Some(consumer) = &self.current_consumer {
            log::debug!("Disposing existing consumer");
            if let Err(e) = consumer.lock().await.dispose().await {
                log::error!("Failed to dispose existing consumer: {e}");
                // Continue anyway - we'll create a new one
            }
        }

        // Create new consumer
        log::debug!("Creating new consumer for queue: {}", queue_info.name);
        let mut client = self.service_bus_client.lock().await;
        let consumer = client
            .create_consumer_for_queue(
                queue_info.name.clone(),
                ServiceBusReceiverOptions::default(),
            )
            .await
            .map_err(|e| {
                ServiceBusError::ConsumerCreationFailed(format!(
                    "Failed to create consumer for queue {}: {}",
                    queue_info.name, e
                ))
            })?;

        // Update state
        self.current_consumer = Some(Arc::new(Mutex::new(consumer)));
        self.current_queue = Some(queue_info);

        if let Some(queue) = self.current_queue.as_ref() {
            log::info!("Successfully switched to queue: {}", queue.name);
        }
        Ok(())
    }

    /// Get current queue information
    pub fn current_queue(&self) -> Option<&QueueInfo> {
        self.current_queue.as_ref()
    }

    /// Check if consumer is available and ready
    pub fn is_consumer_ready(&self) -> bool {
        self.current_consumer.is_some() && self.current_queue.is_some()
    }

    /// Peek messages from the current queue
    pub async fn peek_messages(
        &self,
        max_count: u32,
        from_sequence: Option<i64>,
    ) -> ServiceBusResult<Vec<MessageModel>> {
        let consumer = self.get_consumer()?;
        let mut consumer_guard = consumer.lock().await;

        consumer_guard
            .peek_messages(max_count, from_sequence)
            .await
            .map_err(|e| ServiceBusError::MessageReceiveFailed(e.to_string()))
    }

    /// Receive messages from the current queue (for processing)
    pub async fn receive_messages(
        &self,
        max_count: u32,
    ) -> ServiceBusResult<Vec<azservicebus::ServiceBusReceivedMessage>> {
        let consumer = self.get_consumer()?;
        let mut consumer_guard = consumer.lock().await;

        // Use timeout-based receive for consistency and to prevent indefinite blocking
        let timeout = Duration::from_secs(self.batch_config.receive_timeout_secs());
        consumer_guard
            .receive_messages_with_timeout(max_count, timeout)
            .await
            .map_err(|e| ServiceBusError::MessageReceiveFailed(e.to_string()))
    }

    /// Complete a single message
    pub async fn complete_message(
        &self,
        message: &azservicebus::ServiceBusReceivedMessage,
    ) -> ServiceBusResult<()> {
        let consumer = self.get_consumer()?;
        let mut consumer_guard = consumer.lock().await;

        consumer_guard
            .complete_message(message)
            .await
            .map_err(|e| ServiceBusError::MessageCompleteFailed(e.to_string()))
    }

    /// Complete multiple messages
    pub async fn complete_messages(
        &self,
        messages: &[azservicebus::ServiceBusReceivedMessage],
    ) -> ServiceBusResult<()> {
        let consumer = self.get_consumer()?;
        let mut consumer_guard = consumer.lock().await;

        consumer_guard
            .complete_messages(messages)
            .await
            .map_err(|e| ServiceBusError::MessageCompleteFailed(e.to_string()))
    }

    /// Abandon a single message
    pub async fn abandon_message(
        &self,
        message: &azservicebus::ServiceBusReceivedMessage,
    ) -> ServiceBusResult<()> {
        let consumer = self.get_consumer()?;
        let mut consumer_guard = consumer.lock().await;

        consumer_guard
            .abandon_message(message)
            .await
            .map_err(|e| ServiceBusError::MessageAbandonFailed(e.to_string()))
    }

    /// Abandon multiple messages
    pub async fn abandon_messages(
        &self,
        messages: &[azservicebus::ServiceBusReceivedMessage],
    ) -> ServiceBusResult<()> {
        let consumer = self.get_consumer()?;
        let mut consumer_guard = consumer.lock().await;

        consumer_guard
            .abandon_messages(messages)
            .await
            .map_err(|e| ServiceBusError::MessageAbandonFailed(e.to_string()))
    }

    /// Dead letter a single message
    pub async fn dead_letter_message(
        &self,
        message: &azservicebus::ServiceBusReceivedMessage,
        reason: Option<String>,
        error_description: Option<String>,
    ) -> ServiceBusResult<()> {
        let consumer = self.get_consumer()?;
        let mut consumer_guard = consumer.lock().await;

        consumer_guard
            .dead_letter_message(message, reason, error_description)
            .await
            .map_err(|e| ServiceBusError::MessageDeadLetterFailed(e.to_string()))
    }

    /// Find a specific message by ID and sequence number (used for targeted operations)
    pub async fn find_message(
        &self,
        message_id: &str,
        sequence_number: i64,
        max_position: Option<usize>,
    ) -> ServiceBusResult<Option<azservicebus::ServiceBusReceivedMessage>> {
        let consumer = self.get_consumer()?;
        let batch_size = self.batch_config.bulk_chunk_size() as u32;
        let timeout = Duration::from_secs(self.batch_config.bulk_processing_time_secs());
        let max_position = max_position.unwrap_or(self.batch_config.max_messages_to_process());

        log::info!(
            "Searching for message {message_id} (sequence: {sequence_number}) in batches of {batch_size} up to position {max_position}"
        );

        let mut processed_count = 0;

        while processed_count < max_position {
            log::debug!(
                "Search batch: fetching {batch_size} messages (processed: {processed_count}/{max_position})"
            );

            let messages = {
                let mut consumer_guard = consumer.lock().await;
                consumer_guard
                    .receive_messages_with_timeout(batch_size, timeout)
                    .await
                    .map_err(|e| ServiceBusError::MessageReceiveFailed(e.to_string()))?
            };

            if messages.is_empty() {
                log::debug!("No more messages available - stopping search");
                break;
            }

            processed_count += messages.len();

            // Look for the target message in this batch
            for message in messages.into_iter() {
                let msg_id = message.message_id().unwrap_or_default();
                let msg_seq = message.sequence_number();

                if msg_id == message_id && msg_seq == sequence_number {
                    log::info!(
                        "Found target message {message_id} (sequence: {sequence_number}) after processing {processed_count} messages"
                    );
                    return Ok(Some(message));
                }

                // Abandon non-target messages to keep the queue flowing
                let mut consumer_guard = consumer.lock().await;
                if let Err(e) = consumer_guard.abandon_message(&message).await {
                    log::warn!("Failed to abandon non-target message: {e}");
                }
            }
        }

        log::info!(
            "Message {message_id} (sequence: {sequence_number}) not found after processing {processed_count} messages (max: {max_position})"
        );
        Ok(None)
    }

    /// Dispose current consumer
    pub async fn dispose_consumer(&mut self) -> ServiceBusResult<()> {
        if let Some(consumer) = self.current_consumer.take() {
            log::info!("Disposing consumer for queue: {:?}", self.current_queue);
            consumer.lock().await.dispose().await.map_err(|e| {
                ServiceBusError::InternalError(format!("Failed to dispose consumer: {e}"))
            })?;
        }
        self.current_queue = None;
        Ok(())
    }

    /// Get the current consumer, returning an error if not available
    fn get_consumer(&self) -> ServiceBusResult<Arc<Mutex<Consumer>>> {
        self.current_consumer
            .clone()
            .ok_or(ServiceBusError::ConsumerNotFound)
    }

    /// Get raw consumer for advanced operations (used by bulk operations)
    pub fn get_raw_consumer(&self) -> Option<Arc<Mutex<Consumer>>> {
        self.current_consumer.clone()
    }

    /// Reset the ServiceBusClient reference after connection reset
    pub async fn reset_client(
        &mut self,
        new_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
    ) -> ServiceBusResult<()> {
        log::info!("Resetting ServiceBusClient reference in ConsumerManager");

        // Dispose existing consumer if any
        self.dispose_consumer().await?;

        // Update the client reference
        self.service_bus_client = new_client;

        log::info!("ConsumerManager client reference updated successfully");
        Ok(())
    }
}
