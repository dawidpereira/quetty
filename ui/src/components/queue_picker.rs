use azservicebus::ServiceBusReceiverOptions;
use server::consumer::ServiceBusClientExt;
use server::service_bus_manager::ServiceBusManager;
use tuirealm::command::CmdResult;
use tuirealm::event::{Key, KeyEvent};
use tuirealm::props::{Alignment, Color, Style, TextModifiers};
use tuirealm::ratatui::layout::Rect;
use tuirealm::ratatui::widgets::{List, ListItem};
use tuirealm::terminal::TerminalAdapter;
use tuirealm::{Component, Event, Frame, MockComponent, NoUserEvent};

use crate::app::model::Model;
use crate::config::CONFIG;
use crate::error::{AppError, AppResult};

use crate::components::common::{
    LoadingActivityMsg, MessageActivityMsg, Msg, NamespaceActivityMsg, QueueActivityMsg,
};

const CMD_RESULT_QUEUE_SELECTED: &str = "QueueSelected";
const CMD_RESULT_NAMESPACE_UNSELECTED: &str = "NamespaceUnselected";

pub struct QueuePicker {
    queues: Vec<String>,
    selected: usize,
}

impl QueuePicker {
    pub fn new(queues: Option<Vec<String>>) -> Self {
        Self {
            queues: queues.unwrap_or_default(),
            selected: 0,
        }
    }
}

