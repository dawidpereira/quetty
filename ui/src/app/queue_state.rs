use crate::app::updates::messages::MessagePaginationState;
use server::bulk_operations::MessageIdentifier;
use server::model::MessageModel;
use server::service_bus_manager::QueueType;
use std::collections::HashSet;

/// Unique identifier for a message combining ID and sequence
/// State for managing bulk selection of messages
#[derive(Debug, Default)]
pub struct BulkSelectionState {
    /// Set of selected message identifiers
    pub selected_messages: HashSet<MessageIdentifier>,
    /// Whether we're currently in bulk selection mode
    pub selection_mode: bool,
    /// Index of the last selected message for range selection (future use)
    pub last_selected_index: Option<usize>,
}

impl BulkSelectionState {
    /// Toggle selection for a message
    pub fn toggle_selection(&mut self, message_id: MessageIdentifier) -> bool {
        if self.selected_messages.contains(&message_id) {
            self.selected_messages.remove(&message_id);
            false
        } else {
            self.selected_messages.insert(message_id);
            true
        }
    }

    /// Select all messages from a given list
    pub fn select_all(&mut self, messages: &[MessageModel]) {
        for message in messages {
            self.selected_messages
                .insert(MessageIdentifier::from_message(message));
        }
        if !messages.is_empty() {
            self.selection_mode = true;
        }
    }

    /// Clear all selections
    pub fn clear_all(&mut self) {
        self.selected_messages.clear();
        self.selection_mode = false;
        self.last_selected_index = None;
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

    /// Remove messages from selection (used when messages are deleted/moved)
    pub fn remove_messages(&mut self, message_ids: &[MessageIdentifier]) {
        for id in message_ids {
            self.selected_messages.remove(id);
        }
        // Exit selection mode if no messages are selected
        if self.selected_messages.is_empty() {
            self.selection_mode = false;
        }
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
                            "Failed to strip DLQ suffix from queue name: {}",
                            current_queue_name
                        );
                        current_queue_name.clone()
                    })
            } else {
                current_queue_name.clone()
            };

            let target_queue = match new_queue_type {
                QueueType::Main => base_queue_name,
                QueueType::DeadLetter => format!("{}/$deadletterqueue", base_queue_name),
            };

            self.pending_queue = Some(target_queue.clone());
            self.current_queue_type = new_queue_type;

            // Clear current messages and pagination state when switching
            self.messages = None;
            self.message_pagination.reset();

            // Clear selections when switching queues (as per user requirement)
            self.bulk_selection.clear_all();

            Some(target_queue)
        } else {
            None
        }
    }
}
