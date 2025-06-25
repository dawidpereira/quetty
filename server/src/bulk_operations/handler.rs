use super::collector::MessageCollector;
use super::deleter::MessageDeleter;
use super::sender::MessageSender;
use super::types::*;
use azservicebus::ServiceBusMessage;
use std::collections::HashMap;
use std::error::Error;

/// Main coordinator for bulk operations
pub struct BulkOperationHandler {
    config: BatchConfig,
    collector: MessageCollector,
    sender: MessageSender,
    deleter: MessageDeleter,
}

impl BulkOperationHandler {
    pub fn new(config: BatchConfig) -> Self {
        Self {
            collector: MessageCollector::new(),
            sender: MessageSender::new(config.clone()),
            deleter: MessageDeleter::new(config.clone()),
            config,
        }
    }

    /// Execute a bulk send operation
    pub async fn bulk_send(
        &self,
        context: BulkOperationContext,
        params: BulkSendParams,
    ) -> Result<BulkOperationResult, Box<dyn Error + Send + Sync>> {
        let total_requested = params.message_identifiers.len();
        let mut result = BulkOperationResult::new(total_requested);

        log::info!(
            "Starting bulk send operation: {} messages to queue '{}'",
            total_requested,
            params.target_queue
        );

        if total_requested == 0 {
            log::warn!("No messages provided for bulk send operation");
            return Ok(result);
        }

        // Choose execution path based on whether we have pre-fetched data
        if let Some(messages_data) = params.messages_data.clone() {
            self.execute_bulk_send_with_data(context, &params, messages_data, &mut result)
                .await?;
        } else {
            self.execute_bulk_send_with_retrieval(context, &params, &mut result)
                .await?;
        }

        log::info!(
            "Bulk send operation completed: {} successful, {} failed, {} not found out of {} requested",
            result.successful,
            result.failed,
            result.not_found,
            total_requested
        );

        Ok(result)
    }

    /// Execute bulk send with pre-fetched message data
    async fn execute_bulk_send_with_data(
        &self,
        context: BulkOperationContext,
        params: &BulkSendParams,
        messages_data: Vec<(MessageIdentifier, Vec<u8>)>,
        result: &mut BulkOperationResult,
    ) -> Result<BulkOperationResult, Box<dyn Error + Send + Sync>> {
        log::debug!(
            "Executing bulk send with {} pre-fetched messages",
            messages_data.len()
        );

        // Convert message data to ServiceBusMessage objects
        let service_bus_messages: Vec<ServiceBusMessage> = messages_data
            .iter()
            .map(|(_, data)| ServiceBusMessage::new(data.clone()))
            .collect();

        // Send messages using the sender
        self.sender.send_messages_to_queue(
            &params.target_queue,
            service_bus_messages,
            context.service_bus_client.clone(),
        ).await?;

        // Track all messages as successful since they were sent successfully
        for (identifier, _) in messages_data {
            result.add_successful_message(identifier);
        }

        log::info!(
            "Successfully sent {} messages using pre-fetched data",
            result.successful
        );

        Ok(result.clone())
    }

    /// Execute bulk send with message retrieval from source queue
    async fn execute_bulk_send_with_retrieval(
        &self,
        context: BulkOperationContext,
        params: &BulkSendParams,
        result: &mut BulkOperationResult,
    ) -> Result<BulkOperationResult, Box<dyn Error + Send + Sync>> {
        log::debug!(
            "Executing bulk send with retrieval for {} message identifiers",
            params.message_identifiers.len()
        );

        // Create a lookup map for target messages
        let target_map: HashMap<String, MessageIdentifier> = params
            .message_identifiers
            .iter()
            .map(|m| (m.id.clone(), m.clone()))
            .collect();

        // Calculate batch size with buffer
        let buffer_percentage = self.config.buffer_percentage();
        let min_buffer_size = self.config.min_buffer_size();
        let max_batch_size = self.config.max_batch_size() as usize;

        let buffer_size = std::cmp::max(
            (target_map.len() as f64 * buffer_percentage) as usize,
            min_buffer_size,
        );
        let batch_size = std::cmp::min(target_map.len() + buffer_size, max_batch_size);

        log::debug!(
            "Collecting messages: target_count={}, buffer_size={}, batch_size={}",
            target_map.len(),
            buffer_size,
            batch_size
        );

        // Collect target messages from the source queue
        let (target_messages, non_target_messages) = self
            .collector
            .collect_target_messages(context.consumer.clone(), &target_map, batch_size)
            .await?;

        if target_messages.is_empty() {
            log::warn!("No target messages found in queue");
            result.not_found = target_map.len();
            return Ok(result.clone());
        }

        // Process the collected target messages
        let process_params = ProcessTargetMessagesParams::new(
            target_messages,
            &context,
            params,
            &target_map,
            result,
        );

        self.sender.process_target_messages(process_params).await?;

        // Abandon non-target messages to make them available again
        if !non_target_messages.is_empty() {
            self.abandon_non_target_messages(
                context.consumer.clone(),
                non_target_messages,
                result,
            )
            .await?;
        }

        Ok(result.clone())
    }

    /// Execute bulk delete operation
    pub async fn bulk_delete(
        &self,
        context: BulkOperationContext,
        params: &BulkSendParams,
    ) -> Result<BulkOperationResult, Box<dyn Error + Send + Sync>> {
        self.deleter.bulk_delete(context, params).await
    }

    /// Abandon non-target messages that were collected but not needed
    async fn abandon_non_target_messages(
        &self,
        consumer: std::sync::Arc<tokio::sync::Mutex<crate::consumer::Consumer>>,
        non_target_messages: Vec<azservicebus::ServiceBusReceivedMessage>,
        _result: &mut BulkOperationResult,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        log::debug!("Abandoning {} non-target messages", non_target_messages.len());

        let mut consumer_guard = consumer.lock().await;
        consumer_guard
            .abandon_messages(&non_target_messages)
            .await
            .map_err(|e| -> Box<dyn Error + Send + Sync> {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to abandon non-target messages: {}", e),
                ))
            })?;

        log::debug!(
            "Successfully abandoned {} non-target messages",
            non_target_messages.len()
        );
        Ok(())
    }
} 