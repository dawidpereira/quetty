use super::Model;
use crate::components::common::{MessageActivityMsg, Msg, NamespaceActivityMsg, QueueActivityMsg};
use crate::config;
use crate::error::AppError;
use server::service_bus_manager::ServiceBusManager;
use server::service_bus_manager::{QueueType, ServiceBusCommand, ServiceBusResponse};

use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Load namespaces using TaskManager
    pub fn load_namespaces(&self) {
        let tx_to_main = self.tx_to_main.clone();

        self.task_manager
            .execute("Loading namespaces...", async move {
                log::debug!("Requesting namespaces from Azure AD");

                let namespaces = ServiceBusManager::list_namespaces_azure_ad(
                    config::get_config_or_panic().azure_ad(),
                )
                .await
                .map_err(|e| {
                    log::error!("Failed to list namespaces: {}", e);
                    AppError::ServiceBus(e.to_string())
                })?;

                log::info!("Loaded {} namespaces", namespaces.len());

                // Send loaded namespaces
                if let Err(e) = tx_to_main.send(Msg::NamespaceActivity(
                    NamespaceActivityMsg::NamespacesLoaded(namespaces),
                )) {
                    log::error!("Failed to send namespaces loaded message: {}", e);
                    return Err(AppError::Component(e.to_string()));
                }

                Ok(())
            });
    }

    /// Load queues using TaskManager
    pub fn load_queues(&self) {
        let tx_to_main = self.tx_to_main.clone();

        self.task_manager.execute("Loading queues...", async move {
            log::debug!("Requesting queues from Azure AD");

            let queues =
                ServiceBusManager::list_queues_azure_ad(config::get_config_or_panic().azure_ad())
                    .await
                    .map_err(|e| {
                        log::error!("Failed to list queues: {}", e);
                        AppError::ServiceBus(e.to_string())
                    })?;

            log::info!("Loaded {} queues", queues.len());

            // Send loaded queues
            if let Err(e) =
                tx_to_main.send(Msg::QueueActivity(QueueActivityMsg::QueuesLoaded(queues)))
            {
                log::error!("Failed to send queues loaded message: {}", e);
                return Err(AppError::Component(e.to_string()));
            }

            Ok(())
        });
    }

    /// Create new consumer for the selected queue using TaskManager
    pub fn new_consumer_for_queue(&mut self) {
        // Extract the queue from the mutable reference to self
        let queue = self
            .queue_state
            .pending_queue
            .take()
            .expect("No queue selected");
        log::info!("Switching to queue: {}", queue);

        // Store the queue name to update current_queue_name when switch is complete
        let queue_name_for_update = queue.clone();
        let service_bus_manager = self.service_bus_manager.clone();
        let tx_to_main = self.tx_to_main.clone();

        // Determine the correct queue type from the queue name
        let queue_type = QueueType::from_queue_name(&queue);

        self.task_manager
            .execute(format!("Connecting to queue {}...", queue), async move {
                log::debug!("Switching to queue: {} (type: {:?})", queue, queue_type);

                // Use the service bus manager to switch queues with correct type
                let command = ServiceBusCommand::SwitchQueue {
                    queue_name: queue.clone(),
                    queue_type,
                };

                let response = service_bus_manager
                    .lock()
                    .await
                    .execute_command(command)
                    .await;

                let queue_info = match response {
                    ServiceBusResponse::QueueSwitched { queue_info } => {
                        log::info!("Successfully switched to queue: {}", queue_info.name);
                        queue_info
                    }
                    ServiceBusResponse::Error { error } => {
                        log::error!("Failed to switch to queue {}: {}", queue, error);
                        return Err(AppError::ServiceBus(error.to_string()));
                    }
                    _ => {
                        return Err(AppError::ServiceBus(
                            "Unexpected response for switch queue".to_string(),
                        ));
                    }
                };

                // Send queue switched message (equivalent to consumer created)
                if let Err(e) = tx_to_main.send(Msg::MessageActivity(
                    MessageActivityMsg::QueueSwitched(queue_info),
                )) {
                    log::error!("Failed to send queue switched message: {}", e);
                    return Err(AppError::Component(e.to_string()));
                }

                // Send a separate message to update the current queue name
                if let Err(e) = tx_to_main.send(Msg::MessageActivity(
                    MessageActivityMsg::QueueNameUpdated(queue_name_for_update),
                )) {
                    log::error!("Failed to send queue name updated message: {}", e);
                    return Err(AppError::Component(e.to_string()));
                }

                Ok(())
            });
    }

    /// Load messages from current queue using TaskManager
    pub fn load_messages(&self) {
        let service_bus_manager = self.service_bus_manager.clone();
        let tx_to_main = self.tx_to_main.clone();
        let max_messages = config::get_config_or_panic().max_messages();

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

    /// Force reload messages - useful after bulk operations that modify the queue
    pub fn handle_force_reload_messages(&mut self) -> Option<Msg> {
        log::info!("Force reloading messages after bulk operation - resetting pagination state");

        // Reset pagination state to clear all existing messages and start fresh
        self.reset_pagination_state();

        // Load fresh messages from the beginning
        self.load_messages();

        None
    }
}
