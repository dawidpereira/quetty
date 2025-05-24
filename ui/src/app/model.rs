use crate::app::view::{
    view_message_details, view_message_picker, view_namespace_picker, view_queue_picker,
    with_error_popup,
};
use crate::components::common::{ComponentId, Msg};
use crate::components::error_popup::ErrorPopup;
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

        // Load namespaces and handle any errors through the message system
        if let Err(e) = app.load_namespaces() {
            // Send the error through the channel to be handled in the main event loop
            if app.tx_to_main.send(Msg::Error(e.clone())).is_err() {
                // If the channel send fails, handle the error directly
                log::error!("Failed to send error through channel: {}", e);
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

            // Apply the view based on the app state, with error popup handling
            view_result = match self.app_state {
                AppState::NamespacePicker => {
                    with_error_popup(&mut self.app, f, &chunks, view_namespace_picker)
                }
                AppState::QueuePicker => {
                    with_error_popup(&mut self.app, f, &chunks, view_queue_picker)
                }
                AppState::MessagePicker => {
                    with_error_popup(&mut self.app, f, &chunks, view_message_picker)
                }
                AppState::MessageDetails => {
                    with_error_popup(&mut self.app, f, &chunks, view_message_details)
                }
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

    /// Mount error popup and give focus to it
    pub fn mount_error_popup(&mut self, error: &AppError) -> AppResult<()> {
        log::error!("Displaying error popup: {}", error);

        self.app
            .remount(
                ComponentId::ErrorPopup,
                Box::new(ErrorPopup::new(error)),
                Vec::default(),
            )
            .map_err(|e| AppError::Component(e.to_string()))?;

        self.app
            .active(&ComponentId::ErrorPopup)
            .map_err(|e| AppError::Component(e.to_string()))?;

        self.redraw = true;

        Ok(())
    }

    /// Unmount error popup and return focus to previous component
    pub fn unmount_error_popup(&mut self) -> AppResult<()> {
        self.app
            .umount(&ComponentId::ErrorPopup)
            .map_err(|e| AppError::Component(e.to_string()))?;

        // Return to appropriate state
        match self.app_state {
            AppState::NamespacePicker => {
                self.app
                    .active(&ComponentId::NamespacePicker)
                    .map_err(|e| AppError::Component(e.to_string()))?;
            }
            AppState::QueuePicker => {
                self.app
                    .active(&ComponentId::QueuePicker)
                    .map_err(|e| AppError::Component(e.to_string()))?;
            }
            AppState::MessagePicker => {
                self.app
                    .active(&ComponentId::Messages)
                    .map_err(|e| AppError::Component(e.to_string()))?;
            }
            AppState::MessageDetails => {
                self.app
                    .active(&ComponentId::MessageDetails)
                    .map_err(|e| AppError::Component(e.to_string()))?;
            }
        }

        self.redraw = true;

        Ok(())
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

            // Process the message and handle any resulting errors
            let result = match msg {
                Msg::AppClose => {
                    self.quit = true; // Terminate
                    None
                }
                Msg::Submit(lines) => {
                    match ClipboardContext::new() {
                        Ok(mut ctx) => {
                            if let Err(e) = ctx.set_contents(lines.join("\n")) {
                                if let Err(err) = self.mount_error_popup(&AppError::Component(
                                    format!("Error copying to clipboard: {}", e),
                                )) {
                                    log::error!("Failed to mount error popup: {}", err);
                                }
                            }
                        }
                        Err(e) => {
                            if let Err(err) = self.mount_error_popup(&AppError::Component(format!(
                                "Failed to initialize clipboard: {}",
                                e
                            ))) {
                                log::error!("Failed to mount error popup: {}", err);
                            }
                        }
                    }
                    None
                }
                Msg::MessageActivity(msg) => self.update_messages(msg),
                Msg::QueueActivity(msg) => self.update_queue(msg),
                Msg::NamespaceActivity(msg) => self.update_namespace(msg),
                Msg::Error(e) => {
                    log::error!("Error received: {}", e);
                    if let Err(err) = self.mount_error_popup(&e) {
                        log::error!("Failed to mount error popup: {}", err);
                        // Fallback to terminal error handling
                        crate::error::handle_error(e);
                    }
                    None
                }
                Msg::CloseErrorPopup => {
                    if let Err(e) = self.unmount_error_popup() {
                        log::error!("Failed to unmount error popup: {}", e);
                    }
                    None
                }
                _ => None,
            };

            if let Some(Msg::Error(e)) = result {
                log::error!("Error from message processing: {}", e);
                if let Err(err) = self.mount_error_popup(&e) {
                    log::error!("Failed to mount error popup: {}", err);
                    crate::error::handle_error(e);
                }
                None
            } else {
                result
            }
        } else {
            None
        }
    }
}
