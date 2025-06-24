use super::{AppState, Model};
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
        // Create the underlying Azure Service Bus client
        let config = config::get_config_or_panic();
        let connection_string = config.servicebus().connection_string()
            .map_err(|e| AppError::Config(format!("Failed to get connection string: {}", e)))?;
        
        let azure_service_bus_client = AzureServiceBusClient::new_from_connection_string(
            connection_string,
            ServiceBusClientOptions::default(),
        )
        .await
        .map_err(|e| AppError::ServiceBus(e.to_string()))?;

        // Create the service bus manager directly
        let service_bus_manager = Arc::new(Mutex::new(ServiceBusManager::new(Arc::new(Mutex::new(azure_service_bus_client)))));

        let (tx_to_main, rx_to_main) = mpsc::channel();
        let taskpool = TaskPool::new(10);

        // Create error reporter for enhanced error handling
        let error_reporter = ErrorReporter::new(tx_to_main.clone());

        // Create task manager for consistent async operations
        let task_manager =
            TaskManager::new(taskpool.clone(), tx_to_main.clone(), error_reporter.clone());

        let queue_state = QueueState::new();
        let mut app = Self {
            app: Self::init_app(&queue_state)?,
            quit: false,
            redraw: true,
            terminal: TerminalBridge::init_crossterm()
                .map_err(|e| AppError::Component(e.to_string()))?,
            app_state: AppState::NamespacePicker,
            tx_to_main,
            rx_to_main,
            taskpool,
            service_bus_manager, // Use ServiceBusManager directly
            selected_namespace: None,
            loading_message: None,
            previous_state: None,
            active_component: ComponentId::NamespacePicker,
            queue_state,
            pending_confirmation_action: None,
            is_editing_message: false,
            error_reporter,
            task_manager,
        };

        // Initialize loading indicator with ComponentState pattern using extension trait
        app.app.mount_with_state(
            ComponentId::LoadingIndicator,
            LoadingIndicator::new("Loading...", true),
            Vec::default(),
        )?;

        // Use task manager for loading namespaces instead of direct error handling
        app.load_namespaces();

        Ok(app)
    }
}
