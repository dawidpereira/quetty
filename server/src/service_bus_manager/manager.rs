use super::AzureAdConfig;
use super::azure_management_client::StatisticsConfig;
use super::command_handlers::*;
use super::commands::ServiceBusCommand;
use super::consumer_manager::ConsumerManager;
use super::errors::{ServiceBusError, ServiceBusResult};
use super::producer_manager::ProducerManager;
use super::queue_statistics_service::QueueStatisticsService;
use super::responses::ServiceBusResponse;
use super::types::QueueInfo;
use crate::bulk_operations::{BulkOperationHandler, types::BatchConfig};
use azservicebus::{ServiceBusClient, ServiceBusClientOptions, core::BasicRetryPolicy};
use std::sync::Arc;
use tokio::sync::Mutex;

/// The main service bus manager that orchestrates all service bus operations
pub struct ServiceBusManager {
    queue_handler: QueueCommandHandler,
    message_handler: MessageCommandHandler,
    send_handler: SendCommandHandler,
    status_handler: StatusCommandHandler,
    bulk_handler: BulkCommandHandler,
    resource_handler: ResourceCommandHandler,

    // Shared state
    consumer_manager: Arc<Mutex<ConsumerManager>>,
    producer_manager: Arc<Mutex<ProducerManager>>,
    service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,

    // Connection reset capability
    connection_string: String,

    // Error tracking
    last_error: Arc<Mutex<Option<String>>>,
}

impl ServiceBusManager {
    /// Create a new ServiceBusManager
    pub fn new(
        service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
        azure_ad_config: AzureAdConfig,
        statistics_config: StatisticsConfig,
        batch_config: BatchConfig,
        connection_string: String,
    ) -> Self {
        let consumer_manager = Arc::new(Mutex::new(ConsumerManager::new(
            service_bus_client.clone(),
            batch_config.clone(),
        )));
        let producer_manager = Arc::new(Mutex::new(ProducerManager::new(
            service_bus_client.clone(),
            batch_config.clone(),
        )));
        let bulk_handler_inner = Arc::new(BulkOperationHandler::new(batch_config.clone()));
        let statistics_service = Arc::new(QueueStatisticsService::new(
            statistics_config,
            azure_ad_config,
        ));

        Self {
            queue_handler: QueueCommandHandler::new(consumer_manager.clone(), statistics_service),
            message_handler: MessageCommandHandler::new(consumer_manager.clone()),
            send_handler: SendCommandHandler::new(producer_manager.clone()),
            status_handler: StatusCommandHandler::new(
                consumer_manager.clone(),
                producer_manager.clone(),
            ),
            bulk_handler: BulkCommandHandler::new(
                bulk_handler_inner,
                consumer_manager.clone(),
                producer_manager.clone(),
                batch_config.clone(),
            ),
            resource_handler: ResourceCommandHandler::new(
                consumer_manager.clone(),
                producer_manager.clone(),
            ),
            consumer_manager,
            producer_manager,
            service_bus_client,
            connection_string,
            last_error: Arc::new(Mutex::new(None)),
        }
    }

    /// Execute a service bus command and return the response
    pub async fn execute_command(&self, command: ServiceBusCommand) -> ServiceBusResponse {
        log::debug!("Executing command: {command:?}");

        let result = self.handle_command(command).await;

        match result {
            Ok(response) => {
                let mut last_error = self.last_error.lock().await;
                *last_error = None;
                response
            }
            Err(error) => {
                let mut last_error = self.last_error.lock().await;
                *last_error = Some(error.to_string());
                log::error!("Command execution failed: {error}");
                ServiceBusResponse::Error { error }
            }
        }
    }

