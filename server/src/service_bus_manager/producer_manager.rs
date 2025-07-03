use super::errors::{ServiceBusError, ServiceBusResult};
use super::types::{MessageData, OperationStats};
use crate::producer::{Producer, ServiceBusClientProducerExt};
use azservicebus::{
    ServiceBusClient, ServiceBusMessage, ServiceBusSenderOptions, core::BasicRetryPolicy,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ProducerManager {
    producers: HashMap<String, Arc<Mutex<Producer>>>,
    service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
    batch_config: crate::bulk_operations::types::BatchConfig,
}

impl ProducerManager {
    pub fn new(
        service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
        batch_config: crate::bulk_operations::types::BatchConfig,
    ) -> Self {
        Self {
            producers: HashMap::new(),
            service_bus_client,
            batch_config,
        }
    }

    /// Send a single message to a queue using the service bus
    pub async fn send_message(
        &mut self,
        queue_name: &str,
        message: MessageData,
    ) -> ServiceBusResult<()> {
        log::info!(
            "Sending message to queue '{}' (content: {} bytes)",
            queue_name,
            message.content.len()
        );

        // Get or create producer for the queue
        let producer = self.get_or_create_producer(queue_name).await?;

        // Convert MessageData to ServiceBusMessage
        let service_bus_message = self.create_service_bus_message(&message)?;

        // Send the message
        producer
            .lock()
            .await
            .send_message(service_bus_message)
            .await
            .map_err(|e| {
                ServiceBusError::MessageSendFailed(format!(
                    "Failed to send message to queue {}: {}",
                    queue_name, e
                ))
            })?;

        log::info!("Successfully sent message to queue: {}", queue_name);
        Ok(())
    }

    /// Send multiple messages to a queue
    pub async fn send_messages(
        &mut self,
        queue_name: &str,
        messages: Vec<MessageData>,
    ) -> ServiceBusResult<OperationStats> {
        log::info!(
            "Sending {} messages to queue: {}",
            messages.len(),
            queue_name
        );

        let mut stats = OperationStats::new();
        stats.total = messages.len();

        if messages.is_empty() {
            return Ok(stats);
        }

        // Get or create producer for the queue
        let producer = self.get_or_create_producer(queue_name).await?;

        // Convert MessageData to ServiceBusMessage objects
        let mut service_bus_messages = Vec::new();
        for message in &messages {
            match self.create_service_bus_message(message) {
                Ok(sb_message) => service_bus_messages.push(sb_message),
                Err(e) => {
                    log::error!("Failed to create ServiceBusMessage: {}", e);
                    stats.failed += 1;
                }
            }
        }

        // Send messages in batches
        let batch_size = self.batch_config.bulk_chunk_size();
        for (batch_index, batch) in service_bus_messages.chunks(batch_size).enumerate() {
            match producer.lock().await.send_messages(batch.to_vec()).await {
                Ok(()) => {
                    stats.successful += batch.len();
                    log::debug!("Successfully sent batch of {} messages", batch.len());
                }
                Err(e) => {
                    stats.failed += batch.len();
                    log::error!("Failed to send batch of {} messages: {}", batch.len(), e);
                }
            }

            // Brief pause for large operations
            if messages.len() > 500 && batch_index % 3 == 2 {
                log::debug!("Brief pause to prevent connection overwhelm");
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
        }

        log::info!(
            "Send messages completed: {} successful, {} failed",
            stats.successful,
            stats.failed
        );
        Ok(stats)
    }

    /// Send messages with repeat count (for bulk operations)
    pub async fn send_messages_with_repeat(
        &mut self,
        queue_name: &str,
        messages: Vec<MessageData>,
        repeat_count: usize,
    ) -> ServiceBusResult<OperationStats> {
        log::info!(
            "Sending {} messages to queue '{}' with repeat count {}",
            messages.len(),
            queue_name,
            repeat_count
        );

        let mut stats = OperationStats::new();
        stats.total = messages.len() * repeat_count;

        if messages.is_empty() || repeat_count == 0 {
            return Ok(stats);
        }

        // Get or create producer for the queue
        let producer = self.get_or_create_producer(queue_name).await?;

        // Convert MessageData to ServiceBusMessage objects and repeat them
        let mut all_messages = Vec::new();
        for _ in 0..repeat_count {
            for message in &messages {
                match self.create_service_bus_message(message) {
                    Ok(sb_message) => all_messages.push(sb_message),
                    Err(e) => {
                        log::error!("Failed to create ServiceBusMessage: {}", e);
                        stats.failed += 1;
                    }
                }
            }
        }

        // Send messages in batches
        let batch_size = self.batch_config.bulk_chunk_size();
        for (batch_index, batch) in all_messages.chunks(batch_size).enumerate() {
            match producer.lock().await.send_messages(batch.to_vec()).await {
                Ok(()) => {
                    stats.successful += batch.len();
                    log::debug!("Successfully sent batch of {} messages", batch.len());
                }
                Err(e) => {
                    stats.failed += batch.len();
                    log::error!("Failed to send batch of {} messages: {}", batch.len(), e);
                }
            }

            // Brief pause for large operations
            if messages.len() > 500 && batch_index % 3 == 2 {
                log::debug!("Brief pause to prevent connection overwhelm");
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
        }

        log::info!(
            "Send messages with repeat completed: {} successful, {} failed",
            stats.successful,
            stats.failed
        );
        Ok(stats)
    }

    /// Send raw message data (used for bulk operations with existing message data)
    pub async fn send_raw_messages(
        &mut self,
        queue_name: &str,
        messages_data: Vec<Vec<u8>>,
        repeat_count: usize,
    ) -> ServiceBusResult<OperationStats> {
        log::info!(
            "Sending {} messages to queue '{}' with repeat count {}",
            messages_data.len(),
            queue_name,
            repeat_count
        );

        let mut stats = OperationStats::new();
        let total_messages = messages_data.len() * repeat_count;
        stats.total = total_messages;

        // Check if this is a DLQ operation
        if queue_name.ends_with("/$deadletterqueue") {
            // For DLQ operations, we need to handle this differently
            // We cannot directly send to DLQ - this needs to be done via dead_letter_message operation
            // on received messages, not by sending new messages
            log::error!("Cannot send messages directly to DLQ: {}", queue_name);
            stats.failed = total_messages;
            return Ok(stats);
        }

        // Get or create producer for the queue
        let producer = self.get_or_create_producer(queue_name).await?;

        // Convert raw data to ServiceBusMessage objects
        let mut all_messages = Vec::new();
        for _ in 0..repeat_count {
            for data in &messages_data {
                let message = azservicebus::ServiceBusMessage::new(data.to_vec());
                all_messages.push(message);
            }
        }

        // Use smaller batch size for bulk operations to prevent Azure Service Bus connection overwhelm
        // Conservative size to avoid race conditions in AMQP layer
        let batch_size = if total_messages > 500 {
            // For large operations, use smaller batches
            self.batch_config.bulk_chunk_size().min(500)
        } else {
            self.batch_config.bulk_chunk_size()
        };

        let mut successful_count = 0;
        let mut failed_count = 0;

        for (batch_index, batch) in all_messages.chunks(batch_size).enumerate() {
            log::debug!(
                "Sending batch {} of {} messages",
                batch_index + 1,
                batch.len()
            );

            match producer.lock().await.send_messages(batch.to_vec()).await {
                Ok(()) => {
                    successful_count += batch.len();
                    log::debug!("Successfully sent batch of {} messages", batch.len());
                }
                Err(e) => {
                    failed_count += batch.len();
                    log::error!("Failed to send batch of {} messages: {}", batch.len(), e);
                }
            }

            // Add a small delay between batches for large operations to prevent overwhelming the connection
            if total_messages > 1000 && batch_index % 3 == 2 {
                log::debug!("Brief pause to prevent connection overwhelm");
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }

        stats.successful = successful_count;
        stats.failed = failed_count;

        log::info!(
            "Bulk send completed: {} successful, {} failed out of {} total",
            stats.successful,
            stats.failed,
            stats.total
        );

        Ok(stats)
    }

    /// Get or create a producer for the specified queue
    async fn get_or_create_producer(
        &mut self,
        queue_name: &str,
    ) -> ServiceBusResult<Arc<Mutex<Producer>>> {
        // Check if producer already exists
        if let Some(producer) = self.producers.get(queue_name) {
            return Ok(Arc::clone(producer));
        }

        // Create new producer
        log::debug!("Creating new producer for queue: {}", queue_name);
        let mut client = self.service_bus_client.lock().await;
        let producer = client
            .create_producer_for_queue(queue_name, ServiceBusSenderOptions::default())
            .await
            .map_err(|e| {
                ServiceBusError::ProducerCreationFailed(format!(
                    "Failed to create producer for queue {}: {}",
                    queue_name, e
                ))
            })?;

        let producer_arc = Arc::new(Mutex::new(producer));
        self.producers
            .insert(queue_name.to_string(), Arc::clone(&producer_arc));

        log::info!("Successfully created producer for queue: {}", queue_name);
        Ok(producer_arc)
    }

    /// Convert MessageData to ServiceBusMessage
    fn create_service_bus_message(
        &self,
        message_data: &MessageData,
    ) -> ServiceBusResult<ServiceBusMessage> {
        let message = ServiceBusMessage::new(message_data.content.clone().into_bytes());

        // Note: Application properties not currently supported in this version
        // If needed, this can be implemented when the azservicebus crate supports it
        if message_data.properties.is_some() {
            log::debug!(
                "Application properties provided but not yet supported by azservicebus crate"
            );
        }

        Ok(message)
    }

    /// Get statistics about active producers
    pub fn get_producer_stats(&self) -> HashMap<String, usize> {
        self.producers
            .keys()
            .map(|queue_name| (queue_name.to_string(), 1)) // 1 producer per queue
            .collect()
    }

    /// Dispose a producer for a specific queue
    pub async fn dispose_producer(&mut self, queue_name: &str) -> ServiceBusResult<()> {
        if let Some(producer) = self.producers.remove(queue_name) {
            log::info!("Disposing producer for queue: {}", queue_name);
            producer.lock().await.dispose().await.map_err(|e| {
                ServiceBusError::InternalError(format!(
                    "Failed to dispose producer for queue {}: {}",
                    queue_name, e
                ))
            })?;
        }
        Ok(())
    }

    /// Dispose all producers
    pub async fn dispose_all_producers(&mut self) -> ServiceBusResult<()> {
        log::info!("Disposing all {} producers", self.producers.len());

        let mut errors = Vec::new();
        for (queue_name, producer) in self.producers.drain() {
            if let Err(e) = producer.lock().await.dispose().await {
                errors.push(format!(
                    "Failed to dispose producer for queue {}: {}",
                    queue_name, e
                ));
            }
        }

        if !errors.is_empty() {
            return Err(ServiceBusError::InternalError(format!(
                "Failed to dispose some producers: {}",
                errors.join("; ")
            )));
        }

        Ok(())
    }

    /// Check if a producer exists for a queue
    pub fn has_producer(&self, queue_name: &str) -> bool {
        self.producers.contains_key(queue_name)
    }

    /// Get number of active producers
    pub fn producer_count(&self) -> usize {
        self.producers.len()
    }

    /// Reset the ServiceBusClient reference after connection reset
    pub async fn reset_client(
        &mut self,
        new_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
    ) -> ServiceBusResult<()> {
        log::info!("Resetting ServiceBusClient reference in ProducerManager");
        
        // Dispose all existing producers
        self.dispose_all_producers().await?;
        
        // Update the client reference
        self.service_bus_client = new_client;
        
        log::info!("ProducerManager client reference updated successfully");
        Ok(())
    }
}
