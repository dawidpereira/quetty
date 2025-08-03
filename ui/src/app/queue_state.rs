use crate::app::managers::queue_stats_manager::QueueStatsManager;
use crate::app::updates::messages::MessagePaginationState;
use quetty_server::bulk_operations::MessageIdentifier;
use quetty_server::model::MessageModel;
use quetty_server::service_bus_manager::QueueType;
use std::collections::HashSet;

/// Unique identifier for a message combining ID and sequence
/// State for managing bulk selection of messages
#[derive(Debug, Default)]
pub struct BulkSelectionState {
    /// Set of selected message identifiers
    pub selected_messages: HashSet<MessageIdentifier>,
    /// Whether we're currently in bulk selection mode
    pub selection_mode: bool,
    /// Index of the last selected message for range selection and max_position calculation
    pub last_selected_index: Option<usize>,
    /// Set of all selected indices (for tracking highest index across pages)
    pub selected_indices: HashSet<usize>,
}

impl BulkSelectionState {
    /// Toggle selection for a message
    pub fn toggle_selection(&mut self, message_id: MessageIdentifier, index: usize) -> bool {
        if self.selected_messages.contains(&message_id) {
            self.selected_messages.remove(&message_id);
            self.selected_indices.remove(&index);
            self.last_selected_index = self.selected_indices.iter().max().copied();
            false
        } else {
            self.selected_messages.insert(message_id);
            self.selected_indices.insert(index);
            self.last_selected_index = self.selected_indices.iter().max().copied();
            true
        }
    }

    /// Select all messages from a given list
    pub fn select_all(&mut self, messages: &[MessageModel]) {
        for (index, message) in messages.iter().enumerate() {
            self.selected_messages
                .insert(MessageIdentifier::from_message(message));
            self.selected_indices.insert(index);
        }
        // Set last_selected_index to the highest index
        self.last_selected_index = self.selected_indices.iter().max().copied();
        if !messages.is_empty() {
            self.selection_mode = true;
        }
    }

    /// Clear all selections
    pub fn clear_all(&mut self) {
        self.selected_messages.clear();
        self.selected_indices.clear();
        self.last_selected_index = None;
        self.selection_mode = false;
    }

    /// Get the number of selected messages
    pub fn selection_count(&self) -> usize {
        self.selected_messages.len()
    }

    /// Check if any messages are selected
    pub fn has_selections(&self) -> bool {
        !self.selected_messages.is_empty()
    }

    /// Enter bulk selection mode
    pub fn enter_selection_mode(&mut self) {
        self.selection_mode = true;
    }

    /// Exit bulk selection mode and clear selections
    pub fn exit_selection_mode(&mut self) {
        self.clear_all();
    }

    /// Get a vector of selected message identifiers
    pub fn get_selected_messages(&self) -> Vec<MessageIdentifier> {
        self.selected_messages.iter().cloned().collect()
    }

    /// Get the highest selected position (1-based) for max_position calculation
    pub fn get_highest_selected_position(&self) -> Option<usize> {
        self.selected_indices.iter().max().map(|&index| index + 1)
    }

    /// Check if selected messages are contiguous from the beginning (index 0)
    /// This helps determine if we need to show delivery count warnings
    pub fn are_selections_contiguous_from_start(&self) -> bool {
        if self.selected_indices.is_empty() {
            return true; // No selections means no gaps
        }

        // Get the minimum and maximum selected indices
        let min_index = self.selected_indices.iter().min().unwrap_or(&0);
        let max_index = self.selected_indices.iter().max().unwrap_or(&0);

        // If we don't start from index 0, there are gaps
        if *min_index != 0 {
            return false;
        }

        // Check if all indices from 0 to max_index are selected
        for i in 0..=*max_index {
            if !self.selected_indices.contains(&i) {
                return false; // Found a gap
            }
        }

        true // All indices from 0 to max are selected (contiguous)
    }

    /// Remove messages from selection (used when messages are deleted/moved)
    pub fn remove_messages(&mut self, message_ids: &[MessageIdentifier]) {
        for id in message_ids {
            self.selected_messages.remove(id);
        }
        // Note: We can't easily remove specific indices here since we don't have the mapping
        // The indices will be cleared when selections are cleared or when messages are reloaded
        // Exit selection mode if no messages are selected
        if self.selected_messages.is_empty() {
            self.selection_mode = false;
            self.last_selected_index = None;
            self.selected_indices.clear();
        }
    }

