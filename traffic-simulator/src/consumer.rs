use azservicebus::{ServiceBusReceiver, ServiceBusReceivedMessage};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Simplified consumer for traffic simulation
#[derive(Debug)]
pub struct Consumer {
    receiver: Arc<Mutex<Option<ServiceBusReceiver>>>,
}

impl Consumer {
    pub fn new(receiver: ServiceBusReceiver) -> Self {
        Self {
            receiver: Arc::new(Mutex::new(Some(receiver))),
        }
    }

    pub async fn receive_messages_with_timeout(
        &mut self,
        max_count: u32,
        timeout: Duration,
    ) -> Result<Vec<ServiceBusReceivedMessage>, Box<dyn std::error::Error>> {
        let mut guard = self.receiver.lock().await;
        if let Some(receiver) = guard.as_mut() {
            match tokio::time::timeout(timeout, receiver.receive_messages(max_count)).await {
                Ok(result) => result.map_err(|e| e.into()),
                Err(_) => Ok(Vec::new()), // Timeout - return empty
            }
        } else {
            Err("Receiver already disposed".into())
        }
    }

    pub async fn complete_message(&mut self, message: &ServiceBusReceivedMessage) -> Result<(), Box<dyn std::error::Error>> {
        let mut guard = self.receiver.lock().await;
        if let Some(receiver) = guard.as_mut() {
            receiver.complete_message(message).await?;
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
