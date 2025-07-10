use crate::app::managers::state_manager::NavigationContext;
use crate::app::queue_state::QueueState;
use crate::app::task_manager::TaskManager;
use crate::components::common::{MessageActivityMsg, Msg, NamespaceActivityMsg, QueueActivityMsg};
use crate::config;
use crate::error::AppError;
use crate::utils::auth::AuthUtils;
use crate::utils::connection_string::ConnectionStringParser;
use server::service_bus_manager::ServiceBusManager;
use server::service_bus_manager::{QueueType, ServiceBusCommand, ServiceBusResponse};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use tokio::sync::Mutex;

/// Manages queue operations and queue state
pub struct QueueManager {
    pub queue_state: QueueState,
    service_bus_manager: Option<Arc<Mutex<ServiceBusManager>>>,
    task_manager: TaskManager,
    tx_to_main: Sender<Msg>,
}

impl QueueManager {
    /// Create a new QueueManager
    pub fn new(
        service_bus_manager: Option<Arc<Mutex<ServiceBusManager>>>,
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

    /// Set the service bus manager after discovery
    pub fn set_service_bus_manager(&mut self, manager: Arc<Mutex<ServiceBusManager>>) {
        self.service_bus_manager = Some(manager);
    }

    /// Load namespaces using TaskManager with timeout
    pub fn load_namespaces(&self, navigation_context: NavigationContext) {
        let config = config::get_config_or_panic();

        if AuthUtils::is_connection_string_auth(config) {
            self.load_namespaces_from_connection_string();
        } else {
            self.load_namespaces_from_azure_ad(navigation_context);
        }
    }

    /// Load namespaces from connection string authentication
    fn load_namespaces_from_connection_string(&self) {
        let config = config::get_config_or_panic();

        log::info!(
            "Using connection string authentication - extracting namespace from connection string"
        );

        // Check if we have encrypted connection string configured
        if !config.servicebus().has_connection_string() {
            log::error!("No encrypted connection string configured");
            self.send_namespaces_loaded(vec![]);
            return;
        }

        // Check if master password is set
        if !crate::config::azure::is_master_password_set() {
            log::error!("Master password not set - cannot decrypt connection string");
            self.send_namespaces_loaded(vec![]);
            return;
        }

        log::info!("Master password is available, attempting to decrypt connection string");

        match config.servicebus().connection_string() {
            Ok(Some(connection_string)) => {
                log::info!(
                    "Successfully decrypted connection string (length: {} chars)",
                    connection_string.len()
                );

                match ConnectionStringParser::extract_namespace(&connection_string) {
                    Ok(namespace) => {
                        log::info!(
                            "Successfully extracted namespace from connection string: '{namespace}'"
                        );
                        log::info!("Sending namespace list with 1 item to trigger auto-selection");
                        self.send_namespaces_loaded(vec![namespace]);
                    }
                    Err(e) => {
                        log::error!("Failed to extract namespace from connection string: {e}");
                        log::error!(
                            "This means the connection string format is invalid or corrupted"
                        );
                        self.send_namespaces_loaded(vec![]);
                    }
                }
            }
            Ok(None) => {
                log::error!(
                    "Connection string decryption returned None - this shouldn't happen if has_connection_string() returned true"
                );
                self.send_namespaces_loaded(vec![]);
            }
            Err(e) => {
                log::error!("Failed to decrypt connection string: {e}");
                log::error!(
                    "This likely means the master password is incorrect or encryption data is corrupted"
                );
                self.send_namespaces_loaded(vec![]);
            }
        }
    }

    /// Load namespaces from Azure AD authentication
    fn load_namespaces_from_azure_ad(&self, navigation_context: NavigationContext) {
        let tx_to_main = self.tx_to_main.clone();

        self.task_manager
            .execute("Loading namespaces...", async move {
                log::debug!("Starting namespace loading with navigation context: {navigation_context:?}");

                // Check if we should use saved namespace or force discovery
                let should_use_saved_config = matches!(navigation_context, NavigationContext::Startup);
                if should_use_saved_config {
                    // Only use saved namespace during startup auto-progression
                    if let Ok(saved_namespace) = std::env::var("AZURE_AD__NAMESPACE") {
                        if !saved_namespace.trim().is_empty() {
                            log::info!(
                                "Startup mode: Found saved namespace '{saved_namespace}', using it directly"
                            );

                            // Send the saved namespace as a single-item list
                            if let Err(e) = tx_to_main.send(Msg::NamespaceActivity(
                                NamespaceActivityMsg::NamespacesLoaded(vec![saved_namespace]),
                            )) {
                                log::error!("Failed to send saved namespace message: {e}");
                            }
                            return Ok(());
                        }
                    }
                } else {
                    log::debug!("Navigation mode ({navigation_context:?}): forcing namespace discovery to allow user selection");
                }

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

    /// Helper method to send namespaces loaded message
    fn send_namespaces_loaded(&self, namespaces: Vec<String>) {
        if let Err(e) = self.tx_to_main.send(Msg::NamespaceActivity(
            NamespaceActivityMsg::NamespacesLoaded(namespaces),
        )) {
            log::error!("Failed to send namespace loaded message: {e}");
        }
    }

    /// Load queues using TaskManager with timeout
    pub fn load_queues(&self) {
        let config = config::get_config_or_panic();

        if AuthUtils::is_connection_string_auth(config) {
            self.load_queues_from_connection_string();
        } else {
            self.load_queues_from_azure_ad();
        }
    }

    /// Load queues from connection string authentication
    fn load_queues_from_connection_string(&self) {
        // Connection string authentication does not support automatic queue discovery
        // because Azure Management API requires Azure AD authentication, not SAS tokens.
        // Connection strings only provide namespace-level access for messaging operations.

        // Note: Queue auto-loading from saved names is now handled in AuthenticationSuccess
        // to ensure proper flow and statistics loading

        log::info!("Using connection string authentication - showing manual queue selection");
        self.send_empty_queue_list_for_manual_selection();
    }

    /// Load queues from Azure AD authentication
    fn load_queues_from_azure_ad(&self) {
        let tx_to_main = self.tx_to_main.clone();

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

    /// Send empty queue list to trigger manual queue selection UI
    fn send_empty_queue_list_for_manual_selection(&self) {
        let tx_to_main = self.tx_to_main.clone();

        self.task_manager.execute("Loading queues...", async move {
            log::debug!("Connection string auth: Returning empty queue list for manual selection");

            // Return empty queue list to show QueuePicker with manual entry option
            // This is the expected behavior for connection string authentication
            let queues = vec![];

            if let Err(e) =
                tx_to_main.send(Msg::QueueActivity(QueueActivityMsg::QueuesLoaded(queues)))
            {
                log::error!("Failed to send queues loaded message: {e}");
                return Err(AppError::Component(e.to_string()));
            }

            Ok(())
        });
    }

    /// Load queues with discovered Azure resources
    /// This method optimizes performance by using discovered Azure AD values
    /// which avoid fresh token requests and use cached authentication
    pub fn load_queues_with_discovery(
        &self,
        subscription_id: String,
        resource_group: String,
        namespace: String,
        auth_service: Arc<crate::services::AuthService>,
        http_client: reqwest::Client,
    ) {
        let tx_to_main = self.tx_to_main.clone();
        let service_bus_manager = self.service_bus_manager.clone();

        if service_bus_manager.is_none() {
            log::warn!("Service bus manager not initialized, cannot load queues with discovery");
            // Send empty queue list to show QueuePicker for manual entry
            if let Err(e) =
                tx_to_main.send(Msg::QueueActivity(QueueActivityMsg::QueuesLoaded(vec![])))
            {
                log::error!("Failed to send empty queues loaded message: {e}");
            }
            return;
        }

        self.task_manager.execute("Loading queues...", async move {
            log::debug!("Requesting queues for discovered namespace: {namespace}");

            // Create an Azure AD config with discovered values for faster queue listing
            let base_config = crate::config::get_config_or_panic().azure_ad();
            let mut enhanced_config = base_config.clone();
            enhanced_config.subscription_id = Some(subscription_id.clone());
            enhanced_config.resource_group = Some(resource_group.clone());
            enhanced_config.namespace = Some(namespace.clone());

            // Try to use the faster Azure AD method with enhanced config
            log::info!(
                "Using Azure AD method with discovered values for queue listing (optimized)"
            );

            match server::service_bus_manager::ServiceBusManager::list_queues_azure_ad(
                &enhanced_config,
            )
            .await
            {
                Ok(queue_names) => {
                    log::info!(
                        "Loaded {} queues using optimized Azure AD method",
                        queue_names.len()
                    );

                    if let Err(e) = tx_to_main.send(Msg::QueueActivity(
                        QueueActivityMsg::QueuesLoaded(queue_names),
                    )) {
                        log::error!("Failed to send queues loaded message: {e}");
                        return Err(AppError::Component(e.to_string()));
                    }
                    return Ok(());
                }
                Err(e) => {
                    log::warn!(
                        "Optimized Azure AD method failed, falling back to Management API: {e}"
                    );
                }
            }

            // Fallback to Azure Management API with fresh token
            log::info!("Using Azure Management API for queue listing (slower fallback)");

            // Get Azure AD token
            let token = match auth_service.get_management_token().await {
                Ok(token) => token,
                Err(e) => {
                    log::error!("Failed to get management token: {e}");
                    return Err(AppError::Auth(e.to_string()));
                }
            };

            // Use Azure Management API to list queues
            let client =
                server::service_bus_manager::azure_management_client::AzureManagementClient::new(
                    http_client,
                );
            let queue_names = client
                .list_queues(&token, &subscription_id, &resource_group, &namespace)
                .await
                .map_err(|e| {
                    log::error!("Failed to list queues: {e}");
                    AppError::ServiceBus(e.to_string())
                })?;

            log::info!(
                "Loaded {} queues from discovered namespace using Management API",
                queue_names.len()
            );

            // Send loaded queues
            if let Err(e) = tx_to_main.send(Msg::QueueActivity(QueueActivityMsg::QueuesLoaded(
                queue_names,
            ))) {
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

        let Some(service_bus_manager) = self.service_bus_manager.clone() else {
            log::error!("Service bus manager not initialized, cannot switch queue");
            // Clear the pending queue since we can't proceed
            self.queue_state.pending_queue = None;
            // Send error to show that queue switch failed
            if let Err(e) = self.tx_to_main.send(Msg::ShowError(
                "Service Bus Manager not initialized. Please check your authentication."
                    .to_string(),
            )) {
                log::error!("Failed to send Service Bus Manager error: {e}");
            }
            return;
        };
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