    /// Handle a command using specialized command handlers
    async fn handle_command(
        &self,
        command: ServiceBusCommand,
    ) -> ServiceBusResult<ServiceBusResponse> {
        match command {
            // Queue management commands
            ServiceBusCommand::SwitchQueue {
                queue_name,
                queue_type,
            } => {
                self.queue_handler
                    .handle_switch_queue(queue_name, queue_type)
                    .await
            }
            ServiceBusCommand::GetCurrentQueue => {
                self.queue_handler.handle_get_current_queue().await
            }
            ServiceBusCommand::GetQueueStatistics {
                queue_name,
                queue_type,
            } => {
                self.queue_handler
                    .handle_get_queue_statistics(queue_name, queue_type)
                    .await
            }

            // Message retrieval commands
            ServiceBusCommand::PeekMessages {
                max_count,
                from_sequence,
            } => {
                self.message_handler
                    .handle_peek_messages(max_count, from_sequence)
                    .await
            }
            ServiceBusCommand::ReceiveMessages { max_count } => {
                self.message_handler
                    .handle_receive_messages(max_count)
                    .await
            }
            ServiceBusCommand::CompleteMessage { message_id } => {
                self.message_handler
                    .handle_complete_message(message_id)
                    .await
            }
            ServiceBusCommand::AbandonMessage { message_id } => {
                self.message_handler
                    .handle_abandon_message(message_id)
                    .await
            }
            ServiceBusCommand::DeadLetterMessage {
                message_id,
                reason,
                error_description,
            } => {
                self.message_handler
                    .handle_dead_letter_message(message_id, reason, error_description)
                    .await
            }

            // Bulk operation commands
            ServiceBusCommand::BulkComplete { message_ids } => {
                self.bulk_handler.handle_bulk_complete(message_ids).await
            }
            ServiceBusCommand::BulkDelete {
                message_ids,
                max_position,
            } => {
                self.bulk_handler
                    .handle_bulk_delete(message_ids, max_position)
                    .await
            }
            ServiceBusCommand::BulkAbandon { message_ids } => {
                self.bulk_handler.handle_bulk_abandon(message_ids).await
            }
            ServiceBusCommand::BulkDeadLetter {
                message_ids,
                reason,
                error_description,
            } => {
                self.bulk_handler
                    .handle_bulk_dead_letter(message_ids, reason, error_description)
                    .await
            }
            ServiceBusCommand::BulkSend {
                message_ids,
                target_queue,
                should_delete_source,
                repeat_count,
                max_position,
            } => {
                self.bulk_handler
                    .handle_bulk_send(
                        message_ids,
                        target_queue,
                        should_delete_source,
                        repeat_count,
                        max_position,
                    )
                    .await
            }
            ServiceBusCommand::BulkSendPeeked {
                messages_data,
                target_queue,
                repeat_count,
            } => {
                self.bulk_handler
                    .handle_bulk_send_peeked(messages_data, target_queue, repeat_count)
                    .await
            }

            // Send operation commands
            ServiceBusCommand::SendMessage {
                queue_name,
                message,
            } => {
                self.send_handler
                    .handle_send_message(queue_name, message)
                    .await
            }
            ServiceBusCommand::SendMessages {
                queue_name,
                messages,
            } => {
                self.send_handler
                    .handle_send_messages(queue_name, messages)
                    .await
            }

            // Status and health commands
            ServiceBusCommand::GetConnectionStatus => {
                self.status_handler.handle_get_connection_status().await
            }
            ServiceBusCommand::GetQueueStats { queue_name } => {
                self.status_handler.handle_get_queue_stats(queue_name).await
            }

            // Resource management commands
            ServiceBusCommand::DisposeConsumer => {
                self.resource_handler.handle_dispose_consumer().await
            }
            ServiceBusCommand::DisposeAllResources => {
                self.resource_handler.handle_dispose_all_resources().await
            }
            ServiceBusCommand::ResetConnection => self.handle_reset_connection().await,
        }
    }

    // Static methods for Azure AD operations (keep existing functionality)
    pub async fn get_azure_ad_token(
        config: &AzureAdConfig,
    ) -> Result<String, Box<dyn std::error::Error>> {
        config.get_azure_ad_token().await
    }

    pub async fn list_queues_azure_ad(
        config: &AzureAdConfig,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        config.list_queues_azure_ad().await
    }

    pub async fn list_namespaces_azure_ad(
        config: &AzureAdConfig,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        config.list_namespaces_azure_ad().await
    }

    // Helper methods with clean interfaces
    pub async fn get_current_queue(&self) -> Option<QueueInfo> {
        let consumer = self.consumer_manager.lock().await;
        consumer.current_queue().cloned()
    }

    pub async fn is_connected(&self) -> bool {
        let consumer = self.consumer_manager.lock().await;
        let producer = self.producer_manager.lock().await;
        consumer.is_consumer_ready() || producer.producer_count() > 0
    }

    pub async fn get_producer_count(&self) -> usize {
        let producer = self.producer_manager.lock().await;
        producer.producer_count()
    }

    pub async fn get_last_error(&self) -> Option<String> {
        let last_error = self.last_error.lock().await;
        last_error.clone()
    }

    /// Reset the entire AMQP connection by creating a new ServiceBusClient
    pub async fn handle_reset_connection(&self) -> ServiceBusResult<ServiceBusResponse> {
        log::info!("Resetting ServiceBus connection completely");

        // First dispose all existing resources
        let _ = self.resource_handler.handle_dispose_all_resources().await;

        // Create a new ServiceBusClient from the stored connection string
        let new_client = ServiceBusClient::new_from_connection_string(
            &self.connection_string,
            ServiceBusClientOptions::default(),
        )
        .await
        .map_err(|e| {
            ServiceBusError::ConnectionFailed(format!(
                "Failed to create new ServiceBus client: {e}"
            ))
        })?;

        // Replace the client in the Arc<Mutex>
        {
            let mut client_guard = self.service_bus_client.lock().await;
            *client_guard = new_client;
        }

        // Update the consumer and producer managers with the new client
        {
            let mut consumer_manager = self.consumer_manager.lock().await;
            consumer_manager
                .reset_client(self.service_bus_client.clone())
                .await?;
        }

        {
            let mut producer_manager = self.producer_manager.lock().await;
            producer_manager
                .reset_client(self.service_bus_client.clone())
                .await?;
        }

        log::info!("ServiceBus connection reset successfully");
        Ok(ServiceBusResponse::ConnectionReset)
    }
}
