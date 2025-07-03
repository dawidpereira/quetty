use crate::bulk_operations::resource_guard::acquire_lock_with_timeout;
use crate::bulk_operations::types::{
    BatchConfig, BulkOperationContext, BulkOperationResult, BulkSendParams, MessageIdentifier,
};
use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;
use tokio::time::interval;

/// Parameters for batch iteration processing
#[derive(Debug)]
struct BatchIterationParams<'a> {
    context: &'a BulkOperationContext,
    batch_size: usize,
    target_map: &'a HashMap<String, MessageIdentifier>,
    pending_messages: &'a mut Vec<azservicebus::ServiceBusReceivedMessage>,
    processed_count: &'a mut usize,
    found_targets: &'a mut usize,
    highest_sequence_seen: &'a mut i64,
    result: &'a mut BulkOperationResult,
}

/// Simple batch-based message deleter
pub struct BulkDeleter {
    config: BatchConfig,
}

impl BulkDeleter {
    pub fn new(config: BatchConfig) -> Self {
        Self { config }
    }

    pub async fn delete_messages(
        &self,
        context: BulkOperationContext,
        params: BulkSendParams,
    ) -> Result<BulkOperationResult, Box<dyn Error + Send + Sync>> {
        let targets = params.message_identifiers;
        let mut result = BulkOperationResult::new(targets.len());

        if targets.is_empty() {
            return Ok(result);
        }

        log::info!(
            "Starting batch-based bulk delete for {} messages",
            targets.len()
        );

        // Use max_position for small batch logic only
        let max_index = params.max_position;

        log::info!("Maximum target index: {}", max_index);

        // Check if position is too high
        let max_allowed_index = self.config.max_messages_to_process();
        if max_index > max_allowed_index {
            let error_msg = format!(
                "Index {} is too high. Maximum allowed index is {}.",
                max_index, max_allowed_index
            );
            log::error!("{}", error_msg);
            result.add_failure(error_msg);
            return Ok(result);
        }

        // Execute deletion based on position range
        let small_batch_threshold = self.config.max_batch_size() as usize;
        if max_index <= small_batch_threshold {
            self.delete_small_batch(&context, targets, max_index, &mut result)
                .await?;
        } else {
            self.delete_large_batch(&context, targets, &mut result)
                .await?;
        }

        log::info!(
            "Batch deletion completed: {} successful, {} failed, {} not found",
            result.successful,
            result.failed,
            result.not_found
        );

        Ok(result)
    }

    /// Handle small batches (position <= max_batch_size): single batch with size = position
    async fn delete_small_batch(
        &self,
        context: &BulkOperationContext,
        targets: Vec<MessageIdentifier>,
        batch_size: usize,
        result: &mut BulkOperationResult,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        log::info!(
            "Small batch mode: fetching {} messages in single batch",
            batch_size
        );

        let target_map: HashMap<String, MessageIdentifier> = targets
            .into_iter()
            .map(|target| (target.id.clone(), target))
            .collect();

        let messages = self.receive_messages(context, batch_size).await?;

        if messages.is_empty() {
            log::info!(" No messages available - breaking");
            for _target in target_map.values() {
                result.add_not_found();
            }
            return Ok(());
        }

        self.process_messages(context, messages, &target_map, result)
            .await?;
        Ok(())
    }

