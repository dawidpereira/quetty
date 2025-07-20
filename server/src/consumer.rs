use azservicebus::receiver::DeadLetterOptions;
use azservicebus::{ServiceBusClient, ServiceBusReceiver, ServiceBusReceiverOptions};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::model::MessageModel;

/// A wrapper around Azure Service Bus receiver for consuming messages from queues.
///
/// The Consumer provides a high-level interface for receiving, processing, and managing
/// messages from Azure Service Bus queues. It supports both peek operations (non-destructive)
/// and receive operations (which lock messages for processing).
///
/// # Thread Safety
///
/// The Consumer is thread-safe and can be shared across async tasks. The underlying
/// receiver is protected by a mutex to ensure safe concurrent access.
///
/// # Examples
///
/// ```no_run
/// use quetty_server::consumer::Consumer;
/// use azservicebus::{ServiceBusReceiver, ServiceBusReceiverOptions};
///
/// async fn example(receiver: ServiceBusReceiver) {
///     let mut consumer = Consumer::new(receiver);
///
///     // Peek at messages without consuming them
///     let messages = consumer.peek_messages(10, None).await?;
///
///     // Receive messages for processing
///     let received = consumer.receive_messages_with_timeout(5, std::time::Duration::from_secs(10)).await?;
///
///     // Process and complete messages
///     for message in &received {
///         consumer.complete_message(message).await?;
///     }
/// }
/// ```
#[derive(Debug)]
pub struct Consumer {
    receiver: Arc<Mutex<Option<ServiceBusReceiver>>>,
}

impl PartialEq for Consumer {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.receiver, &other.receiver)
    }
}

impl Consumer {
    /// Creates a new Consumer wrapping the provided Service Bus receiver.
    ///
    /// # Arguments
    ///
    /// * `receiver` - The Azure Service Bus receiver to wrap
    pub fn new(receiver: ServiceBusReceiver) -> Self {
        Self {
            receiver: Arc::new(Mutex::new(Some(receiver))),
        }
    }

    /// Peeks at messages in the queue without consuming them.
    ///
    /// This operation allows you to inspect messages without locking them
    /// or affecting their delivery count. Useful for browsing queue contents.
    ///
    /// # Arguments
    ///
    /// * `max_count` - Maximum number of messages to peek at
    /// * `from_sequence_number` - Optional starting sequence number
    ///
    /// # Returns
    ///
    /// Vector of MessageModel instances representing the peeked messages
    ///
    /// # Errors
    ///
    /// Returns an error if the receiver has been disposed or if the Service Bus operation fails
    pub async fn peek_messages(
        &mut self,
        max_count: u32,
        from_sequence_number: Option<i64>,
    ) -> Result<Vec<MessageModel>, Box<dyn std::error::Error>> {
        let mut guard = self.receiver.lock().await;
        if let Some(receiver) = guard.as_mut() {
            let messages = receiver
                .peek_messages(max_count, from_sequence_number)
                .await?;
            let result = MessageModel::try_convert_messages_collect(messages);
            Ok(result)
        } else {
            Err("Receiver already disposed".into())
        }
    }

    /// Receives messages from the queue with a timeout.
    ///
    /// This operation locks the received messages for processing. The messages
    /// must be completed, abandoned, or dead-lettered to release the lock.
    ///
    /// # Arguments
    ///
    /// * `max_count` - Maximum number of messages to receive
    /// * `timeout` - Maximum time to wait for messages
    ///
    /// # Returns
    ///
    /// Vector of received messages that are locked for processing.
    /// Returns an empty vector if the timeout expires before messages are available.
    ///
    /// # Errors
    ///
    /// Returns an error if the receiver has been disposed or if the Service Bus operation fails
    pub async fn receive_messages_with_timeout(
        &mut self,
        max_count: u32,
        timeout: Duration,
    ) -> Result<Vec<azservicebus::ServiceBusReceivedMessage>, Box<dyn std::error::Error>> {
        let mut guard = self.receiver.lock().await;
        if let Some(receiver) = guard.as_mut() {
            match tokio::time::timeout(timeout, receiver.receive_messages(max_count)).await {
                Ok(result) => result.map_err(|e| e.into()),
                Err(_) => {
                    // Timeout occurred - return empty vector instead of error
                    log::debug!(
                        "receive_messages timed out after {timeout:?}, returning empty result"
                    );
                    Ok(Vec::new())
                }
            }
        } else {
            Err("Receiver already disposed".into())
        }
    }

    /// Abandons a received message, returning it to the queue.
    ///
    /// The message becomes available for redelivery and its delivery count is incremented.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to abandon
    ///
    /// # Errors
    ///
    /// Returns an error if the receiver has been disposed or if the Service Bus operation fails
    pub async fn abandon_message(
        &mut self,
        message: &azservicebus::ServiceBusReceivedMessage,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut guard = self.receiver.lock().await;
        if let Some(receiver) = guard.as_mut() {
            receiver.abandon_message(message, None).await?;
            Ok(())
        } else {
            Err("Receiver already disposed".into())
        }
    }

