use tuirealm::command::CmdResult;
use tuirealm::event::{Key, KeyEvent, KeyModifiers};
use tuirealm::props::{Alignment, Color, Style, TextModifiers};
use tuirealm::ratatui::layout::Rect;
use tuirealm::ratatui::widgets::{List, ListItem};
use tuirealm::{Component, Event, Frame, MockComponent, NoUserEvent};

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
    fn query(&self, _attr: tuirealm::Attribute) -> Option<tuirealm::AttrValue> { None }
    fn attr(&mut self, _attr: tuirealm::Attribute, _value: tuirealm::AttrValue) {}
    fn state(&self) -> tuirealm::State {
        if let Some(ns) = self.namespaces.get(self.selected) {
            tuirealm::State::One(tuirealm::StateValue::String(ns.clone()))
        } else {
            tuirealm::State::None
        }
    }
    fn perform(&mut self, _cmd: tuirealm::command::Cmd) -> tuirealm::command::CmdResult {
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
                CmdResult::Changed(tuirealm::State::One(tuirealm::StateValue::Usize(self.selected)))
            }
            Event::Keyboard(KeyEvent {
                code: Key::Up | Key::Char('k'),
                ..
            }) => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                CmdResult::Changed(tuirealm::State::One(tuirealm::StateValue::Usize(self.selected)))
            }
            Event::Keyboard(KeyEvent {
                code: Key::Enter, ..
            }) => {
                if let Some(ns) = self.namespaces.get(self.selected).cloned() {
                    CmdResult::Custom(
                        CMD_RESULT_NAMESPACE_SELECTED,
                        tuirealm::State::One(tuirealm::StateValue::String(ns)),
                    )
                } else {
                    CmdResult::None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Esc, ..
            }) => {
                CmdResult::Custom("NamespacePickerBack", tuirealm::State::None)
            }
            _ => CmdResult::None,
        };

        match cmd_result {
            CmdResult::Custom(CMD_RESULT_NAMESPACE_SELECTED, state) => {
                if let tuirealm::State::One(tuirealm::StateValue::String(ns)) = state {
                    Some(Msg::NamespaceActivity(NamespaceActivityMsg::NamespaceSelected(ns)))
                } else {
                    None
                }
            }
            CmdResult::Custom("NamespacePickerBack", _) => {
                Some(Msg::AppClose) // or a custom back message
            }
            CmdResult::Changed(_) => Some(Msg::ForceRedraw),
            _ => Some(Msg::ForceRedraw),
        }
    }
} 