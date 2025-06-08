use crate::app::model::{AppState, Model};
use crate::components::common::{
    ComponentId, LoadingActivityMsg, MessageActivityMsg, Msg, PopupActivityMsg,
};
use crate::config::CONFIG;
use crate::error::AppError;
use server::consumer::Consumer;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tuirealm::terminal::TerminalAdapter;

// Bulk send configuration is now handled via the config system

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Handle opening empty message details in edit mode for composition
    pub fn handle_compose_new_message(&mut self) -> Option<Msg> {
        if let Err(e) = self.remount_messages_with_focus(false) {
            return Some(Msg::Error(e));
        }

        self.app_state = AppState::MessageDetails;

        if let Err(e) = self.app.active(&ComponentId::MessageDetails) {
            log::error!("Failed to activate message details: {}", e);
        }

        if let Err(e) = self.remount_message_details_for_composition() {
            return Some(Msg::Error(e));
        }

        self.is_editing_message = true;
        if let Err(e) = self.update_global_key_watcher_editing_state() {
            log::error!("Failed to update global key watcher: {}", e);
        }

        Some(Msg::ForceRedraw)
    }

    /// Handle setting the repeat count for bulk message sending
    pub fn handle_set_message_repeat_count(&self) -> Option<Msg> {
        let bulk_config = CONFIG.bulk_operations();

        Some(Msg::PopupActivity(PopupActivityMsg::ShowNumberInput {
            title: "Bulk Send Message".to_string(),
            message: format!(
                "How many times should the message be sent?\n(Limit: {}-{})",
                bulk_config.min_count(),
                bulk_config.max_count()
            ),
            min_value: bulk_config.min_count(),
            max_value: bulk_config.max_count(),
        }))
    }

    /// Handle updating the repeat count (internal message)
    pub fn handle_update_repeat_count(&mut self, count: usize) -> Option<Msg> {
        self.queue_state.message_repeat_count = count;
        self.handle_compose_new_message()
    }

    /// Handle successful message sending by auto-reload only for empty queues
    pub fn handle_messages_sent_successfully(&mut self) -> Option<Msg> {
        let current_messages = self
            .queue_state
            .message_pagination
            .get_current_page_messages(CONFIG.max_messages());

        // Only auto-reload if the queue appears to be empty (0 messages shown)
        // This is the only case where auto-reload makes sense with Azure Service Bus
        if current_messages.len() < CONFIG.max_messages() as usize {
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
        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Start(
            "Loading messages...".to_string(),
        ))) {
            log::error!("Failed to send loading start message: {e}");
        }

        let consumer = self.queue_state.consumer.clone().ok_or_else(|| {
            log::error!("No consumer available");
            AppError::State("No consumer available".to_string())
        })?;

        let tx_to_main_err = tx_to_main.clone();
        taskpool.execute(async move {
            let result = Self::execute_fresh_message_load(tx_to_main.clone(), consumer).await;

            if let Err(e) = result {
                log::error!("Error loading messages from beginning: {e}");

                if let Err(err) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Stop)) {
                    log::error!("Failed to send loading stop message: {err}");
                }

                let _ = tx_to_main_err.send(Msg::Error(e));
            }
        });

        Ok(())
    }

    /// Execute fresh message loading from the beginning
    async fn execute_fresh_message_load(
        tx_to_main: Sender<Msg>,
        consumer: Arc<Mutex<Consumer>>,
    ) -> Result<(), AppError> {
        let mut consumer = consumer.lock().await;

        let messages = consumer
            .peek_messages(CONFIG.max_messages(), None)
            .await
            .map_err(|e| {
                log::error!("Failed to peek messages from beginning: {e}");
                AppError::ServiceBus(e.to_string())
            })?;

        log::info!("Loaded {} messages from beginning", messages.len());

        if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Stop)) {
            log::error!("Failed to send loading stop message: {e}");
        }

        let activity_msg = if messages.is_empty() {
            MessageActivityMsg::MessagesLoaded(messages)
        } else {
            MessageActivityMsg::NewMessagesLoaded(messages)
        };

        tx_to_main
            .send(Msg::MessageActivity(activity_msg))
            .map_err(|e| {
                log::error!("Failed to send messages loaded message: {e}");
                AppError::Component(e.to_string())
            })?;

        Ok(())
    }
}
