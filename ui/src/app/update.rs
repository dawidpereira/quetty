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
                self.remount_message_details(index);
                self.app_state = AppState::MessageDetails;
                Some(Msg::ForceRedraw)
            }
            MessageActivityMsg::CancelEditMessage => {
                self.app_state = AppState::MessagePicker;
                None
            }
            MessageActivityMsg::MessagesLoaded(messages) => {
                self.messages = Some(messages);
                self.remount_messages();
                self.remount_message_details(0);
                self.app_state = AppState::MessagePicker;
                None
            }
            MessageActivityMsg::ConsumerCreated(consumer) => {
                self.consumer = Some(Arc::new(Mutex::new(consumer)));
                self.load_messages();
                None
            }
            MessageActivityMsg::PreviewMessageDetails(index) => {
                self.remount_message_details(index);
                None
            }
        }
    }

    pub fn update_queue(&mut self, msg: QueueActivityMsg) -> Option<Msg> {
        match msg {
            QueueActivityMsg::QueueSelected(queue) => {
                self.pending_queue = Some(queue);
                self.new_consumer_for_queue();
                None
            }
            QueueActivityMsg::QueuesLoaded(queues) => {
                self.remount_queue_picker(Some(queues));
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
                self.remount_namespace_picker(Some(namespace));
                self.app_state = AppState::NamespacePicker;
                None
            }
            NamespaceActivityMsg::NamespaceSelected => {
                self.load_queues();
                None
            }
            NamespaceActivityMsg::NamespaceUnselected => {
                self.load_namespaces();
                None
            }
        }
    }
}
