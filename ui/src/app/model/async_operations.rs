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
        if let Some(queue) = self.queue_manager.queue_state.pending_queue.clone() {
            self.queue_manager.switch_to_queue(queue);
        }
    }

    /// Force reload messages - useful after bulk operations that modify the queue
    pub fn handle_force_reload_messages(&mut self) -> Option<Msg> {
        log::info!("Force reloading messages after bulk operation - complete state reset");

        self.reset_pagination_state();
        self.queue_state_mut().messages = None;
        self.queue_state_mut().bulk_selection.clear_all();

        if let Err(e) = self
            .app
            .active(&crate::components::common::ComponentId::Messages)
        {
            log::warn!(
                "Failed to activate messages component during force reload: {}",
                e
            );
        }

        self.message_manager.force_reload_messages();

        // Force a full UI redraw to clear stale state
        self.set_redraw(true);
        Some(crate::components::common::Msg::ForceRedraw)
    }
}
