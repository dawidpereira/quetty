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
use tuirealm::props::TextModifiers;
use tuirealm::props::{Alignment, Color};
use tuirealm::ratatui::layout::{Constraint, Direction, Layout};
use tuirealm::terminal::{CrosstermTerminalAdapter, TerminalAdapter, TerminalBridge};
use tuirealm::{Application, EventListenerCfg, Sub, SubClause, SubEventClause, Update};

use crate::components::common::{ComponentId, MessageActivityMsg, Msg, QueueActivityMsg};
use crate::components::global_key_watcher::GlobalKeyWatcher;
use crate::components::label::Label;
use crate::components::message_details::MessageDetails;
use crate::components::messages::Messages;
use crate::components::queue_picker::QueuePicker;
use crate::config;

pub enum AppState {
    QueuePicker,
    Main,
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

    pub taskpool: TaskPool,
    pub tx_to_main: Sender<Msg>,
    pub rx_to_main: Receiver<Msg>,

    pub service_bus_client: Arc<Mutex<ServiceBusClient<BasicRetryPolicy>>>,
    pub pending_queue: Option<String>,
    pub consumer: Option<Arc<Mutex<Consumer>>>,
    pub messages: Option<Vec<MessageModel>>,
}

impl Model<CrosstermTerminalAdapter> {
    pub async fn new() -> Self {
        let service_bus_client_result = ServiceBusClient::new_from_connection_string(
            config::CONFIG.servicebus().connection_string(),
            ServiceBusClientOptions::default(),
        )
        .await;
        let (tx_to_main, rx_to_main) = mpsc::channel();
        let taskpool = TaskPool::new(10);

        match service_bus_client_result {
            Ok(service_bus_client) => Self {
                app: Self::init_app(None),
                quit: false,
                redraw: true,
                terminal: TerminalBridge::init_crossterm().expect("Cannot initialize terminal"),
                app_state: AppState::QueuePicker,
                tx_to_main,
                rx_to_main,
                taskpool,
                service_bus_client: Arc::new(Mutex::new(service_bus_client)),
                pending_queue: None,
                consumer: None,
                messages: None,
            },
            Err(e) => {
                panic!("Error creating ServiceBusClient: {}", e);
            }
        }
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

    pub fn view(&mut self) {
        assert!(
            self.terminal
                .draw(|f| {
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

                    match self.app_state {
                        AppState::QueuePicker => {
                            self.app.view(&ComponentId::QueuePicker, f, chunks[3]);

                            if self.app.active(&ComponentId::QueuePicker).is_err() {
                                println!("Error: Messages component not active");
                            }
                        }
                        AppState::Main => {
                            let main_chunks = Layout::default()
                                .direction(Direction::Horizontal)
                                .margin(1)
                                .constraints(
                                    [
                                        Constraint::Min(49), // Messages
                                        Constraint::Min(49),
                                    ]
                                    .as_ref(),
                                )
                                .split(chunks[3]);

                            self.app.view(&ComponentId::Messages, f, main_chunks[0]);
                            self.app
                                .view(&ComponentId::MessageDetails, f, main_chunks[1]);

                            if self.app.active(&ComponentId::Messages).is_err() {
                                println!("Error: Messages component not active");
                            }
                        }
                    }
                })
                .is_ok()
        );
    }

    fn init_app(
        messages: Option<&Vec<MessageModel>>, //NOTE: Not needed in final scope. Will be removed after early development phase.
    ) -> Application<ComponentId, Msg, NoUserEvent> {
        // Setup application
        let mut app: Application<ComponentId, Msg, NoUserEvent> = Application::init(
            EventListenerCfg::default()
                .crossterm_input_listener(
                    config::CONFIG.crossterm_input_listener_interval(),
                    config::CONFIG.crossterm_input_listener_retries(),
                )
                .poll_timeout(config::CONFIG.poll_timeout())
                .tick_interval(config::CONFIG.tick_interval()),
        );

        // Mount components
        assert!(
            app.mount(
                ComponentId::Label,
                Box::new(
                    Label::default()
                        .text("Quetty, the cutest queue manager <3")
                        .alignment(Alignment::Center)
                        .background(Color::Reset)
                        .foreground(Color::Green)
                        .modifiers(TextModifiers::BOLD),
                ),
                Vec::default(),
            )
            .is_ok()
        );

        assert!(
            app.mount(
                ComponentId::Messages,
                Box::new(Messages::new(messages)),
                Vec::default(),
            )
            .is_ok()
        );
        assert!(
            app.mount(
                ComponentId::MessageDetails,
                Box::new(MessageDetails::new(None)),
                Vec::default(),
            )
            .is_ok()
        );
        assert!(
            app.mount(
                ComponentId::QueuePicker,
                Box::new(QueuePicker::new(None)), // Will be remounted with real queues later
                Vec::default(),
            )
            .is_ok()
        );
        assert!(
            app.mount(
                ComponentId::GlobalKeyWatcher,
                Box::new(GlobalKeyWatcher::default()),
                vec![Sub::new(SubEventClause::Any, SubClause::Always)]
            )
            .is_ok()
        );
        assert!(app.active(&ComponentId::Messages).is_ok());
        app
    }

    pub fn remount_queue_picker(&mut self, queues: Vec<String>) {
        assert!(
            self.app
                .remount(
                    ComponentId::QueuePicker,
                    Box::new(QueuePicker::new(Some(queues))),
                    Vec::default(),
                )
                .is_ok()
        );
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
                Msg::MessageActivity(MessageActivityMsg::RefreshMessageDetails(index)) => {
                    let mut message: Option<MessageModel> = None;
                    if self.messages.is_some() {
                        message = self.messages.as_ref().unwrap().get(index).cloned();
                    }
                    assert!(
                        self.app
                            .remount(
                                ComponentId::MessageDetails,
                                Box::new(MessageDetails::new(message)),
                                Vec::default(),
                            )
                            .is_ok()
                    );
                    None
                }
                Msg::MessageActivity(MessageActivityMsg::EditMessage(_)) => {
                    if self.app.active(&ComponentId::MessageDetails).is_err() {
                        println!("Error: MessageDetails component not active");
                        return None;
                    }
                    None
                }
                Msg::MessageActivity(MessageActivityMsg::CancelEditMessage) => {
                    if self.app.active(&ComponentId::Messages).is_err() {
                        println!("Error: Messages component not active");
                        return None;
                    }
                    None
                }
                Msg::MessageActivity(MessageActivityMsg::MessagesLoaded(messages)) => {
                    self.messages = Some(messages);
                    assert!(
                        self.app
                            .remount(
                                ComponentId::Messages,
                                Box::new(Messages::new(self.messages.as_ref())),
                                Vec::default(),
                            )
                            .is_ok()
                    );
                    self.app_state = AppState::Main;
                    None
                }
                Msg::MessageActivity(MessageActivityMsg::ConsumerCreated(consumer)) => {
                    self.consumer = Some(Arc::new(Mutex::new(consumer)));
                    self.load_messages();
                    None
                }
                Msg::QueueActivity(QueueActivityMsg::QueueSelected(queue)) => {
                    self.pending_queue = Some(queue);
                    self.new_consumer_for_queue();
                    None
                }
                Msg::QueueActivity(QueueActivityMsg::QueueUnfocused) => {
                    self.app_state = AppState::QueuePicker;
                    None
                }
                _ => None,
            }
        } else {
            None
        }
    }
}