    /// Handle large batches (position > max_batch_size): scan in batches with lock management
    async fn delete_large_batch(
        &self,
        context: &BulkOperationContext,
        targets: Vec<MessageIdentifier>,
        result: &mut BulkOperationResult,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let batch_size = self.config.bulk_chunk_size();
        log::info!(
            "Large batch mode: scanning in batches of {} using sequence-based stopping",
            batch_size
        );

        // Setup operation state
        let (target_map, target_max_sequence, lock_refresh_handle) =
            self.setup_large_batch_operation(context, targets).await;

        let mut pending_messages = Vec::new();
        let mut processed_count = 0;
        let mut found_targets = 0;
        let mut highest_sequence_seen = 0i64;

        // Main processing loop
        while self.should_continue_scanning(
            found_targets,
            target_map.len(),
            target_max_sequence,
            highest_sequence_seen,
        ) {
            // Log progress less frequently for large operations
            if processed_count % (batch_size * 5) == 0 {
                self.log_bulk_delete_progress(
                    processed_count,
                    highest_sequence_seen,
                    target_max_sequence,
                    found_targets,
                    target_map.len(),
                    pending_messages.len(),
                );
            }

            // Process a single batch iteration
            let batch_params = BatchIterationParams {
                context,
                batch_size,
                target_map: &target_map,
                pending_messages: &mut pending_messages,
                processed_count: &mut processed_count,
                found_targets: &mut found_targets,
                highest_sequence_seen: &mut highest_sequence_seen,
                result,
            };
            let batch_result = self.process_batch_iteration(batch_params).await?;

            if !batch_result {
                break; // No more messages available
            }

            // Check if we found all targets
            if found_targets >= target_map.len() {
                log::info!("All {} targets found and processed", target_map.len());
                break;
            }
        }

        // Cleanup and finalization
        self.cleanup_pending_messages(context, pending_messages)
            .await;
        lock_refresh_handle.abort();

        log::info!(
            "Large batch deletion completed: processed {} messages, found {} targets",
            processed_count,
            found_targets
        );

        Ok(())
    }

    /// Setup the large batch operation with target map, sequence tracking, and lock refresh
    async fn setup_large_batch_operation(
        &self,
        context: &BulkOperationContext,
        targets: Vec<MessageIdentifier>,
    ) -> (
        HashMap<String, MessageIdentifier>,
        i64,
        tokio::task::JoinHandle<()>,
    ) {
        // Calculate the target max sequence number from the message IDs first
        let target_max_sequence = targets
            .iter()
            .map(|msg_id| msg_id.sequence)
            .max()
            .unwrap_or(0);

        let target_map: HashMap<String, MessageIdentifier> = targets
            .into_iter()
            .map(|target| (target.id.clone(), target))
            .collect();

        log::info!("Target max sequence number: {}", target_max_sequence);

        // Start lock refresh task
        let lock_refresh_handle = self.start_lock_refresh_task(context, &[]).await;

        (target_map, target_max_sequence, lock_refresh_handle)
    }

    /// Determine if we should continue scanning based on current state
    fn should_continue_scanning(
        &self,
        found_targets: usize,
        total_targets: usize,
        target_max_sequence: i64,
        highest_sequence_seen: i64,
    ) -> bool {
        found_targets < total_targets
            && (target_max_sequence == 0 || highest_sequence_seen < target_max_sequence)
    }

    /// Log progress for bulk delete operations
    fn log_bulk_delete_progress(
        &self,
        processed_count: usize,
        highest_sequence_seen: i64,
        target_max_sequence: i64,
        found_targets: usize,
        total_targets: usize,
        pending_messages: usize,
    ) {
        log::info!(
            "Bulk delete progress: processed: {} | highest_sequence: {} / target: {} | found_targets: {} / {} | pending_locked: {}",
            processed_count,
            highest_sequence_seen,
            target_max_sequence,
            found_targets,
            total_targets,
            pending_messages
        );
    }

    /// Process a single batch iteration
    async fn process_batch_iteration(
        &self,
        params: BatchIterationParams<'_>,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        log::debug!(
            "Requesting next batch: {} messages (will complete at {})",
            params.batch_size,
            *params.processed_count + params.batch_size
        );

        let messages = self
            .receive_messages(params.context, params.batch_size)
            .await?;

        if messages.is_empty() {
            log::warn!(
                "Received empty batch from Service Bus before reaching target sequence. processed: {} found_targets: {} pending: {}",
                *params.processed_count,
                *params.found_targets,
                params.pending_messages.len()
            );
            return Ok(false);
        }

        // Track highest sequence number seen in this batch
        for msg in &messages {
            let msg_sequence = msg.sequence_number();
            if msg_sequence > *params.highest_sequence_seen {
                *params.highest_sequence_seen = msg_sequence;
            }
        }

        // Check current batch for targets
        let (targets_in_batch, non_targets): (Vec<_>, Vec<_>) =
            messages.into_iter().partition(|msg| {
                if let Some(msg_id) = msg.message_id() {
                    params.target_map.contains_key(msg_id.as_ref())
                } else {
                    false
                }
            });

