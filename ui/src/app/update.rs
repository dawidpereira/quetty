use crate::app::model::{AppState, Model};
use crate::components::common::{MessageActivityMsg, Msg, NamespaceActivityMsg, QueueActivityMsg};
use std::sync::Arc;
use tokio::sync::Mutex;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn update_messages(&mut self, msg: MessageActivityMsg) -> Option<Msg> {
        match msg {
            MessageActivityMsg::EditMessage(index) => {
                if let Err(e) = self.remount_message_details(index) {
                    return Some(Msg::Error(e));
                }
                self.app_state = AppState::MessageDetails;
                Some(Msg::ForceRedraw)
            }
            MessageActivityMsg::CancelEditMessage => {
                self.app_state = AppState::MessagePicker;
                None
            }
            MessageActivityMsg::MessagesLoaded(messages) => {
                self.messages = Some(messages);
                if let Err(e) = self.remount_messages() {
                    return Some(Msg::Error(e));
                }
                if let Err(e) = self.remount_message_details(0) {
                    return Some(Msg::Error(e));
                }
                self.app_state = AppState::MessagePicker;
                None
            }
            MessageActivityMsg::ConsumerCreated(consumer) => {
                self.consumer = Some(Arc::new(Mutex::new(consumer)));
                if let Err(e) = self.load_messages() {
                    return Some(Msg::Error(e));
                }
                None
            }
            MessageActivityMsg::PreviewMessageDetails(index) => {
                if let Err(e) = self.remount_message_details(index) {
                    return Some(Msg::Error(e));
                }
                None
            }
        }
    }

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

    pub fn update_namespace(&mut self, msg: NamespaceActivityMsg) -> Option<Msg> {
        match msg {
            NamespaceActivityMsg::NamespacesLoaded(namespace) => {
                if let Err(e) = self.remount_namespace_picker(Some(namespace)) {
                    return Some(Msg::Error(e));
                }
                self.app_state = AppState::NamespacePicker;
                None
            }
            NamespaceActivityMsg::NamespaceSelected => {
                if let Err(e) = self.load_queues() {
                    return Some(Msg::Error(e));
                }
                None
            }
            NamespaceActivityMsg::NamespaceUnselected => {
                if let Err(e) = self.load_namespaces() {
                    return Some(Msg::Error(e));
                }
                None
            }
        }
    }
}
