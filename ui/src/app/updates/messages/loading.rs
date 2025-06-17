use crate::app::model::Model;
use crate::components::common::{MessageActivityMsg, Msg};
use crate::error::{AppError, AppResult};
use server::consumer::Consumer;
use server::model::MessageModel;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use tokio::sync::Mutex;
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
            self.queue_state.message_pagination.last_loaded_sequence
        );

        let tx_to_main = self.tx_to_main.clone();

        let consumer = self.get_consumer()?;
        let from_sequence = self
            .queue_state
            .message_pagination
            .last_loaded_sequence
            .map(|seq| seq + 1);

        self.task_manager
            .execute("Loading more messages...", async move {
                let result = Self::load_messages_from_consumer_with_count(
                    tx_to_main.clone(),
                    consumer,
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

    fn get_consumer(&self) -> AppResult<Arc<Mutex<Consumer>>> {
        self.queue_state.consumer.clone().ok_or_else(|| {
            log::error!("No consumer available");
            AppError::State("No consumer available".to_string())
        })
    }

    async fn load_messages_from_consumer_with_count(
        tx_to_main: Sender<Msg>,
        consumer: Arc<Mutex<Consumer>>,
        from_sequence: Option<i64>,
        message_count: u32,
    ) -> Result<(), AppError> {
        let mut consumer = consumer.lock().await;

        let messages = consumer
            .peek_messages(message_count, from_sequence)
            .await
            .map_err(|e| {
                log::error!("Failed to peek messages: {}", e);
                AppError::ServiceBus(e.to_string())
            })?;

        log::info!(
            "Loaded {} new messages from API (requested {})",
            messages.len(),
            message_count
        );

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
