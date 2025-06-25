use super::resource_guard::acquire_lock_with_timeout;
use super::types::{
    BatchConfig, BulkOperationContext, BulkOperationResult, MessageIdentifier,
    ProcessTargetMessagesParams, QueueOperationType,
};
use crate::consumer::Consumer;
use crate::producer::ServiceBusClientProducerExt;
use azservicebus::{ServiceBusMessage, ServiceBusSenderOptions};
use azservicebus::core::BasicRetryPolicy;
use azservicebus::ServiceBusClient;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Handles message sending operations to Service Bus queues
pub struct MessageSender {
    config: BatchConfig,
}

impl MessageSender {
    pub fn new(config: BatchConfig) -> Self {
        Self { config }
    }

    /// Process target messages by sending them to the target queue
    pub async fn process_target_messages(
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
    pub async fn send_messages_to_queue(
        &self,
        queue_name: &str,
        messages: Vec<ServiceBusMessage>,
        service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        if messages.is_empty() {
            return Ok(());
        }

        log::debug!("Creating producer for queue: {}", queue_name);

        let mut client = acquire_lock_with_timeout(
            &service_bus_client,
            "send_messages",
            Duration::from_secs(self.config.lock_timeout_secs()),
            None, // No cancellation token available in this context
        )
        .await?;
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

    /// Complete processed messages (marks them as processed and removes from queue)
    async fn complete_processed_messages(
        &self,
        messages: &[azservicebus::ServiceBusReceivedMessage],
        context: &BulkOperationContext,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        log::debug!("Completing {} processed messages", messages.len());

        let mut consumer_guard = context.consumer.lock().await;

        for message in messages {
            if context.is_cancelled() {
                log::warn!("Operation cancelled, stopping message completion");
                break;
            }

            let message_id = message
                .message_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            consumer_guard
                .complete_message(message)
                .await
                .map_err(|e| -> Box<dyn Error + Send + Sync> {
                    log::error!("Failed to complete message {}: {}", message_id, e);
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to complete message {}: {}", message_id, e),
                    ))
                })?;
        }

        drop(consumer_guard);
        log::info!("Successfully completed {} messages", messages.len());
        Ok(())
    }

    /// Track successfully processed messages in the result
    fn track_successful_messages(
        &self,
        messages: &[azservicebus::ServiceBusReceivedMessage],
        target_map: &HashMap<String, MessageIdentifier>,
        result: &mut BulkOperationResult,
    ) {
        log::debug!("Tracking {} successful messages", messages.len());

        for message in messages {
            if let Some(message_id) = message.message_id() {
                if let Some(target_id) = target_map.get(&*message_id) {
                    result.add_successful_message(target_id.clone());
                }
            }
        }

        log::debug!(
            "Tracked messages - Total successful: {}",
            result.successful
        );
    }
} 