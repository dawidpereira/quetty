use super::resource_guard::acquire_lock_with_timeout;
use super::types::{
    BatchConfig, BulkOperationContext, BulkOperationResult, BulkSendParams, MessageIdentifier,
};
use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;

/// Handles bulk message deletion operations
pub struct MessageDeleter {
    config: BatchConfig,
}

/// Processing state for batch operations to reduce parameter count
struct BatchProcessingState {
    remaining_targets: HashMap<String, MessageIdentifier>,
    processed_message_ids: std::collections::HashSet<String>,
    total_processed: usize,
}

impl BatchProcessingState {
    fn new(target_map: HashMap<String, MessageIdentifier>) -> Self {
        Self {
            remaining_targets: target_map,
            processed_message_ids: std::collections::HashSet::new(),
            total_processed: 0,
        }
    }
}

impl MessageDeleter {
    pub fn new(config: BatchConfig) -> Self {
        Self { config }
    }

    /// Perform bulk delete operation on target messages
    pub async fn bulk_delete(
        &self,
        context: BulkOperationContext,
        params: &BulkSendParams,
    ) -> Result<BulkOperationResult, Box<dyn Error + Send + Sync>> {
        let total_requested = params.message_identifiers.len();
        let mut result = BulkOperationResult::new(total_requested);

        log::info!(
            "Starting bulk delete operation for {} messages",
            total_requested
        );

        if total_requested == 0 {
            log::warn!("No messages provided for bulk delete operation");
            return Ok(result);
        }

        // Create lookup map and process configuration
        let target_map: HashMap<String, MessageIdentifier> = params
            .message_identifiers
            .iter()
            .map(|m| (m.id.clone(), m.clone()))
            .collect();

        let batch_size = std::cmp::min(10, total_requested);
        let max_messages_to_process = (total_requested * self.config.max_messages_multiplier())
            .clamp(
                self.config.min_messages_to_process(),
                self.config.max_messages_to_process(),
            );

        log::info!(
            "Processing {} messages with batch size {} (max {} to process)",
            total_requested,
            batch_size,
            max_messages_to_process
        );

        // Main processing loop
        let mut state = BatchProcessingState::new(target_map.clone());
        let start_time = std::time::Instant::now();

        while !state.remaining_targets.is_empty()
            && state.total_processed < max_messages_to_process
            && start_time.elapsed().as_secs() < self.config.bulk_processing_time_secs()
        {
            if context.is_cancelled() {
                log::info!("Bulk delete operation cancelled");
                break;
            }

            match self
                .process_batch(&context, &target_map, &mut state, batch_size, &mut result)
                .await?
            {
                true => continue,
                false => break,
            }
        }

        result.not_found = state.remaining_targets.len();
        log::info!(
            "Bulk delete completed: {} successful, {} failed, {} not found",
            result.successful,
            result.failed,
            result.not_found
        );

        Ok(result)
    }

    async fn process_batch(
        &self,
        context: &BulkOperationContext,
        target_map: &HashMap<String, MessageIdentifier>,
        state: &mut BatchProcessingState,
        batch_size: usize,
        result: &mut BulkOperationResult,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        // Receive messages
        let received_messages = {
            let mut consumer_guard = acquire_lock_with_timeout(
                &context.consumer,
                "receive_messages",
                Duration::from_secs(self.config.lock_timeout_secs()),
                Some(&context.cancel_token),
            )
            .await?;

            consumer_guard
                .receive_messages(batch_size as u32)
                .await
                .map_err(|e| {
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to receive messages: {}", e),
                    )) as Box<dyn Error + Send + Sync>
                })?
        };

        if received_messages.is_empty() {
            return Ok(false);
        }

        // Separate target and non-target messages
        let mut target_messages = Vec::new();
        let mut non_target_messages = Vec::new();

        for message in received_messages {
            let message_id = message
                .message_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            if state.processed_message_ids.contains(&message_id) {
                continue;
            }

            state.processed_message_ids.insert(message_id.clone());
            state.total_processed += 1;

            if state.remaining_targets.contains_key(&message_id) {
                state.remaining_targets.remove(&message_id);
                target_messages.push(message);
            } else {
                non_target_messages.push(message);
            }
        }

        // Complete target messages
        if !target_messages.is_empty() {
            match self.complete_messages(&target_messages, context).await {
                Ok(()) => {
                    self.track_successful_deletions(&target_messages, target_map, result);
                }
                Err(e) => {
                    result.add_failure(format!("Failed to delete messages: {}", e));
                }
            }
        }

        // Abandon non-target messages
        if !non_target_messages.is_empty() {
            let _ = self.abandon_messages(&non_target_messages, context).await;
        }

        Ok(true)
    }

    async fn complete_messages(
        &self,
        messages: &[azservicebus::ServiceBusReceivedMessage],
        context: &BulkOperationContext,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut consumer_guard = context.consumer.lock().await;
        for message in messages {
            consumer_guard.complete_message(message).await.map_err(
                |e| -> Box<dyn Error + Send + Sync> {
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to complete message: {}", e),
                    ))
                },
            )?;
        }
        Ok(())
    }

    async fn abandon_messages(
        &self,
        messages: &[azservicebus::ServiceBusReceivedMessage],
        context: &BulkOperationContext,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut consumer_guard = context.consumer.lock().await;
        consumer_guard.abandon_messages(messages).await.map_err(
            |e| -> Box<dyn Error + Send + Sync> {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to abandon messages: {}", e),
                ))
            },
        )?;
        Ok(())
    }

    fn track_successful_deletions(
        &self,
        messages: &[azservicebus::ServiceBusReceivedMessage],
        target_map: &HashMap<String, MessageIdentifier>,
        result: &mut BulkOperationResult,
    ) {
        for message in messages {
            if let Some(message_id) = message.message_id() {
                if let Some(target_id) = target_map.get(&*message_id) {
                    result.add_successful_message(target_id.clone());
                }
            }
        }
    }
}

