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
use crate::utils::auth::AuthUtils;
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
    fn init_app(
        queue_state: &QueueState,
        needs_auth: bool,
    ) -> AppResult<Application<ComponentId, Msg, NoUserEvent>> {
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

        // Only mount NamespacePicker if authentication is not needed
        // This prevents it from briefly appearing before the auth popup
        if !needs_auth {
            app.mount(
                ComponentId::NamespacePicker,
                Box::new(NamespacePicker::new(None)),
                Vec::default(),
            )
            .map_err(|e| AppError::Component(e.to_string()))?;
        }

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
        let needs_auth = config.azure_ad().auth_method != "connection_string";

        // Create shared HTTP client
        let http_client = Self::create_http_client();

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
        let auth_service =
            Self::setup_authentication(config, tx_to_main.clone(), http_client.clone())?;

        let queue_state = QueueState::new();
        let mut app = Self {
            app: Self::init_app(&queue_state, needs_auth)?,
            terminal: TerminalBridge::init_crossterm()
                .map_err(|e| AppError::Component(e.to_string()))?,
            rx_to_main,
            taskpool,
            service_bus_manager,
            http_client,
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

    /// Create optimized HTTP client with connection pooling
    fn create_http_client() -> reqwest::Client {
        use std::time::Duration;

        reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(8)
            .build()
            .unwrap_or_else(|e| {
                log::warn!(
                    "Failed to create optimized HTTP client: {e}, falling back to default client"
                );
                reqwest::Client::new()
            })
    }

    /// Create Service Bus manager based on configuration
    async fn create_service_bus_manager(
        config: &crate::config::AppConfig,
    ) -> AppResult<Option<Arc<Mutex<ServiceBusManager>>>> {
        let auth_method = &config.azure_ad().auth_method;
        let needs_auth = auth_method != "connection_string";

        if needs_auth {
            // Azure AD auth is configured - we'll create the manager later after authentication/discovery
            log::info!(
                "Azure AD authentication configured, will create Service Bus manager after auth"
            );
            Ok(None)
        } else {
            // Connection string auth - check if we have encrypted connection string but no password yet
            if config.servicebus().has_connection_string() {
                // We have an encrypted connection string but may not have password yet
                // Don't try to decrypt during startup - defer until user provides password
                log::info!(
                    "Encrypted connection string available, will create Service Bus manager after password input"
                );
                Ok(None)
            } else {
                // No encrypted connection string configured
                log::warn!(
                    "Connection string authentication configured but no encrypted connection string available"
                );
                Ok(None)
            }
        }
    }

    /// Log authentication configuration information
    fn log_authentication_info(config: &crate::config::AppConfig) {
        if config.azure_ad().auth_method != "connection_string" {
            log::info!("Azure AD authentication configured for management operations");
            log::info!("Flow: {}", config.azure_ad().auth_method);
            if config.azure_ad().auth_method == "device_code" {
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
        http_client: reqwest::Client,
    ) -> AppResult<Option<Arc<crate::services::AuthService>>> {
        // Only create auth service if we have required auth fields and not using connection_string
        if !AuthUtils::is_connection_string_auth(config) && config.has_required_auth_fields() {
            let auth_service = Arc::new(
                crate::services::AuthService::new(
                    config.azure_ad(),
                    tx_to_main.clone(),
                    http_client,
                )
                .map_err(|e| AppError::Component(e.to_string()))?,
            );

            // Set the global auth state for server components to use
            let auth_state = auth_service.auth_state_manager();
            server::auth::set_global_auth_state(auth_state.clone());

            // Start the token refresh service with failure callback
            let tx_clone = tx_to_main.clone();
            tokio::spawn(async move {
                let failure_callback = Arc::new(move |error: server::auth::TokenRefreshError| {
                    log::error!("Token refresh failed: {error}");

                    // Send notification to UI
                    let _ = tx_clone.send(crate::components::common::Msg::AuthActivity(
                        crate::components::common::AuthActivityMsg::TokenRefreshFailed(
                            error.to_string(),
                        ),
                    ));
                });

                auth_state
                    .start_refresh_service_with_callback(Some(failure_callback))
                    .await;
            });

            Ok(Some(auth_service))
        } else {
            if !AuthUtils::is_connection_string_auth(config) {
                log::info!("Skipping auth service creation - missing required auth fields");
            }
            Ok(None)
        }
    }

    /// Trigger initial authentication flow or load namespaces
    fn trigger_initial_flow(
        needs_auth: bool,
        app: &mut Model<CrosstermTerminalAdapter>,
    ) -> AppResult<()> {
        let config = config::get_config_or_panic();

        log::info!(
            "Authentication check: needs_auth = {}, has_auth_service = {}",
            needs_auth,
            app.auth_service.is_some()
        );

        // First check if we have the required configuration for the selected auth method
        if !config.has_required_auth_fields() {
            log::info!("Required configuration fields are missing, opening config screen directly");

            // Set authentication flag to prevent namespace loading
            app.state_manager.is_authenticating = true;

            // Auto-open config screen for missing authentication fields
            let tx = app.state_manager.tx_to_main.clone();
            tokio::spawn(async move {
                // Small delay to ensure UI is ready
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                let _ = tx.send(Msg::ToggleConfigScreen);
            });

            return Ok(());
        }

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

                        // Check if error is due to missing fields, redirect to config
                        let error_str = e.to_string();
                        if error_str.contains("client ID")
                            || error_str.contains("tenant ID")
                            || error_str.contains("Invalid authentication request")
                        {
                            log::info!(
                                "Authentication failed due to invalid credentials, opening config screen"
                            );
                            let _ = tx.send(Msg::ToggleConfigScreen);
                        } else {
                            let _ = tx.send(Msg::Error(e));
                        }
                    }
                });

                // IMPORTANT: Do not load namespaces when authentication is needed
                // The namespace loading will happen after successful authentication
                log::info!("Skipping namespace loading - authentication required first");
            } else {
                // Auth service should have been created but wasn't - config issue
                log::error!("Authentication required but auth service not initialized");
                return Err(AppError::Config(
                    "Authentication required but auth service not initialized".to_string(),
                ));
            }
        } else {
            // Using connection string authentication - check if we can decrypt connection string
            log::info!("Using connection string authentication");

            let config = config::get_config_or_panic();

            // Check if we have encrypted connection string data first
            if !config.servicebus().has_connection_string() {
                // No encrypted connection string configured at all
                log::info!("No connection string configured - opening config screen");
                app.state_manager.is_authenticating = true;

                let tx = app.state_manager.tx_to_main.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    let _ = tx.send(Msg::ToggleConfigScreen);
                });
            } else {
                // We have encrypted connection string, try to decrypt it
                match config.servicebus().connection_string() {
                    Ok(Some(_)) => {
                        // Successfully decrypted connection string - load namespaces directly
                        log::info!("Connection string decrypted successfully - loading namespaces");
                        app.queue_manager.load_namespaces();
                    }
                    Ok(None) => {
                        // This shouldn't happen if has_connection_string() returned true
                        log::error!(
                            "has_connection_string() returned true but connection_string() returned None"
                        );
                        app.state_manager.is_authenticating = true;

                        let tx = app.state_manager.tx_to_main.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            let _ = tx.send(Msg::ToggleConfigScreen);
                        });
                    }
                    Err(e) => {
                        // Failed to decrypt - likely missing master password
                        log::info!(
                            "Failed to decrypt connection string (master password needed): {e}"
                        );
                        log::info!("Opening password popup for master password input");
                        app.state_manager.is_authenticating = true;

                        let tx = app.state_manager.tx_to_main.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            let _ = tx.send(Msg::TogglePasswordPopup);
                        });
                    }
                }
            }
        }

        Ok(())
    }
}
