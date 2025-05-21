use crate::app::model::Model;
use crate::components::common::ComponentId;
use crate::components::message_details::MessageDetails;
use crate::components::messages::Messages;
use crate::components::namespace_picker::NamespacePicker;
use crate::components::queue_picker::QueuePicker;
use crate::error::{AppError, AppResult};
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn remount_message_details(&mut self, index: usize) -> AppResult<()> {
        let message = if let Some(messages) = &self.messages {
            messages.get(index).cloned()
        } else {
            None
        };

        self.app
            .remount(
                ComponentId::MessageDetails,
                Box::new(MessageDetails::new(message)),
                Vec::default(),
            )
            .map_err(|e| AppError::Component(e.to_string()))?;

        Ok(())
    }

    pub fn remount_messages(&mut self) -> AppResult<()> {
        self.app
            .remount(
                ComponentId::Messages,
                Box::new(Messages::new(self.messages.as_ref())),
                Vec::default(),
            )
            .map_err(|e| AppError::Component(e.to_string()))?;

        Ok(())
    }

    pub fn remount_queue_picker(&mut self, queues: Option<Vec<String>>) -> AppResult<()> {
        self.app
            .remount(
                ComponentId::QueuePicker,
                Box::new(QueuePicker::new(queues)),
                Vec::default(),
            )
            .map_err(|e| AppError::Component(e.to_string()))?;

        Ok(())
    }

    pub fn remount_namespace_picker(&mut self, namespaces: Option<Vec<String>>) -> AppResult<()> {
        self.app
            .remount(
                ComponentId::NamespacePicker,
                Box::new(NamespacePicker::new(namespaces)),
                Vec::default(),
            )
            .map_err(|e| AppError::Component(e.to_string()))?;

        Ok(())
    }
}
