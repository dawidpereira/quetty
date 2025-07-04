use crate::app::queue_state::QueueState;
use crate::app::task_manager::TaskManager;
use crate::components::common::{MessageActivityMsg, Msg, NamespaceActivityMsg, QueueActivityMsg};
use crate::config;
use crate::error::AppError;
use server::service_bus_manager::ServiceBusManager;
use server::service_bus_manager::{QueueType, ServiceBusCommand, ServiceBusResponse};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use tokio::sync::Mutex;

/// Manages queue operations and queue state
pub struct QueueManager {
    pub queue_state: QueueState,
    service_bus_manager: Arc<Mutex<ServiceBusManager>>,
    task_manager: TaskManager,
    tx_to_main: Sender<Msg>,
}

impl QueueManager {
    /// Create a new QueueManager
    pub fn new(
        service_bus_manager: Arc<Mutex<ServiceBusManager>>,
        task_manager: TaskManager,
        tx_to_main: Sender<Msg>,
    ) -> Self {
        Self {
            queue_state: QueueState::new(),
            service_bus_manager,
            task_manager,
            tx_to_main,
        }
    }

    /// Load namespaces using TaskManager with timeout
    pub fn load_namespaces(&self) {
        let tx_to_main = self.tx_to_main.clone();

        // Use execute with built-in timeout for namespace loading
        self.task_manager
            .execute("Loading namespaces...", async move {
                log::debug!("Requesting namespaces from Azure AD");

                let namespaces = ServiceBusManager::list_namespaces_azure_ad(
                    config::get_config_or_panic().azure_ad(),
                )
                .await
                .map_err(|e| {
                    log::error!("Failed to list namespaces: {e}");
                    AppError::ServiceBus(e.to_string())
                })?;

                log::info!("Loaded {} namespaces", namespaces.len());

                // Send loaded namespaces
                if let Err(e) = tx_to_main.send(Msg::NamespaceActivity(
                    NamespaceActivityMsg::NamespacesLoaded(namespaces),
                )) {
                    log::error!("Failed to send namespaces loaded message: {e}");
                    return Err(AppError::Component(e.to_string()));
                }

                Ok(())
            });
    }

    /// Load queues using TaskManager with timeout
    pub fn load_queues(&self) {
        let tx_to_main = self.tx_to_main.clone();

        // Use execute with built-in timeout for queue loading
        self.task_manager.execute("Loading queues...", async move {
            log::debug!("Requesting queues from Azure AD");

            let queues =
                ServiceBusManager::list_queues_azure_ad(config::get_config_or_panic().azure_ad())
                    .await
                    .map_err(|e| {
                        log::error!("Failed to list queues: {e}");
                        AppError::ServiceBus(e.to_string())
                    })?;

            log::info!("Loaded {} queues", queues.len());

            // Send loaded queues
            if let Err(e) =
                tx_to_main.send(Msg::QueueActivity(QueueActivityMsg::QueuesLoaded(queues)))
            {
                log::error!("Failed to send queues loaded message: {e}");
                return Err(AppError::Component(e.to_string()));
            }

            Ok(())
        });
    }

    /// Switch to a new queue
    pub fn switch_to_queue(&mut self, queue_name: String) {
        // Store the queue name for later use
        self.queue_state.pending_queue = Some(queue_name.clone());

        log::info!("Switching to queue: {queue_name}");

        let service_bus_manager = self.service_bus_manager.clone();
        let tx_to_main = self.tx_to_main.clone();
        let queue_name_for_update = queue_name.clone();

        // Determine the correct queue type from the queue name
        let queue_type = QueueType::from_queue_name(&queue_name);

        // Generate unique operation ID for cancellation support
        let operation_id = format!(
            "switch_queue_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );

        self.task_manager.execute_with_progress(
            format!("Connecting to queue {queue_name}..."),
            operation_id,
            move |progress: crate::app::task_manager::ProgressReporter| {
                Box::pin(async move {
                    log::debug!("Switching to queue: {queue_name} (type: {queue_type:?})");

                    progress.report_progress("Establishing connection...");

                    // Use the service bus manager to switch queues with correct type
                    let command = ServiceBusCommand::SwitchQueue {
                        queue_name: queue_name.clone(),
                        queue_type,
                    };

                    progress.report_progress("Switching to queue...");

                    let mgr_lock = service_bus_manager.lock().await;
                    let response = mgr_lock.execute_command(command).await;

                    let queue_info = match response {
                        ServiceBusResponse::QueueSwitched { queue_info } => {
                            progress.report_progress("Queue switch successful!");
                            log::info!("Successfully switched to queue: {}", queue_info.name);
                            queue_info
                        }
                        ServiceBusResponse::Error { error } => {
                            log::error!("Failed to switch to queue {queue_name}: {error}");
                            return Err(AppError::ServiceBus(error.to_string()));
                        }
                        _ => {
                            return Err(AppError::ServiceBus(
                                "Unexpected response for switch queue".to_string(),
                            ));
                        }
                    };

                    progress.report_progress("Updating UI state...");

                    // Send queue switched message via MessageActivity
                    if let Err(e) = tx_to_main.send(Msg::MessageActivity(
                        MessageActivityMsg::QueueSwitched(queue_info),
                    )) {
                        log::error!("Failed to send queue switched message: {e}");
                        return Err(AppError::Component(e.to_string()));
                    }

                    // Send a separate message to update the current queue name
                    if let Err(e) = tx_to_main.send(Msg::MessageActivity(
                        MessageActivityMsg::QueueNameUpdated(queue_name_for_update),
                    )) {
                        log::error!("Failed to send queue name updated message: {e}");
                        return Err(AppError::Component(e.to_string()));
                    }

                    Ok(())
                })
            },
        );
    }
}
