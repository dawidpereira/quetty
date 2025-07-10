use azservicebus::{
    ServiceBusClient, ServiceBusMessage, ServiceBusSender, ServiceBusSenderOptions,
};
use std::sync::Arc;
use tokio::sync::Mutex;

/// A wrapper around Azure Service Bus sender for producing messages to queues.
///
/// The Producer provides a high-level interface for sending messages to Azure Service Bus queues.
/// It supports both single message sending and batch operations for improved performance.
///
/// # Thread Safety
///
/// The Producer is thread-safe and can be shared across async tasks. The underlying
/// sender is protected by a mutex to ensure safe concurrent access.
///
/// # Examples
///
/// ```no_run
/// use server::producer::Producer;
/// use azservicebus::{ServiceBusSender, ServiceBusMessage};
///
/// async fn example(sender: ServiceBusSender) -> Result<(), Box<dyn std::error::Error>> {
///     let mut producer = Producer::new(sender);
///
///     // Send a single text message
///     let message = Producer::create_text_message("Hello, world!");
///     producer.send_message(message).await?;
///
///     // Send multiple messages in a batch
///     let messages = vec![
///         Producer::create_text_message("Message 1"),
///         Producer::create_text_message("Message 2"),
///     ];
///     producer.send_messages(messages).await?;
///
///     Ok(())
/// }
/// ```
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
    /// Creates a new Producer wrapping the provided Service Bus sender.
    ///
    /// # Arguments
    ///
    /// * `sender` - The Azure Service Bus sender to wrap
    pub fn new(sender: ServiceBusSender) -> Self {
        Self {
            sender: Arc::new(Mutex::new(Some(sender))),
        }
    }

    /// Sends a single message to the queue.
    ///
    /// # Arguments
    ///
    /// * `message` - The ServiceBusMessage to send
    ///
    /// # Errors
    ///
    /// Returns an error if the sender has been disposed or if the Service Bus operation fails
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

    /// Sends multiple messages to the queue in a batch operation.
    ///
    /// Batch sending is more efficient than sending individual messages
    /// when you need to send multiple messages at once.
    ///
    /// # Arguments
    ///
    /// * `messages` - Vector of ServiceBusMessage instances to send
    ///
    /// # Errors
    ///
    /// Returns an error if the sender has been disposed or if the Service Bus operation fails
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

    /// Creates a new message with the given byte array body.
    ///
    /// # Arguments
    ///
    /// * `body` - The message body as a byte vector
    ///
    /// # Returns
    ///
    /// A ServiceBusMessage with the specified body
    pub fn create_message(body: Vec<u8>) -> ServiceBusMessage {
        ServiceBusMessage::new(body)
    }

    /// Creates a new message with a string body.
    ///
    /// # Arguments
    ///
    /// * `text` - The message text content
    ///
    /// # Returns
    ///
    /// A ServiceBusMessage with the text as the body
    pub fn create_text_message(text: &str) -> ServiceBusMessage {
        ServiceBusMessage::new(text.as_bytes().to_vec())
    }

    /// Creates a new message with a JSON-serialized body.
    ///
    /// # Arguments
    ///
    /// * `data` - The data to serialize as JSON
    ///
    /// # Returns
    ///
    /// A ServiceBusMessage with the JSON data as the body
    ///
    /// # Errors
    ///
    /// Returns an error if the data cannot be serialized to JSON
    pub fn create_json_message<T: serde::Serialize>(
        data: &T,
    ) -> Result<ServiceBusMessage, Box<dyn std::error::Error>> {
        let json_bytes = serde_json::to_vec(data)?;
        let message = ServiceBusMessage::new(json_bytes);
        // Set content type to indicate JSON
        // Note: This depends on the azservicebus API - may need adjustment
        Ok(message)
    }

    /// Disposes the underlying Service Bus sender, releasing all resources.
    ///
    /// After disposal, all other operations on this Producer will fail.
    ///
    /// # Errors
    ///
    /// Returns an error if the disposal operation fails
    pub async fn dispose(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut guard = self.sender.lock().await;
        if let Some(sender) = guard.take() {
            sender.dispose().await?;
        }
        Ok(())
    }
}

/// Extension trait for ServiceBusClient to create Producer instances.
///
/// This trait provides a convenient method to create a Producer directly
/// from a ServiceBusClient without manually creating the sender first.
pub trait ServiceBusClientProducerExt {
    /// Creates a Producer for the specified queue.
    ///
    /// # Arguments
    ///
    /// * `queue_name` - Name of the queue to create a producer for
    /// * `options` - Configuration options for the sender
    ///
    /// # Returns
    ///
    /// A Producer instance configured for the specified queue
    ///
    /// # Errors
    ///
    /// Returns an error if the sender creation fails
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
    /// Creates a Producer for the specified queue using this ServiceBusClient.
    ///
    /// This method handles the creation of the underlying sender and wraps it
    /// in a Producer instance for easier usage.
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
