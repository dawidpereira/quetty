use super::Model;
use crate::app::managers::{QueueManager, StateManager};

use crate::app::queue_state::QueueState;
use crate::app::task_manager::TaskManager;
use crate::components::common::{ComponentId, Msg};
use crate::components::global_key_watcher::GlobalKeyWatcher;
use crate::components::loading_indicator::LoadingIndicator;
use crate::components::message_details::MessageDetails;
use crate::components::messages::Messages;
use crate::components::namespace_picker::NamespacePicker;
use crate::components::queue_picker::QueuePicker;
use crate::components::state::ComponentStateMount;
use crate::components::text_label::TextLabel;
use crate::config;
use crate::error::{AppError, AppResult, ErrorReporter};
use azservicebus::{ServiceBusClient as AzureServiceBusClient, ServiceBusClientOptions};
use server::service_bus_manager::ServiceBusManager;
use server::taskpool::TaskPool;
use std::sync::Arc;
use std::sync::mpsc;
use tokio::sync::Mutex;
use tuirealm::event::NoUserEvent;
use tuirealm::terminal::{CrosstermTerminalAdapter, TerminalAdapter, TerminalBridge};
use tuirealm::{Application, EventListenerCfg, Sub, SubClause, SubEventClause};

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    fn init_app(queue_state: &QueueState) -> AppResult<Application<ComponentId, Msg, NoUserEvent>> {
        let config = config::get_config_or_panic();
        let mut app: Application<ComponentId, Msg, NoUserEvent> = Application::init(
            EventListenerCfg::default()
                .crossterm_input_listener(
                    config.crossterm_input_listener_interval(),
                    config.crossterm_input_listener_retries(),
                )
                .poll_timeout(config.poll_timeout())
                .tick_interval(config.tick_interval()),
        );
        app.mount(
            ComponentId::TextLabel,
            Box::new(TextLabel::new(
                "Quetty, the cutest queue manager <3".to_string(),
            )),
            Vec::default(),
        )
        .map_err(|e| AppError::Component(e.to_string()))?;

        app.mount(
            ComponentId::NamespacePicker,
            Box::new(NamespacePicker::new(None)),
            Vec::default(),
        )
        .map_err(|e| AppError::Component(e.to_string()))?;

        app.mount(
            ComponentId::QueuePicker,
            Box::new(QueuePicker::new(None)),
            Vec::default(),
        )
        .map_err(|e| AppError::Component(e.to_string()))?;

        app.mount(
            ComponentId::Messages,
            Box::new(Messages::new(queue_state.messages.as_ref())),
            Vec::default(),
        )
        .map_err(|e| AppError::Component(e.to_string()))?;

        // Initialize MessageDetails with ComponentState pattern using extension trait
        app.mount_with_state(
            ComponentId::MessageDetails,
            MessageDetails::new(None),
            Vec::default(),
        )?;

        app.mount(
            ComponentId::GlobalKeyWatcher,
            Box::new(GlobalKeyWatcher::default()),
            vec![Sub::new(SubEventClause::Any, SubClause::Always)],
        )
        .map_err(|e| AppError::Component(e.to_string()))?;

        app.active(&ComponentId::Messages)
            .map_err(|e| AppError::Component(e.to_string()))?;

        Ok(app)
    }
}

impl Model<CrosstermTerminalAdapter> {
    pub async fn new() -> AppResult<Self> {
        let config = config::get_config_or_panic();
        let needs_auth = config.auth().primary_method() == "azure_ad";

        // Create Service Bus manager
        let service_bus_manager = Self::create_service_bus_manager(config).await?;

        // Log authentication configuration
        Self::log_authentication_info(config);

        let (tx_to_main, rx_to_main) = mpsc::channel();
        let taskpool = TaskPool::new(10);

        // Initialize managers
        let (error_reporter, task_manager, state_manager, queue_manager) =
            Self::initialize_managers(
                service_bus_manager.clone(),
                taskpool.clone(),
                tx_to_main.clone(),
            );

        // Setup authentication if needed
        let auth_service = Self::setup_authentication(config, tx_to_main.clone())?;

        let queue_state = QueueState::new();
        let mut app = Self {
            app: Self::init_app(&queue_state)?,
            terminal: TerminalBridge::init_crossterm()
                .map_err(|e| AppError::Component(e.to_string()))?,
            rx_to_main,
            taskpool,
            service_bus_manager,
            error_reporter,
            task_manager,
            state_manager,
            queue_manager,
            auth_service,
        };

        // Don't mount loading indicator if we need authentication
        if !needs_auth {
            // Initialize loading indicator with ComponentState pattern using extension trait
            app.app.mount_with_state(
                ComponentId::LoadingIndicator,
                LoadingIndicator::new("Loading...", true),
                Vec::default(),
            )?;
        }

        // Trigger initial authentication or load namespaces
        Self::trigger_initial_flow(needs_auth, &mut app)?;

        Ok(app)
    }

