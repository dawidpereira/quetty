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

use crate::app::views::{
    view_message_details, view_message_picker, view_namespace_picker, view_queue_picker,
};
use crate::components::common::{
    ComponentId, MessageActivityMsg, Msg, NamespaceActivityMsg, QueueActivityMsg,
};
use crate::components::global_key_watcher::GlobalKeyWatcher;
use crate::components::label::Label;
use crate::components::message_details::MessageDetails;
use crate::components::messages::Messages;
use crate::components::namespace_picker::NamespacePicker;
use crate::components::queue_picker::QueuePicker;
use crate::config;

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
    pub async fn new() -> Self {
        let service_bus_client_result = ServiceBusClient::new_from_connection_string(
            config::CONFIG.servicebus().connection_string(),
            ServiceBusClientOptions::default(),
        )
        .await;
        let (tx_to_main, rx_to_main) = mpsc::channel();
        let taskpool = TaskPool::new(10);

        let mut app = match service_bus_client_result {
            Ok(service_bus_client) => Self {
                app: Self::init_app(None),
                quit: false,
                redraw: true,
                terminal: TerminalBridge::init_crossterm().expect("Cannot initialize terminal"),
                app_state: AppState::NamespacePicker,
                tx_to_main,
                rx_to_main,
                taskpool,
                service_bus_client: Arc::new(Mutex::new(service_bus_client)),
                pending_queue: None,
                consumer: None,
                messages: None,
                selected_namespace: None,
            },
            Err(e) => {
                panic!("Error creating ServiceBusClient: {}", e);
            }
        };
        app.load_namespaces();
        app
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
                        AppState::NamespacePicker => {
                            view_namespace_picker(&mut self.app, f, &chunks);
                        }
                        AppState::QueuePicker => {
                            view_queue_picker(&mut self.app, f, &chunks);
                        }
                        AppState::MessagePicker => {
                            view_message_picker(&mut self.app, f, &chunks);
                        }
                        AppState::MessageDetails => {
                            view_message_details(&mut self.app, f, &chunks);
                        }
                    }
                })
                .is_ok()
        );
    }

    fn init_app(
        messages: Option<&Vec<MessageModel>>,
    ) -> Application<ComponentId, Msg, NoUserEvent> {
        let mut app: Application<ComponentId, Msg, NoUserEvent> = Application::init(
            EventListenerCfg::default()
                .crossterm_input_listener(
                    config::CONFIG.crossterm_input_listener_interval(),
                    config::CONFIG.crossterm_input_listener_retries(),
                )
                .poll_timeout(config::CONFIG.poll_timeout())
                .tick_interval(config::CONFIG.tick_interval()),
        );

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
                ComponentId::NamespacePicker,
                Box::new(NamespacePicker::new(None)),
                Vec::default(),
            )
            .is_ok()
        );
        assert!(
            app.mount(
                ComponentId::QueuePicker,
                Box::new(QueuePicker::new(None)),
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
                ComponentId::GlobalKeyWatcher,
                Box::new(GlobalKeyWatcher::default()),
                vec![Sub::new(SubEventClause::Any, SubClause::Always)]
            )
            .is_ok()
        );
        assert!(app.active(&ComponentId::Messages).is_ok());
        app
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
                Msg::MessageActivity(MessageActivityMsg::EditMessage(index)) => {
                    self.remount_message_details(index);
                    self.app_state = AppState::MessageDetails;
                    Some(Msg::ForceRedraw)
                }
                Msg::MessageActivity(MessageActivityMsg::PreviewMessageDetails(index)) => {
                    self.remount_message_details(index);
                    Some(Msg::ForceRedraw)
                }
                Msg::MessageActivity(MessageActivityMsg::CancelEditMessage) => {
                    self.app_state = AppState::MessagePicker;
                    None
                }
                Msg::MessageActivity(MessageActivityMsg::MessagesLoaded(messages)) => {
                    self.messages = Some(messages);
                    self.remount_messages();
                    self.remount_message_details(0);
                    self.app_state = AppState::MessagePicker;
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
                Msg::QueueActivity(QueueActivityMsg::QueuesLoaded(queues)) => {
                    self.remount_queue_picker(Some(queues));
                    self.app_state = AppState::QueuePicker;
                    None
                }
                Msg::QueueActivity(QueueActivityMsg::QueueUnselected) => {
                    self.app_state = AppState::QueuePicker;
                    None
                }
                Msg::NamespaceActivity(NamespaceActivityMsg::NamespacesLoaded(namespace)) => {
                    self.remount_namespace_picker(Some(namespace));
                    self.app_state = AppState::NamespacePicker;
                    None
                }
                Msg::NamespaceActivity(NamespaceActivityMsg::NamespaceSelected) => {
                    self.load_queues();
                    None
                }
                Msg::NamespaceActivity(NamespaceActivityMsg::NamespaceUnselected) => {
                    self.load_namespaces();
                    None
                }
                _ => None,
            }
        } else {
            None
        }
    }
}
