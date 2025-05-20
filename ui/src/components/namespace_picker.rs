use server::service_bus_manager::ServiceBusManager;
use tuirealm::command::{Cmd, CmdResult};
use tuirealm::event::{Key, KeyEvent};
use tuirealm::props::{Alignment, Color, Style, TextModifiers};
use tuirealm::ratatui::layout::Rect;
use tuirealm::ratatui::widgets::{List, ListItem};
use tuirealm::terminal::TerminalAdapter;
use tuirealm::{
    AttrValue, Attribute, Component, Event, Frame, MockComponent, NoUserEvent, State, StateValue,
};

use crate::app::model::Model;
use crate::config::CONFIG;

use super::common::{Msg, NamespaceActivityMsg};

const CMD_RESULT_NAMESPACE_SELECTED: &str = "NamespaceSelected";

pub struct NamespacePicker {
    namespaces: Vec<String>,
    selected: usize,
}

impl NamespacePicker {
    pub fn new(namespaces: Option<Vec<String>>) -> Self {
        Self {
            namespaces: namespaces.unwrap_or_default(),
            selected: 0,
        }
    }
}

impl MockComponent for NamespacePicker {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .namespaces
            .iter()
            .enumerate()
            .map(|(i, ns)| {
                let mut item = ListItem::new(ns.clone());
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
                    .title(" Select a namespace ")
                    .title_alignment(Alignment::Center),
            )
            .highlight_style(Style::default().fg(Color::Yellow))
            .highlight_symbol("> ");
        frame.render_widget(list, area);
    }
    fn query(&self, _attr: Attribute) -> Option<AttrValue> {
        None
    }
    fn attr(&mut self, _attr: tuirealm::Attribute, _value: tuirealm::AttrValue) {}
    fn state(&self) -> tuirealm::State {
        if let Some(ns) = self.namespaces.get(self.selected) {
            tuirealm::State::One(tuirealm::StateValue::String(ns.clone()))
        } else {
            tuirealm::State::None
        }
    }
    fn perform(&mut self, _cmd: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for NamespacePicker {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let cmd_result = match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Down | Key::Char('j'),
                ..
            }) => {
                if self.selected + 1 < self.namespaces.len() {
                    self.selected += 1;
                }
                CmdResult::Changed(State::One(StateValue::Usize(self.selected)))
            }
            Event::Keyboard(KeyEvent {
                code: Key::Up | Key::Char('k'),
                ..
            }) => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                CmdResult::Changed(State::One(StateValue::Usize(self.selected)))
            }
            Event::Keyboard(KeyEvent {
                code: Key::Enter | Key::Char('o'),
                ..
            }) => {
                if let Some(ns) = self.namespaces.get(self.selected).cloned() {
                    CmdResult::Custom(
                        CMD_RESULT_NAMESPACE_SELECTED,
                        State::One(StateValue::String(ns)),
                    )
                } else {
                    CmdResult::None
                }
            }
            _ => CmdResult::None,
        };

        match cmd_result {
            CmdResult::Custom(CMD_RESULT_NAMESPACE_SELECTED, state) => {
                if let State::One(StateValue::String(_)) = state {
                    Some(Msg::NamespaceActivity(
                        NamespaceActivityMsg::NamespaceSelected,
                    ))
                } else {
                    None
                }
            }
            _ => Some(Msg::ForceRedraw),
        }
    }
}

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn load_namespaces(&mut self) {
        //TODO: Error handling
        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        // Clone the Arc to pass into async block
        taskpool.execute(async move {
            let namespaces = ServiceBusManager::list_namespaces_azure_ad(CONFIG.azure_ad()).await;
            let _ = tx_to_main.send(Msg::NamespaceActivity(
                NamespaceActivityMsg::NamespacesLoaded(namespaces.unwrap_or_default()),
            ));
        });
    }
}
