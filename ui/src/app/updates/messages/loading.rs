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
        // Check if already loading to prevent concurrent operations
        if self.queue_state().message_pagination.is_loading() {
            log::debug!("Already loading messages, skipping request");
            return Ok(());
        }

        log::debug!(
            "Loading {} messages from API, last_sequence: {:?}",
            message_count,
            self.queue_manager
                .queue_state
                .message_pagination
                .last_loaded_sequence
        );

        // Set loading state
        self.queue_state_mut().message_pagination.set_loading(true);

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

                // Always send a message to clear loading state, even on error
                if let Err(e) = &result {
                    log::error!("Error in message loading task: {e}");
                    // Send empty message list to clear loading state
                    let _ = Self::send_loaded_messages(&tx_to_main, Vec::new());
                }
                result
            });

        Ok(())
    }

    pub(crate) fn get_service_bus_manager(
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

                // Debug: log sequence range of received messages
                if !messages.is_empty() {
                    let first_seq = messages.first().unwrap().sequence;
                    let last_seq = messages.last().unwrap().sequence;
                    log::debug!(
                        "Received messages with sequences: {} to {} (count: {})",
                        first_seq,
                        last_seq,
                        messages.len()
                    );

                    // Check for gaps in sequences
                    let expected_count = (last_seq - first_seq + 1) as usize;
                    if messages.len() != expected_count {
                        log::warn!(
                            "Sequence gap detected! Expected {} messages between {} and {}, got {}",
                            expected_count,
                            first_seq,
                            last_seq,
                            messages.len()
                        );
                    }
                }

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

        // Always send the result, even if empty, so loading state gets cleared
        Self::send_loaded_messages(&tx_to_main, messages)?;

        Ok(())
    }

    fn send_loaded_messages(
        tx_to_main: &Sender<Msg>,
        messages: Vec<MessageModel>,
    ) -> Result<(), AppError> {
        // Always send NewMessagesLoaded, even if empty, so pagination can track end-of-queue
        tx_to_main
            .send(Msg::MessageActivity(MessageActivityMsg::NewMessagesLoaded(
                messages,
            )))
            .map_err(|e| {
                log::error!("Failed to send new messages loaded message: {e}");
                AppError::Component(e.to_string())
            })?;
        Ok(())
    }
}
