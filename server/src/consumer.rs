use azservicebus::receiver::DeadLetterOptions;
use azservicebus::{ServiceBusClient, ServiceBusReceiver, ServiceBusReceiverOptions};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::model::MessageModel;

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
    pub fn new(receiver: ServiceBusReceiver) -> Self {
        Self {
            receiver: Arc::new(Mutex::new(Some(receiver))),
        }
    }

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
                        "receive_messages timed out after {:?}, returning empty result",
                        timeout
                    );
                    Ok(Vec::new())
                }
            }
        } else {
            Err("Receiver already disposed".into())
        }
    }

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

    /// Complete multiple messages in a batch for better performance
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
                            "Successfully completed message {} (sequence: {})",
                            message_id,
                            sequence
                        );
                    }
                    Err(e) => {
                        failed_count += 1;
                        log::error!(
                            "Failed to complete message {} (sequence: {}): {}",
                            message_id,
                            sequence,
                            e
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

    /// Abandon multiple messages in a batch
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

    /// Renew the lock on a received message to extend processing time
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

    /// Renew locks on multiple messages
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
                            "Successfully renewed lock for message {} (sequence: {})",
                            message_id,
                            sequence
                        );
                    }
                    Err(e) => {
                        failed_count += 1;
                        log::warn!(
                            "Failed to renew lock for message {} (sequence: {}): {}",
                            message_id,
                            sequence,
                            e
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

    pub async fn dispose(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut guard = self.receiver.lock().await;
        if let Some(receiver) = guard.take() {
            receiver.dispose().await?;
        }
        Ok(())
    }

    /// Receive deferred messages by sequence numbers. This allows operations (delete/complete)
    /// on messages that were previously deferred without re-activating them first.
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

pub trait ServiceBusClientExt {
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
