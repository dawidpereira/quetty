use azservicebus::ServiceBusReceiverOptions;
use server::consumer::ServiceBusClientExt;
use tuirealm::command::CmdResult;
use tuirealm::event::{Key, KeyEvent};
use tuirealm::props::{Alignment, Color, Style, TextModifiers};
use tuirealm::ratatui::layout::Rect;
use tuirealm::ratatui::widgets::{List, ListItem};
use tuirealm::terminal::TerminalAdapter;
use tuirealm::{Component, Event, Frame, MockComponent, NoUserEvent};

use crate::app::model::Model;

use super::common::{MessageActivityMsg, Msg, QueueActivityMsg};

const CMD_RESULT_QUEUE_SELECTED: &str = "QueueSelected";

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
                code: Key::Enter, ..
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
            CmdResult::Changed(_) => Some(Msg::ForceRedraw),
            _ => Some(Msg::ForceRedraw),
        }
    }
}

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn new_consumer_for_queue(&mut self) {
        //TODO: Error handling
        let queue = self.pending_queue.take().expect("No queue selected");
        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        // Clone the Arc to pass into async block
        let service_bus_client = self.service_bus_client.clone();
        let consumer = self.consumer.clone();

        taskpool.execute(async move {
            // Create a new consumer using the service bus client
            if let Some(consumer) = consumer {
                consumer.lock().await.dispose().await.unwrap();
            }

            let mut client = service_bus_client.lock().await;
            let consumer = client
                .create_consumer_for_queue(queue, ServiceBusReceiverOptions::default())
                .await
                .unwrap();

            // Send the consumer back to the main thread
            let _ = tx_to_main.send(Msg::MessageActivity(MessageActivityMsg::ConsumerCreated(
                consumer,
            )));
        });
    }
}