    /// Select all messages from a given list using a starting global index offset.
    /// This ensures that selected_indices reflect the absolute message positions across pages.
    pub fn select_all_with_offset(&mut self, messages: &[MessageModel], start_index_offset: usize) {
        for (local_idx, message) in messages.iter().enumerate() {
            let global_idx = start_index_offset + local_idx;
            self.selected_messages
                .insert(MessageIdentifier::from_message(message));
            self.selected_indices.insert(global_idx);
        }
        // Set last_selected_index to the highest index across updated selections
        self.last_selected_index = self.selected_indices.iter().max().copied();
        if !messages.is_empty() {
            self.selection_mode = true;
        }
    }

    /// Calculate the sum of gaps between selected message indices.
    /// This is used to validate against Azure Service Bus link credit limit.
    /// Returns the total number of non-selected messages between the first and last selected messages.
    pub fn calculate_gap_sum(&self) -> usize {
        if self.selected_indices.len() <= 1 {
            return 0;
        }

        // Get sorted indices
        let mut sorted_indices: Vec<usize> = self.selected_indices.iter().copied().collect();
        sorted_indices.sort_unstable();

        // Calculate the total range span
        let min_index = sorted_indices.first().copied().unwrap_or(0);
        let max_index = sorted_indices.last().copied().unwrap_or(0);

        // Total gap = (max - min + 1) - number of selected messages
        // This gives us the count of non-selected messages within the range
        let total_span = max_index - min_index + 1;
        let gap_sum = total_span.saturating_sub(sorted_indices.len());

        log::debug!(
            "Gap calculation: min_index={}, max_index={}, total_span={}, selected_count={}, gap_sum={}",
            min_index,
            max_index,
            total_span,
            sorted_indices.len(),
            gap_sum
        );

        gap_sum
    }
}

/// Encapsulates all queue-related state and data
#[derive(Debug)]
pub struct QueueState {
    /// Queue name that is pending selection (before consumer is created)
    pub pending_queue: Option<String>,
    /// Currently selected queue name
    pub current_queue_name: Option<String>,
    /// Current queue type (Main or DeadLetter)
    pub current_queue_type: QueueType,
    /// Currently loaded messages
    pub messages: Option<Vec<MessageModel>>,
    /// Message pagination state
    pub message_pagination: MessagePaginationState,
    /// Queue statistics manager
    pub stats_manager: QueueStatsManager,
    /// Bulk selection state
    pub bulk_selection: BulkSelectionState,
    /// Message repeat count for bulk sending (1-1000)
    pub message_repeat_count: usize,
}

impl Default for QueueState {
    fn default() -> Self {
        Self {
            pending_queue: None,
            current_queue_name: None,
            current_queue_type: QueueType::Main,
            messages: None,
            message_pagination: MessagePaginationState::default(),
            stats_manager: QueueStatsManager::new(),
            bulk_selection: BulkSelectionState::default(),
            message_repeat_count: 1, // Default to sending once
        }
    }
}

impl QueueState {
    /// Create a new QueueState with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the selected queue and reset related state
    pub fn set_selected_queue(&mut self, queue_name: String) {
        self.pending_queue = Some(queue_name.clone());
        self.current_queue_name = Some(queue_name);
        self.current_queue_type = QueueType::Main;
        // Clear previous messages and pagination when switching queues
        self.messages = None;
        self.message_pagination.reset();
    }

    /// Toggle between main queue and dead letter queue
    pub fn toggle_queue_type(&mut self) -> Option<String> {
        if let Some(current_queue_name) = &self.current_queue_name {
            let new_queue_type = match self.current_queue_type {
                QueueType::Main => QueueType::DeadLetter,
                QueueType::DeadLetter => QueueType::Main,
            };

            // Extract the base queue name (remove DLQ suffix if present)
            let base_queue_name = if current_queue_name.ends_with("/$deadletterqueue") {
                current_queue_name
                    .strip_suffix("/$deadletterqueue")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| {
                        log::warn!(
                            "Failed to strip DLQ suffix from queue name: {current_queue_name}"
                        );
                        current_queue_name.clone()
                    })
            } else {
                current_queue_name.clone()
            };

            let target_queue = match new_queue_type {
                QueueType::Main => base_queue_name,
                QueueType::DeadLetter => format!("{base_queue_name}/$deadletterqueue"),
            };

            self.pending_queue = Some(target_queue.clone());
            self.current_queue_name = Some(target_queue.clone());
            self.current_queue_type = new_queue_type;
            self.messages = None;
            self.message_pagination.reset();

            log::info!(
                "Queue toggle: cleared all message cache, switching from {:?} to {:?} ({})",
                match self.current_queue_type {
                    QueueType::Main => QueueType::DeadLetter,
                    QueueType::DeadLetter => QueueType::Main,
                },
                self.current_queue_type,
                target_queue
            );

            // Clear selections when switching queues (as per user requirement)
            self.bulk_selection.clear_all();

            Some(target_queue)
        } else {
            None
        }
    }
}