    /// Moves a message to the dead letter queue.
    ///
    /// Dead lettered messages are permanently removed from the main queue
    /// and can be inspected in the dead letter queue for debugging.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to dead letter
    /// * `reason` - Optional reason for dead lettering
    /// * `error_description` - Optional error description
    ///
    /// # Errors
    ///
    /// Returns an error if the receiver has been disposed or if the Service Bus operation fails
    pub async fn dead_letter_message(
        &mut self,
        message: &azservicebus::ServiceBusReceivedMessage,
        reason: Option<String>,
        error_description: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut guard = self.receiver.lock().await;
        if let Some(receiver) = guard.as_mut() {
            let options = DeadLetterOptions {
                dead_letter_reason: reason,
                dead_letter_error_description: error_description,
                properties_to_modify: None,
            };
            receiver.dead_letter_message(message, options).await?;
            Ok(())
        } else {
            Err("Receiver already disposed".into())
        }
    }

    /// Completes a message, removing it from the queue.
    ///
    /// This indicates successful processing of the message.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to complete
    ///
    /// # Errors
    ///
    /// Returns an error if the receiver has been disposed or if the Service Bus operation fails
    pub async fn complete_message(
        &mut self,
        message: &azservicebus::ServiceBusReceivedMessage,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut guard = self.receiver.lock().await;
        if let Some(receiver) = guard.as_mut() {
            receiver.complete_message(message).await?;
            Ok(())
        } else {
            Err("Receiver already disposed".into())
        }
    }

    /// Completes multiple messages in a batch for better performance.
    ///
    /// Attempts to complete all provided messages, logging results for each.
    /// If any message fails to complete, the operation continues with remaining
    /// messages and returns an error indicating the failure count.
    ///
    /// # Arguments
    ///
    /// * `messages` - Slice of messages to complete
    ///
    /// # Returns
    ///
    /// `Ok(())` if all messages were completed successfully,
    /// `Err` if any messages failed to complete
    ///
    /// # Errors
    ///
    /// Returns an error if the receiver has been disposed or if any message completion fails
    pub async fn complete_messages(
        &mut self,
        messages: &[azservicebus::ServiceBusReceivedMessage],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut guard = self.receiver.lock().await;
        if let Some(receiver) = guard.as_mut() {
            // Complete messages one by one since batch completion may not be available
            let mut completed_count = 0;
            let mut failed_count = 0;

            for message in messages {
                let message_id = message
                    .message_id()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let sequence = message.sequence_number();

                match receiver.complete_message(message).await {
                    Ok(()) => {
                        completed_count += 1;
                        log::debug!(
                            "Successfully completed message {message_id} (sequence: {sequence})"
                        );
                    }
                    Err(e) => {
                        failed_count += 1;
                        log::error!(
                            "Failed to complete message {message_id} (sequence: {sequence}): {e}"
                        );
                        // Don't return early - try to complete as many as possible
                    }
                }
            }

            log::info!(
                "Batch completion result: {} successful, {} failed out of {} messages",
                completed_count,
                failed_count,
                messages.len()
            );

            if failed_count > 0 {
                return Err(format!(
                    "Failed to complete {} out of {} messages",
                    failed_count,
                    messages.len()
                )
                .into());
            }

            Ok(())
        } else {
            Err("Receiver already disposed".into())
        }
    }

    /// Abandons multiple messages in a batch.
    ///
    /// All provided messages are returned to the queue for redelivery.
    ///
    /// # Arguments
    ///
    /// * `messages` - Slice of messages to abandon
    ///
    /// # Errors
    ///
    /// Returns an error if the receiver has been disposed or if any abandon operation fails
    pub async fn abandon_messages(
        &mut self,
        messages: &[azservicebus::ServiceBusReceivedMessage],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut guard = self.receiver.lock().await;
        if let Some(receiver) = guard.as_mut() {
            for message in messages {
                receiver.abandon_message(message, None).await?;
            }
            Ok(())
        } else {
            Err("Receiver already disposed".into())
        }
    }

    /// Renews the lock on a received message to extend processing time.
    ///
    /// This prevents the message from becoming available for redelivery
    /// while it's still being processed.
    ///
    /// # Arguments
    ///
    /// * `message` - The message whose lock should be renewed
    ///
    /// # Errors
    ///
    /// Returns an error if the receiver has been disposed or if the lock renewal fails
    pub async fn renew_message_lock(
        &mut self,
        message: &mut azservicebus::ServiceBusReceivedMessage,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut guard = self.receiver.lock().await;
        if let Some(receiver) = guard.as_mut() {
            receiver.renew_message_lock(message).await?;
            Ok(())
        } else {
            Err("Receiver already disposed".into())
        }
    }

