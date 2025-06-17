use super::Model;
use crate::components::common::{MessageActivityMsg, Msg, NamespaceActivityMsg, QueueActivityMsg};
use crate::config::CONFIG;
use crate::error::AppError;
use azservicebus::ServiceBusReceiverOptions;
use server::consumer::Consumer;
use server::consumer::ServiceBusClientExt;
use server::service_bus_manager::ServiceBusManager;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use tokio::sync::Mutex;
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

                let namespaces = ServiceBusManager::list_namespaces_azure_ad(CONFIG.azure_ad())
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
        let selected_namespace = self.selected_namespace.clone();
        let tx_to_main = self.tx_to_main.clone();

        self.task_manager.execute(
            format!(
                "Loading queues from {}...",
                selected_namespace
                    .clone()
                    .unwrap_or_else(|| "default".to_string())
            ),
            async move {
                let mut config = CONFIG.azure_ad().clone();
                if let Some(ns) = selected_namespace {
                    log::debug!("Using namespace: {}", ns);
                    config.namespace = Some(ns);
                } else {
                    log::warn!("No namespace selected, using default namespace");
                }

                log::debug!("Requesting queues from Azure AD");

                let queues = ServiceBusManager::list_queues_azure_ad(&config)
                    .await
                    .map_err(|e| {
                        log::error!("Failed to list queues: {}", e);
                        AppError::ServiceBus(e.to_string())
                    })?;

                log::info!(
                    "Loaded {} queues from namespace {}",
                    queues.len(),
                    config.namespace()
                );

                // Send loaded queues
                if let Err(e) =
                    tx_to_main.send(Msg::QueueActivity(QueueActivityMsg::QueuesLoaded(queues)))
                {
                    log::error!("Failed to send queues loaded message: {}", e);
                    return Err(AppError::Component(e.to_string()));
                }

                Ok(())
            },
        );
    }

    /// Create new consumer for queue using TaskManager
    pub fn new_consumer_for_queue(&mut self) {
        // Extract the queue from the mutable reference to self
        let queue = self
            .queue_state
            .pending_queue
            .take()
            .expect("No queue selected");
        log::info!("Creating consumer for queue: {}", queue);

        // Store the queue name to update current_queue_name when consumer is created
        let queue_name_for_update = queue.clone();
        let service_bus_client = self.service_bus_client.clone();
        let consumer = self.queue_state.consumer.clone();
        let tx_to_main = self.tx_to_main.clone();

        self.task_manager
            .execute(format!("Connecting to queue {}...", queue), async move {
                if let Some(consumer) = consumer {
                    log::debug!("Disposing existing consumer");
                    if let Err(e) = consumer.lock().await.dispose().await {
                        log::error!("Failed to dispose consumer: {}", e);
                        return Err(AppError::ServiceBus(e.to_string()));
                    }
                }

                log::debug!("Acquiring service bus client lock");
                let mut client = service_bus_client.lock().await;
                log::debug!("Creating receiver for queue: {}", queue);
                let consumer = client
                    .create_consumer_for_queue(queue.clone(), ServiceBusReceiverOptions::default())
                    .await
                    .map_err(|e| {
                        log::error!("Failed to create consumer for queue {}: {}", queue, e);
                        AppError::ServiceBus(e.to_string())
                    })?;

                log::info!("Successfully created consumer for queue: {}", queue);

                // Send consumer created message
                if let Err(e) = tx_to_main.send(Msg::MessageActivity(
                    MessageActivityMsg::ConsumerCreated(consumer),
                )) {
                    log::error!("Failed to send consumer created message: {}", e);
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

    /// Load messages using TaskManager
    pub fn load_messages(&self) {
        let tx_to_main = self.tx_to_main.clone();
        let consumer = match self.queue_state.consumer.clone() {
            Some(consumer) => consumer,
            None => {
                log::error!("No consumer available");
                self.error_reporter.report_simple(
                    AppError::State("No consumer available".to_string()),
                    "MessageLoader",
                    "load_messages",
                );
                return;
            }
        };

        self.task_manager
            .execute("Loading messages...", async move {
                let result =
                    Self::execute_message_loading_task(tx_to_main.clone(), consumer, None).await;
                if let Err(e) = result {
                    log::error!("Error in message loading task: {}", e);
                    return Err(e);
                }
                Ok(())
            });
    }

    /// Execute message loading task asynchronously
    async fn execute_message_loading_task(
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

        log::info!("Loaded {} messages", messages.len());

        // Send initial messages as new messages loaded
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
            // No messages, but still need to update the view
            tx_to_main
                .send(Msg::MessageActivity(MessageActivityMsg::MessagesLoaded(
                    messages,
                )))
                .map_err(|e| {
                    log::error!("Failed to send messages loaded message: {}", e);
                    AppError::Component(e.to_string())
                })?;
        }

        Ok(())
    }
}
