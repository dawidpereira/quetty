use crate::app::model::Model;
use crate::components::common::Msg;
use server::bulk_operations::MessageIdentifier;
use tuirealm::terminal::TerminalAdapter;

pub mod delete_operations;

pub mod send_operations;
pub mod task_manager;
pub mod validation;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Execute bulk resend from DLQ operation
    pub fn handle_bulk_resend_from_dlq_execution(
        &mut self,
        message_ids: Vec<MessageIdentifier>,
    ) -> Option<Msg> {
        send_operations::handle_bulk_resend_from_dlq_execution(self, message_ids)
    }

    /// Execute bulk resend-only from DLQ operation (without deleting from DLQ)
    pub fn handle_bulk_resend_from_dlq_only_execution(
        &mut self,
        message_ids: Vec<MessageIdentifier>,
    ) -> Option<Msg> {
        send_operations::handle_bulk_resend_from_dlq_only_execution(self, message_ids)
    }

    /// Execute bulk send to DLQ operation with deletion (move to DLQ)
    pub fn handle_bulk_send_to_dlq_with_delete_execution(
        &mut self,
        message_ids: Vec<MessageIdentifier>,
    ) -> Option<Msg> {
        send_operations::handle_bulk_send_to_dlq_with_delete_execution(self, message_ids)
    }

    /// Execute bulk delete operation
    pub fn handle_bulk_delete_execution(
        &mut self,
        message_ids: Vec<MessageIdentifier>,
    ) -> Option<Msg> {
        delete_operations::handle_bulk_delete_execution(self, message_ids)
    }
}