impl MockComponent for QueuePicker {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .queues
            .iter()
            .enumerate()
            .map(|(i, q)| {
                let mut item = ListItem::new(q.clone());
                if i == self.selected {
                    item = item.style(Style::default().add_modifier(TextModifiers::REVERSED));
                }
                item
            })
            .collect();
        let list = List::new(items)
            .block(
                tuirealm::ratatui::widgets::Block::default()
                    .borders(tuirealm::ratatui::widgets::Borders::ALL)
                    .border_style(Style::default().fg(Color::Green))
                    .title(" Select a queue ")
                    .title_alignment(Alignment::Center),
            )
            .highlight_style(Style::default().fg(Color::Yellow))
            .highlight_symbol("> ");
        frame.render_widget(list, area);
    }
    fn query(&self, _attr: tuirealm::Attribute) -> Option<tuirealm::AttrValue> {
        None
    }
    fn attr(&mut self, _attr: tuirealm::Attribute, _value: tuirealm::AttrValue) {}
    fn state(&self) -> tuirealm::State {
        if let Some(queue) = self.queues.get(self.selected) {
            tuirealm::State::One(tuirealm::StateValue::String(queue.clone()))
        } else {
            tuirealm::State::None
        }
    }
    fn perform(&mut self, _cmd: tuirealm::command::Cmd) -> tuirealm::command::CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for QueuePicker {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let cmd_result = match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Down | Key::Char('j'),
                ..
            }) => {
                if self.selected + 1 < self.queues.len() {
                    self.selected += 1;
                }
                CmdResult::Changed(tuirealm::State::One(tuirealm::StateValue::Usize(
                    self.selected,
                )))
            }
            Event::Keyboard(KeyEvent {
                code: Key::Up | Key::Char('k'),
                ..
            }) => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                CmdResult::Changed(tuirealm::State::One(tuirealm::StateValue::Usize(
                    self.selected,
                )))
            }
            Event::Keyboard(KeyEvent {
                code: Key::Enter | Key::Char('o'),
                ..
            }) => {
                if let Some(queue) = self.queues.get(self.selected).cloned() {
                    CmdResult::Custom(
                        CMD_RESULT_QUEUE_SELECTED,
                        tuirealm::State::One(tuirealm::StateValue::String(queue)),
                    )
                } else {
                    CmdResult::None
                }
            }
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => CmdResult::Custom(
                CMD_RESULT_NAMESPACE_UNSELECTED,
                tuirealm::State::One(tuirealm::StateValue::String("".to_string())),
            ),
            _ => CmdResult::None,
        };

        match cmd_result {
            CmdResult::Custom(CMD_RESULT_QUEUE_SELECTED, state) => {
                if let tuirealm::State::One(tuirealm::StateValue::String(queue)) = state {
                    Some(Msg::QueueActivity(QueueActivityMsg::QueueSelected(queue)))
                } else {
                    None
                }
            }
            CmdResult::Custom(CMD_RESULT_NAMESPACE_UNSELECTED, _) => Some(Msg::NamespaceActivity(
                NamespaceActivityMsg::NamespaceUnselected,
            )),
            CmdResult::Changed(_) => Some(Msg::ForceRedraw),
            _ => Some(Msg::ForceRedraw),
        }
    }
}

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn new_consumer_for_queue(&mut self) -> AppResult<()> {
        log::debug!("Creating new consumer for queue");
        let queue = self
            .queue_state
            .pending_queue
            .take()
            .expect("No queue selected");
        log::info!("Creating consumer for queue: {}", queue);

        // Store the queue name to update current_queue_name when consumer is created
        let queue_name_for_update = queue.clone();

        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        // Clone the Arc to pass into async block
        let service_bus_client = self.service_bus_client.clone();
        let consumer = self.queue_state.consumer.clone();

        let tx_to_main_err = tx_to_main.clone();
        taskpool.execute(async move {
            let result = async {
                if let Some(consumer) = consumer {
                    log::debug!("Disposing existing consumer");
                    if let Err(e) = consumer.lock().await.dispose().await {
                        log::error!("Failed to dispose consumer: {}", e);
                        return Err(AppError::ServiceBus(e.to_string()));
                    }
                }

                log::debug!("Acquiring service bus client lock");
                let mut client = service_bus_client.lock().await;
                log::debug!("Creating receiver for queue: {}", queue);
                let consumer = client
                    .create_consumer_for_queue(queue.clone(), ServiceBusReceiverOptions::default())
                    .await
                    .map_err(|e| {
                        log::error!("Failed to create consumer for queue {}: {}", queue, e);
                        AppError::ServiceBus(e.to_string())
                    })?;

                log::info!("Successfully created consumer for queue: {}", queue);
                tx_to_main
                    .send(Msg::MessageActivity(MessageActivityMsg::ConsumerCreated(
                        consumer,
                    )))
                    .map_err(|e| {
                        log::error!("Failed to send consumer created message: {}", e);
                        AppError::Component(e.to_string())
                    })?;

                // Send a separate message to update the current queue name
                tx_to_main
                    .send(Msg::MessageActivity(MessageActivityMsg::QueueNameUpdated(
                        queue_name_for_update,
                    )))
                    .map_err(|e| {
                        log::error!("Failed to send queue name updated message: {}", e);
                        AppError::Component(e.to_string())
                    })?;

                Ok::<(), AppError>(())
            }
            .await;
            if let Err(e) = result {
                log::error!("Error in consumer creation task: {}", e);
                let _ = tx_to_main_err.send(Msg::Error(e));
            }
        });

        Ok(())
    }

    pub fn load_queues(&mut self) -> AppResult<()> {
        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();
        let selected_namespace = self.selected_namespace.clone();

        // Show loading indicator
        if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Start(format!(
            "Loading queues from {}...",
            selected_namespace
                .clone()
                .unwrap_or_else(|| "default".to_string())
        )))) {
            log::error!("Failed to send loading start message: {}", e);
        }

        let tx_to_main_err = tx_to_main.clone();
        taskpool.execute(async move {
            let result = async {
                let mut config = CONFIG.azure_ad().clone();
                if let Some(ns) = selected_namespace.clone() {
                    log::debug!("Using namespace: {}", ns);
                    config.namespace = ns;
                } else {
                    log::warn!("No namespace selected, using default namespace");
                }

                // Send an update that we're connecting
                if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Update(
                    format!("Connecting to namespace {}...", config.namespace()),
                ))) {
                    log::error!("Failed to send loading update message: {}", e);
                }

                log::debug!("Requesting queues from Azure AD");

                let queues = ServiceBusManager::list_queues_azure_ad(&config)
                    .await
                    .map_err(|e| {
                        log::error!("Failed to list queues: {}", e);
                        AppError::ServiceBus(e.to_string())
                    })?;

                // Send an update that we've received queues
                if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Update(
                    format!("Processing {} queues...", queues.len()),
                ))) {
                    log::error!("Failed to send loading update message: {}", e);
                }

                log::info!(
                    "Loaded {} queues from namespace {}",
                    queues.len(),
                    selected_namespace.unwrap_or_else(|| "default".to_string())
                );

                // Stop loading indicator
                if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Stop)) {
                    log::error!("Failed to send loading stop message: {}", e);
                }

                // Send loaded queues
                tx_to_main
                    .send(Msg::QueueActivity(QueueActivityMsg::QueuesLoaded(queues)))
                    .map_err(|e| {
                        log::error!("Failed to send queues loaded message: {}", e);
                        AppError::Component(e.to_string())
                    })?;

                Ok::<(), AppError>(())
            }
            .await;
            if let Err(e) = result {
                log::error!("Error in queue loading task: {}", e);

                // Stop loading indicator even if there was an error
                if let Err(err) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Stop)) {
                    log::error!("Failed to send loading stop message: {}", err);
                }

                // Send error message
                let _ = tx_to_main_err.send(Msg::Error(e));
            }
        });

        Ok(())
    }
}