        // Process target messages immediately
        if !targets_in_batch.is_empty() {
            self.process_target_messages(
                params.context,
                targets_in_batch,
                params.target_map,
                params.found_targets,
                params.result,
            )
            .await?;
        }

        // Add non-targets to pending list for lock management (abandon them once at the end)
        params.pending_messages.extend(non_targets);
        *params.processed_count += params.batch_size;

        // Safety check: if we've gone well beyond the target sequence and still have targets
        let target_max_sequence = params
            .target_map
            .values()
            .map(|msg_id| msg_id.sequence)
            .max()
            .unwrap_or(0);
        if target_max_sequence > 0
            && *params.highest_sequence_seen > target_max_sequence + 1000
            && *params.found_targets < params.target_map.len()
        {
            log::warn!(
                "Safety break: highest_sequence {} exceeds target {} by 1000+, {} targets still remaining",
                *params.highest_sequence_seen,
                target_max_sequence,
                *params.found_targets
            );
            return Ok(false);
        }

        log::debug!(
            "End of iteration: processed_count={} | highest_sequence={} | found_targets={} | pending_locked={}",
            *params.processed_count,
            *params.highest_sequence_seen,
            *params.found_targets,
            params.pending_messages.len()
        );

        Ok(true)
    }

    /// Process target messages found in a batch
    async fn process_target_messages(
        &self,
        context: &BulkOperationContext,
        targets_in_batch: Vec<azservicebus::ServiceBusReceivedMessage>,
        target_map: &HashMap<String, MessageIdentifier>,
        found_targets: &mut usize,
        result: &mut BulkOperationResult,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        log::info!(
            "Found {} target messages in current batch",
            targets_in_batch.len()
        );

        for message in targets_in_batch {
            if let Some(msg_id) = message.message_id() {
                if let Some(target) = target_map.get(msg_id.as_ref()) {
                    match self.complete_message(context, &message).await {
                        Ok(_) => {
                            result.add_successful_message(target.clone());
                            log::info!(
                                "Deleted target {} ({}/{})",
                                target.id,
                                *found_targets,
                                target_map.len()
                            );
                            *found_targets += 1;
                        }
                        Err(e) => {
                            log::error!("Failed to delete target {}: {}", target.id, e);
                            result.add_failure(format!("Failed to delete {}: {}", target.id, e));
                            // Abandon the message
                            if let Err(abandon_err) = self.abandon_message(context, &message).await
                            {
                                log::warn!(
                                    "Failed to abandon message after delete failure: {}",
                                    abandon_err
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Clean up pending messages by abandoning them
    async fn cleanup_pending_messages(
        &self,
        context: &BulkOperationContext,
        pending_messages: Vec<azservicebus::ServiceBusReceivedMessage>,
    ) {
        if !pending_messages.is_empty() {
            log::info!(
                "Abandoning {} remaining non-target messages",
                pending_messages.len()
            );
            for message in pending_messages {
                if let Err(e) = self.abandon_message(context, &message).await {
                    log::warn!("Failed to abandon non-target message: {}", e);
                }
            }
        }
    }

    /// Start background task to refresh locks on pending messages every 30 seconds
    async fn start_lock_refresh_task(
        &self,
        context: &BulkOperationContext,
        _pending_messages: &[azservicebus::ServiceBusReceivedMessage],
    ) -> tokio::task::JoinHandle<()> {
        let context_clone = BulkOperationContext {
            consumer: context.consumer.clone(),
            cancel_token: context.cancel_token.clone(),
            queue_name: context.queue_name.clone(),
        };

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Check if operation was cancelled
                if context_clone.cancel_token.is_cancelled() {
                    log::info!("Lock refresh task cancelled");
                    break;
                }

                log::debug!("Lock refresh tick (background task running)");
                // Note: In a real implementation, we would refresh locks on pending messages here
                // For now, we just log that the task is running
            }
        })
    }

    /// Process a batch of messages, deleting targets and abandoning non-targets
    async fn process_messages(
        &self,
        context: &BulkOperationContext,
        messages: Vec<azservicebus::ServiceBusReceivedMessage>,
        target_map: &HashMap<String, MessageIdentifier>,
        result: &mut BulkOperationResult,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        log::info!("Processing {} messages", messages.len());

        for message in messages {
            let message_id = message.message_id();

            if let Some(msg_id) = message_id {
                if let Some(target) = target_map.get(msg_id.as_ref()) {
                    // This is a target message - delete it
                    match self.complete_message(context, &message).await {
                        Ok(_) => {
                            result.add_successful_message(target.clone());
                            log::info!("Deleted target {}", target.id);
                        }
                        Err(e) => {
                            log::error!("Failed to delete target {}: {}", target.id, e);
                            result.add_failure(format!("Failed to delete {}: {}", target.id, e));
                            // Abandon the message
                            if let Err(abandon_err) = self.abandon_message(context, &message).await
                            {
                                log::warn!(
                                    "Failed to abandon message after delete failure: {}",
                                    abandon_err
                                );
                            }
                        }
                    }
                } else {
                    // Not a target - abandon it (this is small batch mode, so immediate abandon is OK)
                    if let Err(e) = self.abandon_message(context, &message).await {
                        log::warn!("Failed to abandon non-target message {:?}: {}", msg_id, e);
                    }
                }
            } else {
                // Message has no ID - abandon it
                if let Err(e) = self.abandon_message(context, &message).await {
                    log::warn!("Failed to abandon message with no ID: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Receive a batch of messages
    async fn receive_messages(
        &self,
        context: &BulkOperationContext,
        count: usize,
    ) -> Result<Vec<azservicebus::ServiceBusReceivedMessage>, Box<dyn Error + Send + Sync>> {
        let mut consumer = acquire_lock_with_timeout(
            &context.consumer,
            "receive_messages",
            Duration::from_secs(self.config.lock_timeout_secs()),
            Some(&context.cancel_token),
        )
        .await?;

        log::debug!("Receiving up to {} messages", count);

        match consumer
            .receive_messages_with_timeout(
                count as u32,
                Duration::from_secs(self.config.bulk_processing_time_secs()),
            )
            .await
        {
            Ok(messages) => {
                log::debug!("Received {} messages", messages.len());
                Ok(messages)
            }
            Err(e) => Err(format!("Failed to receive messages: {}", e).into()),
        }
    }

    /// Complete (delete) a message
    async fn complete_message(
        &self,
        context: &BulkOperationContext,
        message: &azservicebus::ServiceBusReceivedMessage,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut consumer = acquire_lock_with_timeout(
            &context.consumer,
            "complete_message",
            Duration::from_secs(self.config.lock_timeout_secs()),
            Some(&context.cancel_token),
        )
        .await?;

        consumer
            .complete_message(message)
            .await
            .map_err(|e| format!("Failed to complete message: {}", e).into())
    }

    /// Abandon a message (put it back in the queue)
    async fn abandon_message(
        &self,
        context: &BulkOperationContext,
        message: &azservicebus::ServiceBusReceivedMessage,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut consumer = acquire_lock_with_timeout(
            &context.consumer,
            "abandon_message",
            Duration::from_secs(self.config.lock_timeout_secs()),
            Some(&context.cancel_token),
        )
        .await?;

        consumer
            .abandon_message(message)
            .await
            .map_err(|e| format!("Failed to abandon message: {}", e).into())
    }
}

impl Default for BulkDeleter {
    fn default() -> Self {
        Self::new(BatchConfig::default())
    }
}

/// High-level message deleter interface
pub struct MessageDeleter {
    deleter: BulkDeleter,
}

impl MessageDeleter {
    pub fn new(config: super::types::BatchConfig) -> Self {
        Self {
            deleter: BulkDeleter::new(config),
        }
    }

    pub async fn execute(
        &self,
        context: BulkOperationContext,
        params: BulkSendParams,
    ) -> Result<BulkOperationResult, Box<dyn Error + Send + Sync>> {
        self.deleter.delete_messages(context, params).await
    }

    pub async fn delete_messages(
        &self,
        context: BulkOperationContext,
        params: BulkSendParams,
    ) -> Result<BulkOperationResult, Box<dyn Error + Send + Sync>> {
        self.deleter.delete_messages(context, params).await
    }
}
