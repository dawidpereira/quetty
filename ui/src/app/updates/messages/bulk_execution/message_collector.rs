use crate::error::AppError;
use azservicebus::ServiceBusReceivedMessage;
use server::bulk_operations::MessageIdentifier;
use std::collections::{HashMap, HashSet};
use std::error::Error;

/// Context for batch delete operations
pub struct BatchDeleteContext {
    pub target_map: HashMap<String, MessageIdentifier>,
    pub collection_batch_size: usize,
}

impl BatchDeleteContext {
    pub fn new(message_ids: &[MessageIdentifier], batch_size: usize) -> Result<Self, AppError> {
        if message_ids.is_empty() {
            return Err(AppError::State(
                "No message IDs provided for batch delete context".to_string(),
            ));
        }

        let target_map: HashMap<String, MessageIdentifier> = message_ids
            .iter()
            .map(|msg_id| (msg_id.id.clone(), msg_id.clone()))
            .collect();

        // Validate no duplicate message IDs
        if target_map.len() != message_ids.len() {
            log::warn!(
                "Duplicate message IDs detected: {} unique out of {} total",
                target_map.len(),
                message_ids.len()
            );
        }

        log::info!(
            "Created batch delete context for {} unique messages with batch size {}",
            target_map.len(),
            batch_size
        );

        Ok(BatchDeleteContext {
            target_map,
            collection_batch_size: batch_size,
        })
    }

    pub fn target_count(&self) -> usize {
        self.target_map.len()
    }
}

/// State management for message collection during bulk delete operations
pub struct MessageCollector {
    target_map: HashMap<String, MessageIdentifier>,
    found_target_ids: HashSet<String>,
    collected_target: Vec<ServiceBusReceivedMessage>,
    collected_non_target: Vec<ServiceBusReceivedMessage>,
    total_processed: usize,
    consecutive_empty_batches: usize,
    max_empty_batches: usize,
    batch_size: u32,
}

impl MessageCollector {
    pub fn new(context: &BatchDeleteContext) -> Self {
        // Use the configured max attempts for DLQ operations instead of hardcoded value
        let max_empty_batches = crate::config::CONFIG.dlq().max_attempts();
        Self {
            target_map: context.target_map.clone(),
            found_target_ids: HashSet::new(),
            collected_target: Vec::new(),
            collected_non_target: Vec::new(),
            total_processed: 0,
            consecutive_empty_batches: 0,
            max_empty_batches,
            batch_size: context.collection_batch_size as u32,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.found_target_ids.len() >= self.target_map.len()
    }

    pub fn should_stop(&self) -> bool {
        self.consecutive_empty_batches >= self.max_empty_batches
    }

    pub fn target_count(&self) -> usize {
        self.target_map.len()
    }

    pub fn batch_size(&self) -> u32 {
        self.batch_size
    }

    pub fn process_received_messages(&mut self, messages: Vec<ServiceBusReceivedMessage>) -> bool {
        if messages.is_empty() {
            self.consecutive_empty_batches += 1;
            log::debug!(
                "Empty batch #{} - {} messages processed so far, found {}/{} targets",
                self.consecutive_empty_batches,
                self.total_processed,
                self.found_target_ids.len(),
                self.target_map.len()
            );
            return false;
        }

        self.consecutive_empty_batches = 0;

        for message in messages {
            self.total_processed += 1;
            let message_id = message
                .message_id()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            if self.target_map.contains_key(&message_id)
                && !self.found_target_ids.contains(&message_id)
            {
                log::debug!(
                    "Found target message: {} (sequence: {})",
                    message_id,
                    message.sequence_number()
                );
                self.found_target_ids.insert(message_id.clone());
                self.collected_target.push(message);

                if self.is_complete() {
                    log::info!(
                        "Found all {} target messages after processing {} total messages",
                        self.target_map.len(),
                        self.total_processed
                    );
                    return true; // Signal completion
                }
            } else {
                self.collected_non_target.push(message);
            }
        }

        false
    }

    pub fn handle_receive_error(&mut self, error: &dyn Error) {
        self.consecutive_empty_batches += 1;
        log::debug!(
            "Error receiving batch #{}: {} - {} messages processed so far",
            self.consecutive_empty_batches,
            error,
            self.total_processed
        );
    }

    pub fn finalize(
        self,
    ) -> (
        Vec<ServiceBusReceivedMessage>,
        Vec<ServiceBusReceivedMessage>,
    ) {
        let not_found_count = self.target_map.len() - self.found_target_ids.len();
        log::info!(
            "Collection phase complete: {} target messages found, {} not found, {} non-target messages collected, {} messages processed total",
            self.collected_target.len(),
            not_found_count,
            self.collected_non_target.len(),
            self.total_processed
        );

        if not_found_count > 0 {
            let missing_ids: Vec<&String> = self
                .target_map
                .keys()
                .filter(|id| !self.found_target_ids.contains(*id))
                .collect();

            log::warn!(
                "Missing {} target messages: {:?}",
                not_found_count,
                missing_ids
            );
        }

        (self.collected_target, self.collected_non_target)
    }
}
