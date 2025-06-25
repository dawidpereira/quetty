use crate::app::model::Model;
use crate::components::common::{MessageActivityMsg, Msg};
use crate::error::{AppError, AppResult};
use server::model::MessageModel;
use std::sync::mpsc::Sender;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Load a specific count of messages from API
    pub fn load_messages_from_api_with_count(&mut self, message_count: u32) -> AppResult<()> {
        log::debug!(
            "Loading {} messages from API, last_sequence: {:?}",
            message_count,
            self.queue_manager.queue_state.message_pagination.last_loaded_sequence
        );

        let tx_to_main = self.state_manager.tx_to_main.clone();

        let service_bus_manager = self.get_service_bus_manager();
        let from_sequence = self
            .queue_state()
            .message_pagination
            .last_loaded_sequence
            .map(|seq| seq + 1);

        self.task_manager
            .execute("Loading more messages...", async move {
                let result = Self::execute_loading_task(
                    tx_to_main.clone(),
                    service_bus_manager,
                    from_sequence,
                    message_count,
                )
                .await;

                if let Err(e) = &result {
                    log::error!("Error in message loading task: {}", e);
                }
                result
            });

        Ok(())
    }

    fn get_service_bus_manager(
        &self,
    ) -> std::sync::Arc<tokio::sync::Mutex<server::service_bus_manager::ServiceBusManager>> {
        self.service_bus_manager.clone()
    }

    async fn execute_loading_task(
        tx_to_main: Sender<Msg>,
        service_bus_manager: std::sync::Arc<
            tokio::sync::Mutex<server::service_bus_manager::ServiceBusManager>,
        >,
        from_sequence: Option<i64>,
        message_count: u32,
    ) -> Result<(), AppError> {
        use server::service_bus_manager::{ServiceBusCommand, ServiceBusResponse};

        let command = ServiceBusCommand::PeekMessages {
            max_count: message_count,
            from_sequence,
        };

        let response = service_bus_manager
            .lock()
            .await
            .execute_command(command)
            .await;

        let messages = match response {
            ServiceBusResponse::MessagesReceived { messages } => {
                log::info!("Loaded {} new messages from API", messages.len());
                messages
            }
            ServiceBusResponse::Error { error } => {
                return Err(AppError::ServiceBus(error.to_string()));
            }
            _ => {
                return Err(AppError::ServiceBus(
                    "Unexpected response for peek messages".to_string(),
                ));
            }
        };

        Self::send_loaded_messages(&tx_to_main, messages)?;

        Ok(())
    }

    fn send_loaded_messages(
        tx_to_main: &Sender<Msg>,
        messages: Vec<MessageModel>,
    ) -> Result<(), AppError> {
        if !messages.is_empty() {
            tx_to_main
                .send(Msg::MessageActivity(MessageActivityMsg::NewMessagesLoaded(
                    messages,
                )))
                .map_err(|e| {
                    log::error!("Failed to send new messages loaded message: {}", e);
                    AppError::Component(e.to_string())
                })?;
        } else {
            Self::send_page_changed_fallback(tx_to_main)?;
        }
        Ok(())
    }

    fn send_page_changed_fallback(tx_to_main: &Sender<Msg>) -> Result<(), AppError> {
        tx_to_main
            .send(Msg::MessageActivity(MessageActivityMsg::PageChanged))
            .map_err(|e| {
                log::error!("Failed to send page changed message: {}", e);
                AppError::Component(e.to_string())
            })
    }
}