    /// Renews locks on multiple messages.
    ///
    /// Attempts to renew locks for all provided messages, logging results.
    /// Continues processing all messages even if some renewals fail.
    ///
    /// # Arguments
    ///
    /// * `messages` - Mutable slice of messages whose locks should be renewed
    ///
    /// # Errors
    ///
    /// Returns an error if the receiver has been disposed. Lock renewal failures
    /// are logged but do not cause the method to return an error.
    pub async fn renew_message_locks(
        &mut self,
        messages: &mut [azservicebus::ServiceBusReceivedMessage],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut guard = self.receiver.lock().await;
        if let Some(receiver) = guard.as_mut() {
            let mut renewed_count = 0;
            let mut failed_count = 0;

            for message in messages.iter_mut() {
                let message_id = message
                    .message_id()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let sequence = message.sequence_number();

                match receiver.renew_message_lock(message).await {
                    Ok(()) => {
                        renewed_count += 1;
                        log::debug!(
                            "Successfully renewed lock for message {message_id} (sequence: {sequence})"
                        );
                    }
                    Err(e) => {
                        failed_count += 1;
                        log::warn!(
                            "Failed to renew lock for message {message_id} (sequence: {sequence}): {e}"
                        );
                        // Continue trying to renew other locks
                    }
                }
            }

            log::debug!(
                "Lock renewal result: {} successful, {} failed out of {} messages",
                renewed_count,
                failed_count,
                messages.len()
            );

            if failed_count > 0 {
                log::warn!(
                    "Failed to renew locks for {} out of {} messages - some may expire during processing",
                    failed_count,
                    messages.len()
                );
            }

            Ok(())
        } else {
            Err("Receiver already disposed".into())
        }
    }

    /// Disposes the underlying Service Bus receiver, releasing all resources.
    ///
    /// After disposal, all other operations on this Consumer will fail.
    ///
    /// # Errors
    ///
    /// Returns an error if the disposal operation fails
    pub async fn dispose(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut guard = self.receiver.lock().await;
        if let Some(receiver) = guard.take() {
            receiver.dispose().await?;
        }
        Ok(())
    }

    /// Receives deferred messages by their sequence numbers.
    ///
    /// This allows operations (delete/complete) on messages that were previously
    /// deferred without re-activating them in the main queue first.
    ///
    /// # Arguments
    ///
    /// * `sequence_numbers` - Array of sequence numbers for the deferred messages
    ///
    /// # Returns
    ///
    /// Vector of received deferred messages that are locked for processing
    ///
    /// # Errors
    ///
    /// Returns an error if the receiver has been disposed or if the Service Bus operation fails
    pub async fn receive_deferred_messages(
        &mut self,
        sequence_numbers: &[i64],
    ) -> Result<
        Vec<azservicebus::ServiceBusReceivedMessage>,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        let mut guard = self.receiver.lock().await;
        if let Some(receiver) = guard.as_mut() {
            let messages = receiver
                .receive_deferred_messages(sequence_numbers.to_vec())
                .await?;
            Ok(messages)
        } else {
            Err("Receiver already disposed".into())
        }
    }
}

/// Extension trait for ServiceBusClient to create Consumer instances.
///
/// This trait provides a convenient method to create a Consumer directly
/// from a ServiceBusClient without manually creating the receiver first.
pub trait ServiceBusClientExt {
    /// Creates a Consumer for the specified queue.
    ///
    /// # Arguments
    ///
    /// * `queue_name` - Name of the queue to create a consumer for
    /// * `options` - Configuration options for the receiver
    ///
    /// # Returns
    ///
    /// A Consumer instance configured for the specified queue
    ///
    /// # Errors
    ///
    /// Returns an error if the receiver creation fails
    fn create_consumer_for_queue(
        &mut self,
        queue_name: impl Into<String> + Send,
        options: ServiceBusReceiverOptions,
    ) -> impl Future<Output = Result<Consumer, azure_core::Error>>;
}

impl<RP> ServiceBusClientExt for ServiceBusClient<RP>
where
    RP: azservicebus::ServiceBusRetryPolicy
        + From<azservicebus::ServiceBusRetryOptions>
        + Send
        + Sync
        + 'static,
{
    /// Creates a Consumer for the specified queue using this ServiceBusClient.
    ///
    /// This method handles the creation of the underlying receiver and wraps it
    /// in a Consumer instance for easier usage.
    async fn create_consumer_for_queue(
        &mut self,
        queue_name: impl Into<String> + Send,
        options: ServiceBusReceiverOptions,
    ) -> Result<Consumer, azure_core::Error> {
        let receiver = self
            .create_receiver_for_queue(queue_name, options)
            .await
            .map_err(|e| {
                azure_core::Error::message(
                    azure_core::error::ErrorKind::Other,
                    format!("Receiver error: {e}"),
                )
            })?;

        Ok(Consumer::new(receiver))
    }
}
