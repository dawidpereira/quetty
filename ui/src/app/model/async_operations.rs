use super::Model;
use crate::components::common::Msg;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Load namespaces using QueueManager
    pub fn load_namespaces(
        &self,
        navigation_context: crate::app::managers::state_manager::NavigationContext,
    ) {
        // Don't load namespaces if authentication is in progress
        if self.state_manager.is_authenticating {
            log::info!("Skipping namespace loading - authentication in progress");
            return;
        }
        log::info!("Proceeding with namespace loading - not authenticating");
        self.queue_manager.load_namespaces(navigation_context);
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
        // Check if we have a current queue selected
        if self.queue_state().current_queue_name.is_none() {
            log::warn!("Cannot reload messages: no queue currently selected");
            return None;
        }

        // Prevent multiple rapid refresh operations
        if self.queue_state().message_pagination.is_loading() {
            log::debug!("Ignoring refresh request: messages are already loading");
            return None;
        }

        log::info!("Force reloading messages from beginning of queue");
        let current_page_size = crate::config::get_current_page_size();
        log::info!("Force reload will use current page size: {current_page_size} messages");

        // Complete state reset for fresh data
        self.reset_pagination_state();
        self.queue_state_mut().messages = None;
        self.queue_state_mut().bulk_selection.clear_all();

        // Reset last loaded sequence to force loading from beginning
        self.queue_state_mut()
            .message_pagination
            .last_loaded_sequence = None;
        self.queue_state_mut()
            .message_pagination
            .reached_end_of_queue = false;

        if let Err(e) = self
            .app
            .active(&crate::components::common::ComponentId::Messages)
        {
            log::warn!("Failed to activate messages component during force reload: {e}");
        }

        // Use force loading to bypass "already loading" checks and ensure fresh data load
        // Load from current position (None) instead of beginning (Some(0)) to get fresh messages
        if let Err(e) = self.load_messages_from_api_with_force_sequence(current_page_size, None) {
            log::error!("Failed to reload messages: {e}");
        }

        // Force a full UI redraw to clear stale state
        self.set_redraw(true);
        Some(crate::components::common::Msg::ForceRedraw)
    }
}
