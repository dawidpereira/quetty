use crate::app::view::{
    view_message_details, view_message_picker, view_namespace_picker, view_queue_picker,
};
use crate::components::common::{ComponentId, Msg};
use crate::components::global_key_watcher::GlobalKeyWatcher;
use crate::components::message_details::MessageDetails;
use crate::components::messages::Messages;
use crate::components::namespace_picker::NamespacePicker;
use crate::components::queue_picker::QueuePicker;
use crate::components::text_label::TextLabel;
use crate::config;
use crate::error::{AppError, AppResult};
use azservicebus::core::BasicRetryPolicy;
use azservicebus::{ServiceBusClient, ServiceBusClientOptions};
use copypasta::{ClipboardContext, ClipboardProvider};
use server::consumer::Consumer;
use server::model::MessageModel;
use server::taskpool::TaskPool;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;
use tuirealm::event::NoUserEvent;
use tuirealm::ratatui::layout::{Constraint, Direction, Layout};
use tuirealm::terminal::{CrosstermTerminalAdapter, TerminalAdapter, TerminalBridge};
use tuirealm::{Application, EventListenerCfg, Sub, SubClause, SubEventClause, Update};

pub enum AppState {
    NamespacePicker,
    QueuePicker,
    MessagePicker,
    MessageDetails,
}

pub struct Model<T>
where
    T: TerminalAdapter,
{
    /// Application
    pub app: Application<ComponentId, Msg, NoUserEvent>,
    pub app_state: AppState,
    /// Indicates that the application must quit
    pub quit: bool,
    /// Tells whether to redraw interface
    pub redraw: bool,
    /// Used to draw to terminal
    pub terminal: TerminalBridge<T>,

    pub pending_queue: Option<String>,
    pub selected_namespace: Option<String>,

    pub taskpool: TaskPool,
    pub tx_to_main: Sender<Msg>,
    pub rx_to_main: Receiver<Msg>,

    pub service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
    pub consumer: Option<Arc<Mutex<Consumer>>>,
    pub messages: Option<Vec<MessageModel>>,
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

        let mut app = Self {
            app: Self::init_app(None)?,
            quit: false,
            redraw: true,
            terminal: TerminalBridge::init_crossterm()
                .map_err(|e| AppError::Component(e.to_string()))?,
            app_state: AppState::NamespacePicker,
            tx_to_main,
            rx_to_main,
            taskpool,
            service_bus_client: Arc::new(Mutex::new(service_bus_client)),
            pending_queue: None,
            consumer: None,
            messages: None,
            selected_namespace: None,
        };
        if let Err(e) = app.load_namespaces() {
            // Send the error through the channel to be handled in the main event loop
            if app.tx_to_main.send(Msg::Error(e.clone())).is_err() {
                // If the channel send fails, handle the error directly
                crate::error::handle_error(e);
            }
        }
        Ok(app)
    }
}

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn update_outside_msg(&mut self) {
        if let Ok(msg) = self.rx_to_main.try_recv() {
            self.update(Some(msg));
        }
    }

    pub fn view(&mut self) -> AppResult<()> {
        let mut view_result: AppResult<()> = Ok(());
        let _ = self.terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(1),
                        Constraint::Length(1), // Label
                        Constraint::Length(2),
                        Constraint::Min(16), // Main area
                    ]
                    .as_ref(),
                )
                .split(f.area());

            self.app.view(&ComponentId::Label, f, chunks[1]);

            view_result = match self.app_state {
                AppState::NamespacePicker => view_namespace_picker(&mut self.app, f, &chunks),
                AppState::QueuePicker => view_queue_picker(&mut self.app, f, &chunks),
                AppState::MessagePicker => view_message_picker(&mut self.app, f, &chunks),
                AppState::MessageDetails => view_message_details(&mut self.app, f, &chunks),
            };
        });

        view_result
    }

    fn init_app(
        messages: Option<&Vec<MessageModel>>,
    ) -> AppResult<Application<ComponentId, Msg, NoUserEvent>> {
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
            ComponentId::Label,
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
            Box::new(Messages::new(messages)),
            Vec::default(),
        )
        .map_err(|e| AppError::Component(e.to_string()))?;

        app.mount(
            ComponentId::MessageDetails,
            Box::new(MessageDetails::new(None)),
            Vec::default(),
        )
        .map_err(|e| AppError::Component(e.to_string()))?;

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

impl<T> Update<Msg> for Model<T>
where
    T: TerminalAdapter,
{
    fn update(&mut self, msg: Option<Msg>) -> Option<Msg> {
        if let Some(msg) = msg {
            // Set redraw
            self.redraw = true;
            // Match message
            match msg {
                Msg::AppClose => {
                    self.quit = true; // Terminate
                    None
                }
                Msg::Submit(lines) => {
                    match ClipboardContext::new() {
                        Ok(mut ctx) => {
                            if let Err(e) = ctx.set_contents(lines.join("\n")) {
                                //TODO: Move to global error handler
                                println!("Error during copying data to clipboard: {}", e);
                            }
                        }
                        Err(e) => {
                            //TODO: Move to global error handler
                            println!("Failed to initialize clipboard context: {}", e);
                        }
                    }
                    None
                }
                Msg::MessageActivity(msg) => self.update_messages(msg),
                Msg::QueueActivity(msg) => self.update_queue(msg),
                Msg::NamespaceActivity(msg) => self.update_namespace(msg),
                Msg::Error(e) => {
                    crate::error::handle_error(e);
                    None
                }
                _ => None,
            }
        } else {
            None
        }
    }
}
