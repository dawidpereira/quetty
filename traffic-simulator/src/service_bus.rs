use crate::{consumer::Consumer, producer::Producer};
use azservicebus::{
    core::BasicRetryPolicy, ServiceBusClient, ServiceBusClientOptions, ServiceBusReceiverOptions,
    ServiceBusSenderOptions,
};

/// Service Bus client wrapper for traffic simulation
pub struct ServiceBusManager {
    client: Option<ServiceBusClient<BasicRetryPolicy>>,
}

impl ServiceBusManager {
    pub async fn new(connection_string: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let client = ServiceBusClient::new_from_connection_string(
            connection_string,
            ServiceBusClientOptions::default(),
        )
        .await?;
        Ok(Self {
            client: Some(client),
        })
    }

    pub async fn create_producer(
        &mut self,
        queue_name: &str,
    ) -> Result<Producer, Box<dyn std::error::Error>> {
        if let Some(client) = &mut self.client {
            let sender = client
                .create_sender(queue_name, ServiceBusSenderOptions::default())
                .await?;
            Ok(Producer::new(sender))
        } else {
            Err("Client not initialized".into())
        }
    }

    pub async fn create_consumer(
        &mut self,
        queue_name: &str,
    ) -> Result<Consumer, Box<dyn std::error::Error>> {
        if let Some(client) = &mut self.client {
            let receiver = client
                .create_receiver_for_queue(queue_name, ServiceBusReceiverOptions::default())
                .await?;
            Ok(Consumer::new(receiver))
        } else {
            Err("Client not initialized".into())
        }
    }
}
