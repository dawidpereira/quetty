use super::types::{BatchProcessingContext, MessageIdentifier};
use crate::consumer::Consumer;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Handles message collection operations from Service Bus queues
pub struct MessageCollector;

impl MessageCollector {
    pub fn new() -> Self {
        Self
    }

    /// Collect target messages from the queue, separating them from non-target messages
    pub async fn collect_target_messages(
        &self,
        consumer: Arc<Mutex<Consumer>>,
        target_map: &HashMap<String, MessageIdentifier>,
        batch_size: usize,
    ) -> Result<
        (
            Vec<azservicebus::ServiceBusReceivedMessage>,
            Vec<azservicebus::ServiceBusReceivedMessage>,
        ),
        Box<dyn Error + Send + Sync>,
    > {
        let target_count = target_map.len();
        self.log_collection_start(target_count, batch_size);

        let max_messages_to_process = target_count * 3;
        let mut target_messages = Vec::new();
        let mut non_target_messages = Vec::new();
        let mut remaining_targets = target_map.clone();
        let mut messages_processed = 0;

        while !remaining_targets.is_empty() && messages_processed < max_messages_to_process {
            let target_messages_found = target_messages.len();

            let ctx = BatchProcessingContext {
                consumer: consumer.clone(),
                batch_size,
                target_messages_found,
                target_map,
                messages_processed,
                remaining_targets: &mut remaining_targets,
                target_messages_vec: &mut target_messages,
                non_target_messages: &mut non_target_messages,
            };

            match self.process_single_batch(ctx).await? {
                Some(new_messages) => {
                    messages_processed += new_messages;
                }
                None => {
                    self.log_no_more_messages(messages_processed, target_messages.len(), target_count);
                    break;
                }
            }
        }

        self.log_collection_complete(
            &target_messages,
            &non_target_messages,
            messages_processed,
            &remaining_targets,
        );

        Ok((target_messages, non_target_messages))
    }

    fn log_collection_start(&self, target_count: usize, batch_size: usize) {
        log::info!(
            "Starting message collection: {} targets, batch size: {}",
            target_count,
            batch_size
        );
    }

    fn log_no_more_messages(
        &self,
        messages_processed: usize,
        targets_found: usize,
        total_targets: usize,
    ) {
        log::info!(
            "No more messages available. Processed: {}, Found targets: {}/{} ({:.1}%)",
            messages_processed,
            targets_found,
            total_targets,
            if total_targets > 0 {
                (targets_found as f64 / total_targets as f64) * 100.0
            } else {
                0.0
            }
        );
    }

    fn log_collection_complete(
        &self,
        target_messages: &[azservicebus::ServiceBusReceivedMessage],
        non_target_messages: &[azservicebus::ServiceBusReceivedMessage],
        messages_processed: usize,
        remaining_targets: &HashMap<String, MessageIdentifier>,
    ) {
        log::info!(
            "Collection complete. Target messages: {}, Non-target: {}, Total processed: {}, Remaining targets: {}",
            target_messages.len(),
            non_target_messages.len(),
            messages_processed,
            remaining_targets.len()
        );

        if !remaining_targets.is_empty() {
            log::warn!(
                "Could not find {} target messages after processing {} messages",
                remaining_targets.len(),
                messages_processed
            );
        }
    }

    async fn process_single_batch(
        &self,
        ctx: BatchProcessingContext<'_>,
    ) -> Result<Option<usize>, Box<dyn Error + Send + Sync>> {
        let batch_messages = self
            .receive_message_batch(
                ctx.consumer.clone(),
                ctx.batch_size,
                ctx.target_messages_found,
                ctx.target_map,
                ctx.messages_processed,
            )
            .await?;

        if let Some(received_messages) = batch_messages {
            let batch_size = received_messages.len();
            let new_targets_found = self.process_message_batch(
                received_messages,
                ctx.remaining_targets,
                ctx.target_messages_vec,
                ctx.non_target_messages,
            );

            log::debug!(
                "Batch processed: {} messages, {} new targets found",
                batch_size,
                new_targets_found
            );

            Ok(Some(batch_size))
        } else {
            Ok(None)
        }
    }

    async fn receive_message_batch(
        &self,
        consumer: Arc<Mutex<Consumer>>,
        batch_size: usize,
        target_messages_found: usize,
        target_map: &HashMap<String, MessageIdentifier>,
        messages_processed: usize,
    ) -> Result<Option<Vec<azservicebus::ServiceBusReceivedMessage>>, Box<dyn Error + Send + Sync>>
    {
        let max_count = std::cmp::min(
            batch_size.saturating_sub(target_messages_found),
            target_map.len() * 2,
        );

        if max_count == 0 {
            log::debug!("Skipping batch: calculated max_count is 0");
            return Ok(None);
        }

        log::debug!(
            "Receiving batch: max_count={}, processed={}, targets_found={}",
            max_count,
            messages_processed,
            target_messages_found
        );

        let mut consumer_guard = consumer.lock().await;
        let received_messages = consumer_guard
            .receive_messages(max_count.try_into().unwrap_or(1))
            .await
            .map_err(|e| -> Box<dyn Error + Send + Sync> {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to receive messages: {}", e),
                ))
            })?;

        if received_messages.is_empty() {
            log::debug!("No messages received in batch");
            return Ok(None);
        }

        Ok(Some(received_messages))
    }

    fn process_message_batch(
        &self,
        received_messages: Vec<azservicebus::ServiceBusReceivedMessage>,
        remaining_targets: &mut HashMap<String, MessageIdentifier>,
        target_messages: &mut Vec<azservicebus::ServiceBusReceivedMessage>,
        non_target_messages: &mut Vec<azservicebus::ServiceBusReceivedMessage>,
    ) -> usize {
        let mut new_targets_found = 0;

        for message in received_messages {
            if let Some(message_id) = message.message_id() {
                if remaining_targets.contains_key(&*message_id) {
                    remaining_targets.remove(&*message_id);
                    target_messages.push(message);
                    new_targets_found += 1;
                } else {
                    non_target_messages.push(message);
                }
            } else {
                non_target_messages.push(message);
            }
        }

        new_targets_found
    }
} 