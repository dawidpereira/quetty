use crate::app::model::AppState;
use crate::app::model::Model;
use crate::app::updates::messages::async_operations;
use crate::components::common::{ComponentId, MessageActivityMsg, Msg, PopupActivityMsg};
use crate::error::AppError;
use server::bulk_operations::MessageIdentifier;
use server::service_bus_manager::{MessageData, ServiceBusCommand, ServiceBusResponse};
use std::sync::Arc;

use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Handle editing a message at the given index
    pub fn handle_edit_message(&mut self, index: usize) -> Option<Msg> {
        if let Some(current_messages) = &self.queue_manager.queue_state.messages {
            if index < current_messages.len() {
                let selected_message = &current_messages[index];
                log::info!("Starting to edit message {}", selected_message.id);

                // First, defocus the messages component
                if let Err(e) = self.remount_messages_with_focus(false) {
                    self.error_reporter
                        .report_simple(e, "MessageEditor", "handle_edit_message");
                    return Some(Msg::ShowError(
                        "Failed to prepare message editing view".to_string(),
                    ));
                }

                // Set the app state to MessageDetails
                self.set_app_state(AppState::MessageDetails);

                // Activate the MessageDetails component - with proper error recovery
                if let Err(e) = self.app.active(&ComponentId::MessageDetails) {
                    self.error_reporter
                        .report_activation_error("MessageDetails", &e);
                    // Recovery: go back to message picker state
                    self.set_app_state(AppState::MessagePicker);
                    return Some(Msg::ShowError(
                        "Failed to open message editor. Please try again.".to_string(),
                    ));
                }

                // Remount MessageDetails with the selected message
                if let Err(e) = self.remount_message_details(index) {
                    self.error_reporter
                        .report_simple(e, "MessageEditor", "handle_edit_message");
                    // Recovery: go back to message picker state
                    self.set_app_state(AppState::MessagePicker);
                    return Some(Msg::ShowError(
                        "Failed to load message for editing. Please try again.".to_string(),
                    ));
                }

                self.set_editing_message(true);
                if let Err(e) = self.update_global_key_watcher_editing_state() {
                    self.error_reporter.report_key_watcher_error(&e);
                    // This is not critical - continue anyway
                }

                Some(Msg::ForceRedraw)
            } else {
                log::warn!("Index {} out of bounds for messages", index);
                None
            }
        } else {
            log::warn!("No messages available to edit");
            None
        }
    }

    /// Handle canceling message edit
    pub fn handle_cancel_edit_message(&mut self) -> Option<Msg> {
        self.set_editing_message(false);
        if let Err(e) = self.update_global_key_watcher_editing_state() {
            self.error_reporter.report_key_watcher_error(&e);
        }

        // Transition back to message picker view
        self.set_app_state(AppState::MessagePicker);

        // Activate the Messages component first
        if let Err(e) = self.app.active(&ComponentId::Messages) {
            self.error_reporter.report_activation_error("Messages", &e);
        }

        // Re-focus the messages component with focus
        if let Err(e) = self.remount_messages_with_focus(true) {
            self.error_reporter
                .report_mount_error("Messages", "remount_with_focus", &e);
        }

        // Remount message details without focus to remove focus styling
        // (now that Messages is active, MessageDetails will be remounted without focus)
        if let Err(e) = self.remount_message_details(0) {
            self.error_reporter
                .report_mount_error("MessageDetails", "remount_without_focus", &e);
        }

        log::debug!("Cancelled editing message and returned to message list");
        Some(Msg::ForceRedraw)
    }

    /// Handle sending edited message content as new message
    pub fn handle_send_edited_message(&self, content: String) -> Option<Msg> {
        let queue_name = match self.get_current_queue() {
            Ok(name) => name,
            Err(e) => return Some(Msg::PopupActivity(PopupActivityMsg::ShowError(e))),
        };

        let repeat_count = self.queue_manager.queue_state.message_repeat_count;
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

        let service_bus_manager = Arc::clone(&self.service_bus_manager);
        let tx_to_main = self.state_manager.tx_to_main.clone();

        self.task_manager.execute(loading_message, async move {
            let result = if repeat_count == 1 {
                Self::send_single_message(service_bus_manager, queue_name, content).await
            } else {
                Self::send_multiple_messages(service_bus_manager, queue_name, content, repeat_count)
                    .await
            };

            let success_message = if repeat_count == 1 {
                "✅ Message sent successfully!".to_string()
            } else {
                format!("✅ {} messages sent successfully!", repeat_count)
            };

            async_operations::send_completion_messages(
                &tx_to_main,
                result.clone(),
                &success_message,
            );
            result
        });

        None
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
            message_id,
            queue_name
        );

        let service_bus_manager = Arc::clone(&self.service_bus_manager);
        let tx_to_main = self.state_manager.tx_to_main.clone();
        let message_id_str = message_id.to_string();

        self.task_manager
            .execute("Replacing message...", async move {
                let result = async {
                    // Step 1: Send new message with edited content
                    Self::send_single_message(Arc::clone(&service_bus_manager), queue_name.clone(), content)
                        .await?;

                    // Step 2: Delete original message using service bus manager
                    let delete_command = ServiceBusCommand::BulkDelete {
                        message_ids: vec![message_id],
                    };

                    let delete_response = service_bus_manager.lock().await.execute_command(delete_command).await;

                    match delete_response {
                        ServiceBusResponse::BulkOperationCompleted { result } => {
                            if result.successful > 0 {
                                log::info!(
                                    "Successfully deleted original message {} in queue: {}",
                                    message_id_str,
                                    queue_name
                                );
                            } else {
                                log::warn!(
                                    "Message {} was not found or could not be deleted (may have been already processed)",
                                    message_id_str
                                );
                            }
                        }
                        ServiceBusResponse::Error { error } => {
                            log::error!("Failed to delete original message: {}", error);
                            return Err(AppError::ServiceBus(format!("Failed to delete original message: {}", error)));
                        }
                        _ => {
                            return Err(AppError::ServiceBus("Unexpected response for bulk delete".to_string()));
                        }
                    }

                    log::info!(
                        "Successfully replaced message {} in queue: {}",
                        message_id_str,
                        queue_name
                    );
                    Ok::<(), AppError>(())
                }
                .await;

                if result.is_ok() {
                    let _ = tx_to_main.send(Msg::MessageActivity(
                        MessageActivityMsg::BulkRemoveMessagesFromState(vec![message_id_str]),
                    ));
                }

                async_operations::send_completion_messages(
                    &tx_to_main,
                    result.clone(),
                    "✅ Message replaced successfully!",
                );
                result
            });

        None
    }

    /// Send a single message to a queue using the service bus manager
    async fn send_single_message(
        service_bus_manager: std::sync::Arc<
            tokio::sync::Mutex<server::service_bus_manager::ServiceBusManager>,
        >,
        queue_name: String,
        content: String,
    ) -> Result<(), AppError> {
        log::info!(
            "Sending message to queue: {} (content: {} bytes)",
            queue_name,
            content.len()
        );

        let message = MessageData::new(content);
        let command = ServiceBusCommand::SendMessage {
            queue_name: queue_name.clone(),
            message,
        };

        let response = service_bus_manager
            .lock()
            .await
            .execute_command(command)
            .await;

        match response {
            ServiceBusResponse::MessageSent { .. } => {
                log::info!("Successfully sent message to queue: {}", queue_name);
                Ok(())
            }
            ServiceBusResponse::Error { error } => {
                log::error!("Failed to send message to queue {}: {}", queue_name, error);
                Err(AppError::ServiceBus(error.to_string()))
            }
            _ => Err(AppError::ServiceBus(
                "Unexpected response for send message".to_string(),
            )),
        }
    }

    /// Send multiple messages to a queue using the service bus manager
    async fn send_multiple_messages(
        service_bus_manager: std::sync::Arc<
            tokio::sync::Mutex<server::service_bus_manager::ServiceBusManager>,
        >,
        queue_name: String,
        content: String,
        count: usize,
    ) -> Result<(), AppError> {
        log::info!("Sending message {} times to queue: {}", count, queue_name);

        let messages: Vec<MessageData> = (0..count)
            .map(|_| MessageData::new(content.clone()))
            .collect();
        let command = ServiceBusCommand::SendMessages {
            queue_name: queue_name.clone(),
            messages,
        };

        let response = service_bus_manager
            .lock()
            .await
            .execute_command(command)
            .await;

        match response {
            ServiceBusResponse::MessagesSent { stats, .. } => {
                if stats.successful >= count {
                    log::info!(
                        "Successfully sent {} messages to queue: {}",
                        stats.successful,
                        queue_name
                    );
                    Ok(())
                } else {
                    let error_msg = format!(
                        "Failed to send all messages: {} successful, {} failed out of {} requested",
                        stats.successful, stats.failed, count
                    );
                    log::error!("{}", error_msg);
                    Err(AppError::ServiceBus(error_msg))
                }
            }
            ServiceBusResponse::Error { error } => {
                log::error!("Failed to send messages to queue {}: {}", queue_name, error);
                Err(AppError::ServiceBus(error.to_string()))
            }
            _ => Err(AppError::ServiceBus(
                "Unexpected response for send messages".to_string(),
            )),
        }
    }
}
