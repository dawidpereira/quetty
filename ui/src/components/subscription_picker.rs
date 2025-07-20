use crate::components::base_popup::PopupBuilder;
use crate::components::common::{Msg, SubscriptionSelectionMsg};
use crate::theme::ThemeManager;
use quetty_server::service_bus_manager::azure_management_client::Subscription;
use tuirealm::command::{Cmd, CmdResult};
use tuirealm::event::{Event, Key, KeyEvent, NoUserEvent};
use tuirealm::props::TextModifiers;
use tuirealm::ratatui::layout::Rect;
use tuirealm::ratatui::style::Style;
use tuirealm::ratatui::widgets::{List, ListItem};
use tuirealm::{AttrValue, Attribute, Component, Frame, MockComponent, State, StateValue};

pub struct SubscriptionPicker {
    subscriptions: Vec<Subscription>,
    selected: usize,
}

impl SubscriptionPicker {
    pub fn new(subscriptions: Option<Vec<Subscription>>) -> Self {
        Self {
            subscriptions: subscriptions.unwrap_or_default(),
            selected: 0,
        }
    }
}

impl MockComponent for SubscriptionPicker {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .subscriptions
            .iter()
            .enumerate()
            .map(|(i, sub)| {
                let subscription_text =
                    format!("📋 {} ({})", sub.display_name, sub.subscription_id);
                let mut item = ListItem::new(subscription_text);
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
            .collect();

        let popup_block = PopupBuilder::new("Subscription Picker")
            .create_block_with_title("  📋 Select Azure Subscription  ");

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
        if let Some(sub) = self.subscriptions.get(self.selected) {
            State::One(StateValue::String(sub.subscription_id.clone()))
        } else {
            State::None
        }
    }

    fn perform(&mut self, _cmd: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for SubscriptionPicker {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => Some(Msg::SubscriptionSelection(
                SubscriptionSelectionMsg::CancelSelection,
            )),
            Event::Keyboard(KeyEvent {
                code: Key::Enter, ..
            }) => self.subscriptions.get(self.selected).map(|sub| {
                Msg::SubscriptionSelection(SubscriptionSelectionMsg::SubscriptionSelected(
                    sub.subscription_id.clone(),
                ))
            }),
            Event::Keyboard(KeyEvent {
                code: Key::Up | Key::Char('k'),
                ..
            }) => {
                if self.selected > 0 {
                    self.selected -= 1;
                    Some(Msg::SubscriptionSelection(
                        SubscriptionSelectionMsg::SelectionChanged,
                    ))
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Down | Key::Char('j'),
                ..
            }) => {
                if self.selected < self.subscriptions.len().saturating_sub(1) {
                    self.selected += 1;
                    Some(Msg::SubscriptionSelection(
                        SubscriptionSelectionMsg::SelectionChanged,
                    ))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
