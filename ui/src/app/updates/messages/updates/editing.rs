use crate::app::model::{AppState, Model};
use crate::app::updates::messages::async_operations;
use crate::components::common::{
    ComponentId, LoadingActivityMsg, MessageActivityMsg, Msg, PopupActivityMsg,
};
use crate::error::AppError;
use azservicebus::{ServiceBusClient, core::BasicRetryPolicy};
use server::bulk_operations::MessageIdentifier;
use server::consumer::Consumer;
use server::producer::ServiceBusClientProducerExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Handle starting message editing
    pub fn handle_edit_message(&mut self, index: usize) -> Option<Msg> {
        if let Err(e) = self.remount_messages_with_focus(false) {
            self.error_reporter
                .report_simple(e, "MessageEditor", "handle_edit_message");
            return None;
        }

        self.app_state = AppState::MessageDetails;

        if let Err(e) = self.app.active(&ComponentId::MessageDetails) {
            log::error!("Failed to activate message details: {}", e);
        }

        if let Err(e) = self.remount_message_details(index) {
            self.error_reporter
                .report_simple(e, "MessageEditor", "handle_edit_message");
            return None;
        }

        Some(Msg::ForceRedraw)
    }

    /// Handle canceling message editing
    pub fn handle_cancel_edit_message(&mut self) -> Option<Msg> {
        self.is_editing_message = false;
        if let Err(e) = self.update_global_key_watcher_editing_state() {
            log::error!("Failed to update global key watcher: {}", e);
        }

        if let Err(e) = self.remount_messages_with_focus(true) {
            self.error_reporter
                .report_simple(e, "MessageEditor", "handle_cancel_edit_message");
            return None;
        }

        self.app_state = AppState::MessagePicker;

        if let Err(e) = self.app.active(&ComponentId::Messages) {
            log::error!("Failed to activate messages: {}", e);
        }

        if let Err(e) = self.remount_message_details(0) {
            self.error_reporter
                .report_simple(e, "MessageEditor", "handle_cancel_edit_message");
            return None;
        }

        None
    }

    /// Handle sending edited message content as a new message
    pub fn handle_send_edited_message(&self, content: String) -> Option<Msg> {
        let queue_name = match self.get_current_queue() {
            Ok(name) => name,
            Err(e) => return Some(Msg::PopupActivity(PopupActivityMsg::ShowError(e))),
        };

        let repeat_count = self.queue_state.message_repeat_count;
        log::info!(
            "Sending edited message content to queue: {} ({} times)",
            queue_name,
            repeat_count
        );

        let loading_message = if repeat_count == 1 {
            "Sending message...".to_string()
        } else {
            format!("Sending message {} times...", repeat_count)
        };

        let feedback_msg = Some(Msg::LoadingActivity(LoadingActivityMsg::Start(
            loading_message,
        )));

        let service_bus_client = self.service_bus_client.clone();
        let tx_to_main = self.tx_to_main.clone();
        let taskpool = &self.taskpool;

        let task = async move {
            let result = Self::send_message_multiple_times(
                service_bus_client,
                queue_name,
                content,
                repeat_count,
            )
            .await;

            let success_message = if repeat_count == 1 {
                "✅ Message sent successfully!".to_string()
            } else {
                format!("✅ {} messages sent successfully!", repeat_count)
            };

            async_operations::send_completion_messages(&tx_to_main, result, &success_message)
        };

        taskpool.execute(task);

        feedback_msg
    }

    /// Handle replacing original message with edited content (send new + delete original)
    pub fn handle_replace_edited_message(
        &self,
        content: String,
        message_id: MessageIdentifier,
    ) -> Option<Msg> {
        let queue_name = match self.get_current_queue() {
            Ok(name) => name,
            Err(e) => return Some(Msg::PopupActivity(PopupActivityMsg::ShowError(e))),
        };

        log::info!(
            "Replacing message {} with edited content in queue: {}",
            message_id.id,
            queue_name
        );

        let feedback_msg = Some(Msg::LoadingActivity(LoadingActivityMsg::Start(
            "Replacing message...".to_string(),
        )));

        let consumer = self.queue_state.consumer.clone();
        let service_bus_client = self.service_bus_client.clone();
        let tx_to_main = self.tx_to_main.clone();
        let taskpool = &self.taskpool;

        let task = async move {
            let result = async {
                // Step 1: Send new message with edited content
                Self::send_message_to_queue(service_bus_client, queue_name.clone(), content)
                    .await?;

                // Step 2: Delete original message
                match consumer {
                    Some(consumer) => Self::delete_message(consumer, &message_id).await?,
                    None => {
                        return Err(AppError::ServiceBus(
                            "No consumer available for message deletion".to_string(),
                        ));
                    }
                }

                log::info!(
                    "Successfully replaced message {} in queue: {}",
                    message_id.id,
                    queue_name
                );
                Ok::<(), AppError>(())
            }
            .await;

            if result.is_ok() {
                let _ = tx_to_main.send(Msg::MessageActivity(
                    MessageActivityMsg::BulkRemoveMessagesFromState(vec![message_id]),
                ));
            }

            async_operations::send_completion_messages(
                &tx_to_main,
                result,
                "✅ Message replaced successfully!",
            )
        };

        taskpool.execute(task);

        // Return immediate feedback, then stay in current state until task completes
        feedback_msg
    }

    /// Helper function to delete a message by completing it
    async fn delete_message(
        consumer: Arc<Mutex<Consumer>>,
        message_id: &MessageIdentifier,
    ) -> Result<(), AppError> {
        use crate::app::updates::messages::utils::find_target_message;

        let mut consumer_guard = consumer.lock().await;
        match find_target_message(&mut consumer_guard, &message_id.id, message_id.sequence).await {
            Ok(target_msg) => {
                consumer_guard
                    .complete_message(&target_msg)
                    .await
                    .map_err(|e| {
                        AppError::ServiceBus(format!("Failed to delete message: {}", e))
                    })?;
                log::info!("Successfully deleted message: {}", message_id.id);
                Ok(())
            }
            Err(e) => {
                log::warn!(
                    "Message {} not found (may have been already processed): {}",
                    message_id.id,
                    e
                );
                Ok(())
            }
        }
    }

    /// Send a message to a queue
    async fn send_message_to_queue(
        service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
        queue_name: String,
        content: String,
    ) -> Result<(), AppError> {
        let mut client = service_bus_client.lock().await;
        let mut producer = client
            .create_producer_for_queue(
                &queue_name,
                azservicebus::ServiceBusSenderOptions::default(),
            )
            .await
            .map_err(|e| AppError::ServiceBus(format!("Failed to create producer: {}", e)))?;

        let message = azservicebus::ServiceBusMessage::new(content.as_bytes().to_vec());

        producer
            .send_message(message)
            .await
            .map_err(|e| AppError::ServiceBus(format!("Failed to send message: {}", e)))?;

        producer
            .dispose()
            .await
            .map_err(|e| AppError::ServiceBus(format!("Failed to dispose producer: {}", e)))?;

        log::info!("Successfully sent message to queue: {}", queue_name);
        Ok(())
    }

    /// Send a message multiple times to a queue
    async fn send_message_multiple_times(
        service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
        queue_name: String,
        content: String,
        count: usize,
    ) -> Result<(), AppError> {
        log::info!("Sending message {} times to queue: {}", count, queue_name);

        // Send messages sequentially to avoid overwhelming the service
        for i in 1..=count {
            log::debug!("Sending message {}/{} to queue: {}", i, count, queue_name);
            Self::send_message_to_queue(
                service_bus_client.clone(),
                queue_name.clone(),
                content.clone(),
            )
            .await?;
        }

        log::info!(
            "Successfully sent {} messages to queue: {}",
            count,
            queue_name
        );
        Ok(())
    }
}
