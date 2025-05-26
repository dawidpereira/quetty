use azservicebus::{
    ServiceBusClient, ServiceBusMessage, ServiceBusSender, ServiceBusSenderOptions,
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct Producer {
    sender: Arc<Mutex<Option<ServiceBusSender>>>,
}

impl PartialEq for Producer {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.sender, &other.sender)
    }
}

impl Producer {
    pub fn new(sender: ServiceBusSender) -> Self {
        Self {
            sender: Arc::new(Mutex::new(Some(sender))),
        }
    }

    /// Send a single message to the queue
    pub async fn send_message(
        &mut self,
        message: ServiceBusMessage,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut guard = self.sender.lock().await;
        if let Some(sender) = guard.as_mut() {
            sender.send_message(message).await?;
            Ok(())
        } else {
            Err("Sender already disposed".into())
        }
    }

    /// Send multiple messages to the queue in a batch
    pub async fn send_messages(
        &mut self,
        messages: Vec<ServiceBusMessage>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut guard = self.sender.lock().await;
        if let Some(sender) = guard.as_mut() {
            sender.send_messages(messages).await?;
            Ok(())
        } else {
            Err("Sender already disposed".into())
        }
    }

    /// Create a new message with the given body
    pub fn create_message(body: Vec<u8>) -> ServiceBusMessage {
        ServiceBusMessage::new(body)
    }

    /// Create a new message with string body
    pub fn create_text_message(text: &str) -> ServiceBusMessage {
        ServiceBusMessage::new(text.as_bytes().to_vec())
    }

    /// Create a new message with JSON body
    pub fn create_json_message<T: serde::Serialize>(
        data: &T,
    ) -> Result<ServiceBusMessage, Box<dyn std::error::Error>> {
        let json_bytes = serde_json::to_vec(data)?;
        let message = ServiceBusMessage::new(json_bytes);
        // Set content type to indicate JSON
        // Note: This depends on the azservicebus API - may need adjustment
        Ok(message)
    }

    /// Dispose the sender and clean up resources
    pub async fn dispose(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut guard = self.sender.lock().await;
        if let Some(sender) = guard.take() {
            sender.dispose().await?;
        }
        Ok(())
    }
}

/// Extension trait for ServiceBusClient to create producers
pub trait ServiceBusClientProducerExt {
    fn create_producer_for_queue(
        &mut self,
        queue_name: impl Into<String> + Send,
        options: ServiceBusSenderOptions,
    ) -> impl std::future::Future<Output = Result<Producer, azure_core::Error>>;
}

impl<RP> ServiceBusClientProducerExt for ServiceBusClient<RP>
where
    RP: azservicebus::ServiceBusRetryPolicy
        + From<azservicebus::ServiceBusRetryOptions>
        + Send
        + Sync
        + 'static,
{
    async fn create_producer_for_queue(
        &mut self,
        queue_name: impl Into<String> + Send,
        options: ServiceBusSenderOptions,
    ) -> Result<Producer, azure_core::Error> {
        let sender = self.create_sender(queue_name, options).await.map_err(|e| {
            azure_core::Error::message(
                azure_core::error::ErrorKind::Other,
                format!("Sender error: {e}"),
            )
        })?;

        Ok(Producer::new(sender))
    }
}

