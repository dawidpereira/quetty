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
                self.pending_queue = Some(queue);
                if let Err(e) = self.new_consumer_for_queue() {
                    return Some(Msg::Error(e));
                }
                None
            }
            QueueActivityMsg::QueuesLoaded(queues) => {
                if let Err(e) = self.remount_queue_picker(Some(queues)) {
                    return Some(Msg::Error(e));
                }
                self.app_state = AppState::QueuePicker;
                None
            }
            QueueActivityMsg::QueueUnselected => {
                self.app_state = AppState::QueuePicker;
                None
            }
        }
    }
}

