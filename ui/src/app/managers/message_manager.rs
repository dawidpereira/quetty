use crate::app::task_manager::TaskManager;
use crate::components::common::{MessageActivityMsg, Msg};
use crate::config;
use crate::error::AppError;
use server::service_bus_manager::{ServiceBusCommand, ServiceBusManager, ServiceBusResponse};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use tokio::sync::Mutex;

/// Manages message operations and message state
pub struct MessageManager {
    service_bus_manager: Arc<Mutex<ServiceBusManager>>,
    task_manager: TaskManager,
    tx_to_main: Sender<Msg>,
}

impl MessageManager {
    /// Create a new MessageManager
    pub fn new(
        service_bus_manager: Arc<Mutex<ServiceBusManager>>,
        task_manager: TaskManager,
        tx_to_main: Sender<Msg>,
    ) -> Self {
        Self {
            service_bus_manager,
            task_manager,
            tx_to_main,
        }
    }

    /// Load messages from current queue using TaskManager
    pub fn load_messages(&self) {
        let service_bus_manager = self.service_bus_manager.clone();
        let tx_to_main = self.tx_to_main.clone();
        // Use the dynamic page size that can be changed during runtime
        let max_messages = config::get_current_page_size();

        self.task_manager
            .execute("Loading messages...", async move {
                log::debug!("Loading messages from current queue");

                // Peek messages from the current queue
                let command = ServiceBusCommand::PeekMessages {
                    max_count: max_messages,
                    from_sequence: None,
                };

                let response = service_bus_manager
                    .lock()
                    .await
                    .execute_command(command)
                    .await;

                let messages = match response {
                    ServiceBusResponse::MessagesReceived { messages } => {
                        log::info!("Loaded {} messages", messages.len());
                        messages
                    }
                    ServiceBusResponse::Error { error } => {
                        log::error!("Failed to load messages: {}", error);
                        return Err(AppError::ServiceBus(error.to_string()));
                    }
                    _ => {
                        return Err(AppError::ServiceBus(
                            "Unexpected response for peek messages".to_string(),
                        ));
                    }
                };

                // Send loaded messages
                if let Err(e) = tx_to_main.send(Msg::MessageActivity(
                    MessageActivityMsg::MessagesLoaded(messages),
                )) {
                    log::error!("Failed to send messages loaded: {}", e);
                    return Err(AppError::Component(e.to_string()));
                }

                Ok(())
            });
    }

    /// Force reload messages - useful after operations that modify the queue
    pub fn force_reload_messages(&self) {
        log::info!("Force reloading messages after bulk operation");
        self.load_messages();
    }
}
