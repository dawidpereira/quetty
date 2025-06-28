use crate::app::model::{AppState, Model};
use crate::components::common::{Msg, QueueActivityMsg};
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn update_queue(&mut self, msg: QueueActivityMsg) -> Option<Msg> {
        match msg {
            QueueActivityMsg::QueueSelected(queue) => {
                self.queue_state_mut().set_selected_queue(queue);
                self.new_consumer_for_queue();

                // Load stats for the newly selected queue
                self.load_stats_for_current_queue();

                None
            }
            QueueActivityMsg::QueuesLoaded(queues) => {
                if let Err(e) = self.remount_queue_picker(Some(queues)) {
                    self.error_reporter
                        .report_simple(e, "QueueHandler", "update_queue");
                    return None;
                }
                None
            }
            QueueActivityMsg::QueueUnselected => {
                self.set_app_state(AppState::QueuePicker);
                None
            }
            QueueActivityMsg::ToggleDeadLetterQueue => {
                if self.queue_state_mut().toggle_queue_type().is_some() {
                    log::info!("Toggled queue type, switching consumer");
                    self.new_consumer_for_queue();

                    // Load stats for the toggled queue type
                    self.load_stats_for_current_queue();
                }
                None
            }
        }
    }

    /// Load statistics for current queue - check cache first, then API if needed
    fn load_stats_for_current_queue(&mut self) {
        let queue_name = self
            .queue_state()
            .current_queue_name
            .clone()
            .unwrap_or_default();
        let base_queue_name = if queue_name.ends_with("/$deadletterqueue") {
            queue_name.trim_end_matches("/$deadletterqueue").to_string()
        } else {
            queue_name
        };

        // Check if we have valid cache
        if self
            .queue_state()
            .stats_manager
            .has_valid_cache(&base_queue_name)
        {
            log::info!("Using cached stats for queue: {}", base_queue_name);
            // Cache is valid - stats will be displayed immediately in UI
            return;
        }

        log::info!(
            "No valid cache for queue: {}, loading from API",
            base_queue_name
        );

        // No valid cache - load from API in background
        if let Err(e) = self.load_queue_statistics_from_api(&base_queue_name) {
            log::error!("Failed to load queue statistics: {}", e);
        }
    }
}
