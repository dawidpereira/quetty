use azservicebus::receiver::DeadLetterOptions;
use azservicebus::{ServiceBusClient, ServiceBusReceiver, ServiceBusReceiverOptions};
use std::sync::Arc;
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

    pub async fn receive_messages(
        &mut self,
        max_count: u32,
    ) -> Result<Vec<azservicebus::ServiceBusReceivedMessage>, Box<dyn std::error::Error>> {
        let mut guard = self.receiver.lock().await;
        if let Some(receiver) = guard.as_mut() {
            let messages = receiver.receive_messages(max_count).await?;
            Ok(messages)
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
            for message in messages {
                receiver.complete_message(message).await?;
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

    pub async fn dispose(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut guard = self.receiver.lock().await;
        if let Some(receiver) = guard.take() {
            receiver.dispose().await?;
        }
        Ok(())
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
