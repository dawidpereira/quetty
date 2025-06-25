use crate::app::model::{AppState, Model};
use crate::components::common::{ComponentId, MessageActivityMsg, Msg, PopupActivityMsg};
use crate::config;
use crate::error::AppError;
use std::sync::mpsc::Sender;
use tuirealm::terminal::TerminalAdapter;

// Bulk send configuration is now handled via the config system
impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Handle opening empty message details in edit mode for composition
    pub fn handle_compose_new_message(&mut self) -> Option<Msg> {
        if let Err(e) = self.remount_messages_with_focus(false) {
            self.error_reporter
                .report_simple(e, "MessageComposer", "handle_compose_new_message");
            return None;
        }

        self.set_app_state(AppState::MessageDetails);

        if let Err(e) = self.app.active(&ComponentId::MessageDetails) {
            self.error_reporter
                .report_activation_error("MessageDetails", &e);
        }

        if let Err(e) = self.remount_message_details_for_composition() {
            self.error_reporter
                .report_simple(e, "MessageComposer", "handle_compose_new_message");
            return None;
        }

        self.set_editing_message(true);
        if let Err(e) = self.update_global_key_watcher_editing_state() {
            self.error_reporter.report_key_watcher_error(&e);
        }

        Some(Msg::ForceRedraw)
    }

    /// Handle setting the repeat count for bulk message sending
    pub fn handle_set_message_repeat_count(&self) -> Option<Msg> {
        let bulk_config = config::get_config_or_panic().batch();

        Some(Msg::PopupActivity(PopupActivityMsg::ShowNumberInput {
            title: "Set Repeat Count".to_string(),
            message: format!(
                "Enter the number of times to repeat sending selected messages (Min: {}, Max: {})",
                bulk_config.bulk_operation_min_count(),
                bulk_config.bulk_operation_max_count()
            ),
            min_value: bulk_config.bulk_operation_min_count(),
            max_value: bulk_config.bulk_operation_max_count(),
        }))
    }

    /// Handle updating the repeat count (internal message)
    pub fn handle_update_repeat_count(&mut self, count: usize) -> Option<Msg> {
        self.queue_state_mut().message_repeat_count = count;
        self.handle_compose_new_message()
    }

    /// Handle successful message sending by auto-reload only for empty queues
    pub fn handle_messages_sent_successfully(&mut self) -> Option<Msg> {
        let current_messages = self
            .queue_state()
            .message_pagination
            .get_current_page_messages(config::get_config_or_panic().max_messages());

        // Only auto-reload if the queue appears to be empty (0 messages shown)
        // This is the only case where auto-reload makes sense with Azure Service Bus
        if current_messages.len() < config::get_config_or_panic().max_messages() as usize {
            log::info!("Queue appears empty, auto-reloading to show newly sent messages");

            self.reset_pagination_state();
            self.load_messages_from_beginning().err().map(Msg::Error)
        } else {
            log::info!(
                "Queue has {} messages, skipping auto-reload (newly sent messages may not be visible due to Azure Service Bus message ordering)",
                current_messages.len()
            );
            None
        }
    }

    /// Load messages from the beginning (fresh start), similar to initial queue load
    pub fn load_messages_from_beginning(&self) -> Result<(), AppError> {
        let tx_to_main = self.state_manager.tx_to_main.clone();
        let service_bus_manager = self.service_bus_manager.clone();

        self.task_manager
            .execute("Loading messages...", async move {
                let result =
                    Self::execute_fresh_message_load(tx_to_main.clone(), service_bus_manager).await;
                if let Err(e) = &result {
                    log::error!("Error loading messages from beginning: {e}");
                }
                result
            });

        Ok(())
    }

    /// Execute fresh message loading from the beginning
    async fn execute_fresh_message_load(
        tx_to_main: Sender<Msg>,
        service_bus_manager: std::sync::Arc<
            tokio::sync::Mutex<server::service_bus_manager::ServiceBusManager>,
        >,
    ) -> Result<(), AppError> {
        use server::service_bus_manager::{ServiceBusCommand, ServiceBusResponse};

        let command = ServiceBusCommand::PeekMessages {
            max_count: config::get_config_or_panic().max_messages(),
            from_sequence: None,
        };

        let response = service_bus_manager
            .lock()
            .await
            .execute_command(command)
            .await;

        let messages = match response {
            ServiceBusResponse::MessagesReceived { messages } => messages,
            ServiceBusResponse::Error { error } => {
                return Err(AppError::ServiceBus(format!(
                    "Failed to peek messages from beginning: {}",
                    error
                )));
            }
            _ => {
                return Err(AppError::ServiceBus(
                    "Unexpected response for peek messages".to_string(),
                ));
            }
        };

        log::info!("Loaded {} messages from beginning", messages.len());

        let activity_msg = if messages.is_empty() {
            MessageActivityMsg::MessagesLoaded(messages)
        } else {
            MessageActivityMsg::NewMessagesLoaded(messages)
        };

        tx_to_main
            .send(Msg::MessageActivity(activity_msg))
            .map_err(|e| AppError::Component(e.to_string()))?;

        Ok(())
    }
}
