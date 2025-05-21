use crate::app::model::Model;
use crate::components::common::ComponentId;
use crate::components::message_details::MessageDetails;
use crate::components::messages::Messages;
use crate::components::namespace_picker::NamespacePicker;
use crate::components::queue_picker::QueuePicker;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn remount_messages(&mut self) {
        assert!(
            self.app
                .remount(
                    ComponentId::Messages,
                    Box::new(Messages::new(self.messages.as_ref())),
                    Vec::default(),
                )
                .is_ok()
        );
    }

    pub fn remount_message_details(&mut self, index: usize) {
        if self.messages.is_some() {
            let message = self.messages.as_ref().unwrap().get(index).cloned();

            assert!(
                self.app
                    .remount(
                        ComponentId::MessageDetails,
                        Box::new(MessageDetails::new(message)),
                        Vec::default(),
                    )
                    .is_ok()
            );
        }
    }

    pub fn remount_queue_picker(&mut self, queues: Option<Vec<String>>) {
        assert!(
            self.app
                .remount(
                    ComponentId::QueuePicker,
                    Box::new(QueuePicker::new(queues)),
                    Vec::default(),
                )
                .is_ok()
        );
    }

    pub fn remount_namespace_picker(&mut self, namespaces: Option<Vec<String>>) {
        assert!(
            self.app
                .remount(
                    ComponentId::NamespacePicker,
                    Box::new(NamespacePicker::new(namespaces)),
                    Vec::default(),
                )
                .is_ok()
        );
    }
}
