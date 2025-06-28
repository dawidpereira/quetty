use crate::components::base_popup::PopupBuilder;
use crate::components::common::{Msg, NamespaceActivityMsg};
use crate::config::{self};
use crate::theme::ThemeManager;
use tuirealm::command::{Cmd, CmdResult};
use tuirealm::event::{Event, Key, KeyEvent, NoUserEvent};
use tuirealm::props::TextModifiers;
use tuirealm::ratatui::layout::Rect;
use tuirealm::ratatui::style::Style;
use tuirealm::ratatui::widgets::{List, ListItem};
use tuirealm::{AttrValue, Attribute, Component, Frame, MockComponent, State, StateValue};

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
        // Calculate consistent width for namespace icons and names
        let formatted_items: Vec<String> = self
            .namespaces
            .iter()
            .map(|ns| format!("🏢 {}", ns))
            .collect();

        // Find maximum width needed for proper alignment
        let max_width = formatted_items
            .iter()
            .map(|item| item.len())
            .max()
            .unwrap_or(30);
        let padding_width = max_width + 4;

        let items: Vec<ListItem> = self
            .namespaces
            .iter()
            .enumerate()
            .map(|(i, ns)| {
                let namespace_text =
                    format!("{:width$}", format!("🏢 {}", ns), width = padding_width);
                let mut item = ListItem::new(namespace_text);
                if i == self.selected {
                    item = item.style(
                        Style::default()
                            .fg(ThemeManager::namespace_list_item())
                            .bg(ThemeManager::surface())
                            .add_modifier(TextModifiers::BOLD),
                    );
                } else {
                    item = item.style(Style::default().fg(ThemeManager::namespace_list_item()));
                }
                item
            })
            .collect();
        // Use PopupBuilder for consistent styling
        let popup_block = PopupBuilder::new("Namespace Picker")
            .create_block_with_title("  🌐 Select a Namespace  ");

        let list = List::new(items)
            .block(popup_block)
            .highlight_style(
                Style::default()
                    .fg(ThemeManager::namespace_list_item())
                    .bg(ThemeManager::surface())
                    .add_modifier(TextModifiers::BOLD),
            )
            .highlight_symbol("▶ ");
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
                code: Key::Down, ..
            }) => {
                if self.selected + 1 < self.namespaces.len() {
                    self.selected += 1;
                }
                CmdResult::Changed(State::One(StateValue::Usize(self.selected)))
            }
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                CmdResult::Changed(State::One(StateValue::Usize(self.selected)))
            }
            Event::Keyboard(KeyEvent {
                code: Key::Enter, ..
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
            Event::Keyboard(KeyEvent {
                code: Key::Char(c), ..
            }) => {
                let keys = config::get_config_or_panic().keys();
                if c == keys.down() {
                    if self.selected + 1 < self.namespaces.len() {
                        self.selected += 1;
                    }
                    CmdResult::Changed(State::One(StateValue::Usize(self.selected)))
                } else if c == keys.up() {
                    if self.selected > 0 {
                        self.selected -= 1;
                    }
                    CmdResult::Changed(State::One(StateValue::Usize(self.selected)))
                } else if c == keys.namespace_select() {
                    if let Some(ns) = self.namespaces.get(self.selected).cloned() {
                        CmdResult::Custom(
                            CMD_RESULT_NAMESPACE_SELECTED,
                            State::One(StateValue::String(ns)),
                        )
                    } else {
                        CmdResult::None
                    }
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
