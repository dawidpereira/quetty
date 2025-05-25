use crate::app::updates::messages::MessagePaginationState;
use crate::components::common::QueueType;
use server::consumer::Consumer;
use server::model::MessageModel;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Encapsulates all queue-related state and data
#[derive(Debug)]
pub struct QueueState {
    /// Queue name that is pending selection (before consumer is created)
    pub pending_queue: Option<String>,
    /// Currently selected queue name
    pub current_queue_name: Option<String>,
    /// Current queue type (Main or DeadLetter)
    pub current_queue_type: QueueType,
    /// Active consumer for the current queue
    pub consumer: Option<Arc<Mutex<Consumer>>>,
    /// Currently loaded messages
    pub messages: Option<Vec<MessageModel>>,
    /// Message pagination state
    pub message_pagination: MessagePaginationState,
}

impl Default for QueueState {
    fn default() -> Self {
        Self {
            pending_queue: None,
            current_queue_name: None,
            current_queue_type: QueueType::Main,
            consumer: None,
            messages: None,
            message_pagination: MessagePaginationState::default(),
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
        if let Some(queue_name) = &self.current_queue_name {
            let new_queue_type = match self.current_queue_type {
                QueueType::Main => QueueType::DeadLetter,
                QueueType::DeadLetter => QueueType::Main,
            };

            let target_queue = match new_queue_type {
                QueueType::Main => queue_name.clone(),
                QueueType::DeadLetter => format!("{}/$deadletterqueue", queue_name),
            };

            self.pending_queue = Some(target_queue.clone());
            self.current_queue_type = new_queue_type;

            // Clear current messages and pagination state when switching
            self.messages = None;
            self.message_pagination.reset();

            Some(target_queue)
        } else {
            None
        }
    }
}

