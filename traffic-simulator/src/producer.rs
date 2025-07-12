use azservicebus::{ServiceBusMessage, ServiceBusSender};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Simplified producer for traffic simulation
#[derive(Debug)]
pub struct Producer {
    sender: Arc<Mutex<Option<ServiceBusSender>>>,
}

impl Producer {
    pub fn new(sender: ServiceBusSender) -> Self {
        Self {
            sender: Arc::new(Mutex::new(Some(sender))),
        }
    }

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

    pub fn create_text_message(text: &str) -> ServiceBusMessage {
        ServiceBusMessage::new(text.as_bytes().to_vec())
    }

    pub fn create_json_message<T: serde::Serialize>(
        data: &T,
    ) -> Result<ServiceBusMessage, Box<dyn std::error::Error>> {
        let json_bytes = serde_json::to_vec(data)?;
        Ok(ServiceBusMessage::new(json_bytes))
    }

    pub async fn dispose(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut guard = self.sender.lock().await;
        if let Some(sender) = guard.take() {
            sender.dispose().await?;
        }
        Ok(())
    }
}
