use crate::components::base_popup::PopupBuilder;
use crate::components::common::{Msg, ResourceGroupSelectionMsg};
use crate::theme::ThemeManager;
use quetty_server::service_bus_manager::azure_management_client::ResourceGroup;
use tuirealm::command::{Cmd, CmdResult};
use tuirealm::event::{Event, Key, KeyEvent, NoUserEvent};
use tuirealm::props::TextModifiers;
use tuirealm::ratatui::layout::Rect;
use tuirealm::ratatui::style::Style;
use tuirealm::ratatui::widgets::{List, ListItem};
use tuirealm::{AttrValue, Attribute, Component, Frame, MockComponent, State, StateValue};

pub struct ResourceGroupPicker {
    resource_groups: Vec<ResourceGroup>,
    selected: usize,
}

impl ResourceGroupPicker {
    pub fn new(resource_groups: Option<Vec<ResourceGroup>>) -> Self {
        Self {
            resource_groups: resource_groups.unwrap_or_default(),
            selected: 0,
        }
    }
}

impl MockComponent for ResourceGroupPicker {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = if self.resource_groups.is_empty() {
            vec![
                ListItem::new("⚠️  No resource groups found")
                    .style(Style::default().fg(ThemeManager::text_muted())),
                ListItem::new(""),
                ListItem::new("This may be due to limited permissions.")
                    .style(Style::default().fg(ThemeManager::text_muted())),
                ListItem::new("Press ESC to go back to subscription selection.")
                    .style(Style::default().fg(ThemeManager::text_muted())),
            ]
        } else {
            self.resource_groups
                .iter()
                .enumerate()
                .map(|(i, group)| {
                    let group_text = format!("📁 {} ({})", group.name, group.location);
                    let mut item = ListItem::new(group_text);
                    if i == self.selected {
                        item = item.style(
                            Style::default()
                                .fg(ThemeManager::primary_accent())
                                .bg(ThemeManager::surface())
                                .add_modifier(TextModifiers::BOLD),
                        );
                    } else {
                        item = item.style(Style::default().fg(ThemeManager::text_primary()));
                    }
                    item
                })
                .collect()
        };

        let popup_block = PopupBuilder::new("Resource Group Picker")
            .create_block_with_title("  📁 Select Resource Group  ");

        let list = List::new(items)
            .block(popup_block)
            .highlight_style(
                Style::default()
                    .fg(ThemeManager::primary_accent())
                    .bg(ThemeManager::surface())
                    .add_modifier(TextModifiers::BOLD),
            )
            .highlight_symbol("▶ ");
        frame.render_widget(list, area);
    }

    fn query(&self, _attr: Attribute) -> Option<AttrValue> {
        None
    }

    fn attr(&mut self, _attr: Attribute, _value: AttrValue) {}

    fn state(&self) -> State {
        if let Some(group) = self.resource_groups.get(self.selected) {
            State::One(StateValue::String(group.name.clone()))
        } else {
            State::None
        }
    }

    fn perform(&mut self, _cmd: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for ResourceGroupPicker {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => Some(Msg::ResourceGroupSelection(
                ResourceGroupSelectionMsg::CancelSelection,
            )),
            Event::Keyboard(KeyEvent {
                code: Key::Enter, ..
            }) => self.resource_groups.get(self.selected).map(|group| {
                Msg::ResourceGroupSelection(ResourceGroupSelectionMsg::ResourceGroupSelected(
                    group.name.clone(),
                ))
            }),
            Event::Keyboard(KeyEvent {
                code: Key::Up | Key::Char('k'),
                ..
            }) => {
                if self.selected > 0 {
                    self.selected -= 1;
                    Some(Msg::ResourceGroupSelection(
                        ResourceGroupSelectionMsg::SelectionChanged,
                    ))
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Down | Key::Char('j'),
                ..
            }) => {
                if self.selected < self.resource_groups.len().saturating_sub(1) {
                    self.selected += 1;
                    Some(Msg::ResourceGroupSelection(
                        ResourceGroupSelectionMsg::SelectionChanged,
                    ))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