    /// Create Service Bus manager based on configuration
    async fn create_service_bus_manager(
        config: &crate::config::AppConfig,
    ) -> AppResult<Option<Arc<Mutex<ServiceBusManager>>>> {
        let connection_string_opt = config.servicebus().connection_string();
        let needs_auth = config.auth().primary_method() == "azure_ad";

        if let Some(connection_string) = connection_string_opt {
            // Connection string available - create the client and manager
            let azure_service_bus_client = AzureServiceBusClient::new_from_connection_string(
                connection_string,
                ServiceBusClientOptions::default(),
            )
            .await
            .map_err(|e| AppError::ServiceBus(e.to_string()))?;

            // Extract config components
            let azure_ad_config = config.azure_ad();
            let statistics_config =
                server::service_bus_manager::azure_management_client::StatisticsConfig::new(
                    config.queue_stats_display_enabled(),
                    config.queue_stats_cache_ttl_seconds(),
                    config.queue_stats_use_management_api(),
                );
            let batch_config = config.batch();

            Ok(Some(Arc::new(Mutex::new(ServiceBusManager::new(
                Arc::new(Mutex::new(azure_service_bus_client)),
                azure_ad_config.clone(),
                statistics_config,
                batch_config.clone(),
                connection_string.to_string(),
            )))))
        } else if needs_auth {
            // No connection string but Azure AD auth is configured
            // We'll create the manager later after discovery
            log::info!("No connection string configured, will discover from Azure");
            Ok(None)
        } else {
            Err(AppError::Config(
                "Either connection string or Azure AD authentication must be configured"
                    .to_string(),
            ))
        }
    }

    /// Log authentication configuration information
    fn log_authentication_info(config: &crate::config::AppConfig) {
        if config.auth().primary_method() == "azure_ad" {
            log::info!("Azure AD authentication configured for management operations");
            log::info!("Flow: {}", config.azure_ad().flow);
            if config.azure_ad().flow == "device_code" {
                log::info!("Device code flow: You'll be prompted to authenticate in your browser");
                log::info!("This will happen when accessing queue statistics or listing queues");
            }
            log::warn!(
                "Note: Service Bus message operations still use connection string due to SDK limitations"
            );
        } else {
            log::info!("Using connection string authentication");
        }
    }

    /// Initialize all required managers
    fn initialize_managers(
        service_bus_manager: Option<Arc<Mutex<ServiceBusManager>>>,
        taskpool: TaskPool,
        tx_to_main: mpsc::Sender<Msg>,
    ) -> (ErrorReporter, TaskManager, StateManager, QueueManager) {
        // Create error reporter for enhanced error handling
        let error_reporter = ErrorReporter::new(tx_to_main.clone());

        // Create task manager for consistent async operations
        let task_manager =
            TaskManager::new(taskpool.clone(), tx_to_main.clone(), error_reporter.clone());

        // Create managers
        let state_manager = StateManager::new(tx_to_main.clone());
        let queue_manager = QueueManager::new(
            service_bus_manager.clone(),
            task_manager.clone(),
            tx_to_main.clone(),
        );

        (error_reporter, task_manager, state_manager, queue_manager)
    }

    /// Setup authentication service if Azure AD is configured
    fn setup_authentication(
        config: &crate::config::AppConfig,
        tx_to_main: mpsc::Sender<Msg>,
    ) -> AppResult<Option<Arc<crate::services::AuthService>>> {
        if config.auth().primary_method() == "azure_ad" {
            let auth_service = Arc::new(
                crate::services::AuthService::new(config.azure_ad(), tx_to_main.clone())
                    .map_err(|e| AppError::Component(e.to_string()))?,
            );

            // Set the global auth state for server components to use
            let auth_state = auth_service.auth_state_manager();
            server::auth::set_global_auth_state(auth_state);

            Ok(Some(auth_service))
        } else {
            Ok(None)
        }
    }

    /// Trigger initial authentication flow or load namespaces
    fn trigger_initial_flow(
        needs_auth: bool,
        app: &mut Model<CrosstermTerminalAdapter>,
    ) -> AppResult<()> {
        log::info!(
            "Authentication check: needs_auth = {}, has_auth_service = {}",
            needs_auth,
            app.auth_service.is_some()
        );

        if needs_auth {
            if let Some(ref auth_service) = app.auth_service {
                // Set authentication flag to prevent namespace loading
                app.state_manager.is_authenticating = true;

                // Clone auth_service to move into async task
                let auth_service = auth_service.clone();

                // Start authentication process immediately (not in background)
                // This will show the device code popup
                let tx = app.state_manager.tx_to_main.clone();
                tokio::spawn(async move {
                    // Small delay to ensure UI is ready
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                    // Initiate authentication - this will show device code popup
                    if let Err(e) = auth_service.initiate_authentication().await {
                        log::error!("Failed to initiate authentication: {e}");
                        let _ = tx.send(Msg::Error(e));
                    }
                });
            } else {
                // No auth service but needs auth - this shouldn't happen
                return Err(AppError::Config(
                    "Authentication required but auth service not initialized".to_string(),
                ));
            }
        } else {
            // No authentication needed, load namespaces directly
            app.queue_manager.load_namespaces();
        }

        Ok(())
    }
}
