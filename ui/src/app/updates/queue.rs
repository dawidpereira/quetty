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
                self.queue_state.set_selected_queue(queue);
                if let Err(e) = self.new_consumer_for_queue() {
                    self.error_reporter
                        .report_simple(e, "QueueHandler", "update_queue");
                    return None;
                }
                None
            }
            QueueActivityMsg::QueuesLoaded(queues) => {
                if let Err(e) = self.remount_queue_picker(Some(queues)) {
                    self.error_reporter
                        .report_simple(e, "QueueHandler", "update_queue");
                    return None;
                }
                self.app_state = AppState::QueuePicker;
                None
            }
            QueueActivityMsg::QueueUnselected => {
                self.app_state = AppState::QueuePicker;
                None
            }
            QueueActivityMsg::ToggleDeadLetterQueue => {
                if self.queue_state.toggle_queue_type().is_some() {
                    if let Err(e) = self.new_consumer_for_queue() {
                        self.error_reporter
                            .report_simple(e, "QueueHandler", "update_queue");
                        return None;
                    }
                }
                None
            }
        }
    }
}
