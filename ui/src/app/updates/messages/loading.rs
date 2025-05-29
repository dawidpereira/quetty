use crate::app::model::Model;
use crate::components::common::{LoadingActivityMsg, Msg, MessageActivityMsg};
use crate::config::CONFIG;
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
    pub fn load_new_messages_from_api(&mut self) -> AppResult<()> {
        log::debug!(
            "Loading new messages from API, last_sequence: {:?}",
            self.queue_state.message_pagination.last_loaded_sequence
        );

        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        self.send_loading_start_message(&tx_to_main);

        let consumer = self.get_consumer()?;
        let tx_to_main_err = tx_to_main.clone();
        let from_sequence = self
            .queue_state
            .message_pagination
            .last_loaded_sequence
            .map(|seq| seq + 1);

        taskpool.execute(async move {
            Self::execute_message_loading_task(tx_to_main, tx_to_main_err, consumer, from_sequence)
                .await;
        });

        Ok(())
    }

    fn send_loading_start_message(&self, tx_to_main: &Sender<Msg>) {
        if let Err(e) = tx_to_main.send(Msg::LoadingActivity(
            LoadingActivityMsg::Start(
                "Loading more messages...".to_string(),
            ),
        )) {
            log::error!("Failed to send loading start message: {}", e);
        }
    }

    fn get_consumer(&self) -> AppResult<Arc<Mutex<Consumer>>> {
        self.queue_state.consumer.clone().ok_or_else(|| {
            log::error!("No consumer available");
            AppError::State("No consumer available".to_string())
        })
    }

    async fn execute_message_loading_task(
        tx_to_main: Sender<Msg>,
        tx_to_main_err: Sender<Msg>,
        consumer: Arc<Mutex<Consumer>>,
        from_sequence: Option<i64>,
    ) {
        let result =
            Self::load_messages_from_consumer(tx_to_main.clone(), consumer, from_sequence).await;

        if let Err(e) = result {
            Self::handle_loading_error(tx_to_main, tx_to_main_err, e);
        }
    }

    async fn load_messages_from_consumer(
        tx_to_main: Sender<Msg>,
        consumer: Arc<Mutex<Consumer>>,
        from_sequence: Option<i64>,
    ) -> Result<(), AppError> {
        let mut consumer = consumer.lock().await;

        let messages = consumer
            .peek_messages(CONFIG.max_messages(), from_sequence)
            .await
            .map_err(|e| {
                log::error!("Failed to peek messages: {}", e);
                AppError::ServiceBus(e.to_string())
            })?;

        log::info!("Loaded {} new messages from API", messages.len());

        Self::send_loading_stop_message(&tx_to_main);
        Self::send_loaded_messages(&tx_to_main, messages)?;

        Ok(())
    }

    fn send_loading_stop_message(tx_to_main: &Sender<Msg>) {
        if let Err(e) = tx_to_main.send(Msg::LoadingActivity(
            LoadingActivityMsg::Stop,
        )) {
            log::error!("Failed to send loading stop message: {}", e);
        }
    }

    fn send_loaded_messages(
        tx_to_main: &Sender<Msg>,
        messages: Vec<MessageModel>,
    ) -> Result<(), AppError> {
        if !messages.is_empty() {
            tx_to_main
                .send(Msg::MessageActivity(
                    MessageActivityMsg::NewMessagesLoaded(messages),
                ))
                .map_err(|e| {
                    log::error!("Failed to send new messages loaded message: {}", e);
                    AppError::Component(e.to_string())
                })?;
        } else {
            Self::send_page_changed_fallback(tx_to_main)?;
        }
        Ok(())
    }

    fn send_page_changed_fallback(
        tx_to_main: &Sender<Msg>,
    ) -> Result<(), AppError> {
        tx_to_main
            .send(Msg::MessageActivity(
                MessageActivityMsg::PageChanged,
            ))
            .map_err(|e| {
                log::error!("Failed to send page changed message: {}", e);
                AppError::Component(e.to_string())
            })
    }

    fn handle_loading_error(
        tx_to_main: Sender<Msg>,
        tx_to_main_err: Sender<Msg>,
        error: AppError,
    ) {
        log::error!("Error in message loading task: {}", error);

        Self::send_loading_stop_message(&tx_to_main);
        let _ = tx_to_main_err.send(Msg::Error(error));
    }
} 
 