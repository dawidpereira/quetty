use super::{AppState, Model};
use crate::app::queue_state::QueueState;
use crate::components::common::{ComponentId, LoadingActivityMsg, Msg};
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
use azservicebus::{ServiceBusClient, ServiceBusClientOptions};
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
        let mut app: Application<ComponentId, Msg, NoUserEvent> = Application::init(
            EventListenerCfg::default()
                .crossterm_input_listener(
                    config::CONFIG.crossterm_input_listener_interval(),
                    config::CONFIG.crossterm_input_listener_retries(),
                )
                .poll_timeout(config::CONFIG.poll_timeout())
                .tick_interval(config::CONFIG.tick_interval()),
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
        let service_bus_client = ServiceBusClient::new_from_connection_string(
            config::CONFIG.servicebus().connection_string(),
            ServiceBusClientOptions::default(),
        )
        .await
        .map_err(|e| AppError::ServiceBus(e.to_string()))?;

        let (tx_to_main, rx_to_main) = mpsc::channel();
        let taskpool = TaskPool::new(10);

        // Create error reporter for enhanced error handling
        let error_reporter = ErrorReporter::new(tx_to_main.clone());

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
            service_bus_client: Arc::new(Mutex::new(service_bus_client)),
            selected_namespace: None,
            loading_message: None,
            previous_state: None,
            active_component: ComponentId::NamespacePicker,
            queue_state,
            pending_confirmation_action: None,
            is_editing_message: false,
            error_reporter,
        };

        // Initialize loading indicator with ComponentState pattern using extension trait
        app.app.mount_with_state(
            ComponentId::LoadingIndicator,
            LoadingIndicator::new("Loading...", true),
            Vec::default(),
        )?;

        // Load namespaces and handle any errors through the message system
        if app
            .tx_to_main
            .send(Msg::LoadingActivity(LoadingActivityMsg::Start(
                "Loading namespaces...".to_string(),
            )))
            .is_err()
        {
            log::error!("Failed to send loading start message");
        }

        if let Err(e) = app.load_namespaces() {
            app.error_reporter
                .report_simple(e, "Initialization", "load_namespaces");
        }

        Ok(app)
    }
}
