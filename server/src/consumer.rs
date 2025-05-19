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
