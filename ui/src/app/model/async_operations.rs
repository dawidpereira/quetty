use super::Model;
use crate::components::common::Msg;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Load namespaces using QueueManager
    pub fn load_namespaces(&self) {
        self.queue_manager.load_namespaces();
    }

    /// Load queues using QueueManager
    pub fn load_queues(&self) {
        self.queue_manager.load_queues();
    }

    /// Create new consumer for the selected queue using QueueManager
    pub fn new_consumer_for_queue(&mut self) {
        // Extract the queue from the queue manager
        if let Some(queue) = self.queue_manager.queue_state.pending_queue.clone() {
            self.queue_manager.switch_to_queue(queue);
        }
    }

    /// Load messages from current queue using MessageManager
    pub fn load_messages(&self) {
        self.message_manager.load_messages();
    }

    /// Force reload messages - useful after bulk operations that modify the queue
    pub fn handle_force_reload_messages(&mut self) -> Option<Msg> {
        log::info!("Force reloading messages after bulk operation - resetting pagination state");

        // Reset pagination state to clear all existing messages and start fresh
        self.reset_pagination_state();

        // Trigger reload
        self.message_manager.force_reload_messages();

        None
    }
}
