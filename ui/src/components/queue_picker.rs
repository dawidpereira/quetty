use crate::components::base_popup::PopupBuilder;
use crate::components::common::{AzureDiscoveryMsg, Msg, NamespaceActivityMsg, QueueActivityMsg};
use crate::config;
use crate::theme::ThemeManager;
use tuirealm::command::CmdResult;
use tuirealm::event::{Event, Key, KeyEvent, NoUserEvent};
use tuirealm::props::TextModifiers;
use tuirealm::ratatui::layout::Rect;
use tuirealm::ratatui::style::Style;
use tuirealm::ratatui::widgets::{List, ListItem};
use tuirealm::{Component, Frame, MockComponent};

const CMD_RESULT_QUEUE_SELECTED: &str = "QueueSelected";
const CMD_RESULT_NAMESPACE_UNSELECTED: &str = "NamespaceUnselected";

pub struct QueuePicker {
    queues: Vec<String>,
    selected: usize,
    manual_entry_mode: bool,
    manual_queue_name: String,
}

impl QueuePicker {
    pub fn new(queues: Option<Vec<String>>) -> Self {
        Self {
            queues: queues.unwrap_or_default(),
            selected: 0,
            manual_entry_mode: false,
            manual_queue_name: String::new(),
        }
    }
}

impl MockComponent for QueuePicker {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Calculate consistent width for queue icons and names
        let formatted_items: Vec<String> = self
            .queues
            .iter()
            .map(|q| {
                if q.ends_with("/$deadletter") {
                    format!("💀 {}", q.replace("/$deadletter", " (DLQ)"))
                } else {
                    format!("📬 {q}")
                }
            })
            .collect();

        // Find maximum width needed for proper alignment
        let max_width = formatted_items
            .iter()
            .map(|item| item.len())
            .max()
            .unwrap_or(30);
        let padding_width = max_width + 4;

        let items: Vec<ListItem> = self
            .queues
            .iter()
            .enumerate()
            .map(|(i, q)| {
                let queue_text = if q.ends_with("/$deadletter") {
                    format!(
                        "{:width$}",
                        format!("💀 {}", q.replace("/$deadletter", " (DLQ)")),
                        width = padding_width
                    )
                } else {
                    format!("{:width$}", format!("📬 {}", q), width = padding_width)
                };

                let mut item = ListItem::new(queue_text);
                if i == self.selected {
                    item = item.style(
                        Style::default()
                            .fg(ThemeManager::status_info())
                            .bg(ThemeManager::surface())
                            .add_modifier(TextModifiers::BOLD),
                    );
                } else {
                    item = item.style(Style::default().fg(ThemeManager::status_info()));
                }
                item
            })
            .collect();
        // Use PopupBuilder for consistent styling
        let popup_block = PopupBuilder::new("Queue Picker")
            .create_block_with_title("  🗂️  Select a Queue • Press 'd' for Azure Discovery  ");

        if self.manual_entry_mode {
            // Show manual queue entry
            use tuirealm::ratatui::layout::Alignment;
            use tuirealm::ratatui::widgets::Paragraph;

            let help_text = format!(
                "Enter queue name:\n\n{}_\n\n• Press Enter to connect\n• Press ESC to cancel",
                self.manual_queue_name
            );

            let paragraph = Paragraph::new(help_text)
                .block(popup_block)
                .style(Style::default().fg(ThemeManager::status_info()))
                .alignment(Alignment::Center);

            frame.render_widget(paragraph, area);
        } else if self.queues.is_empty() {
            // Show a helpful message when no queues are available with manual entry option
            use tuirealm::ratatui::layout::Alignment;
            use tuirealm::ratatui::widgets::Paragraph;

            let help_text = [
                "",
                "🔍 No queues available for automatic discovery",
                "",
                "📝 Press 'm' to MANUALLY ENTER a queue name",
                "🌐 Press 'd' to change Azure subscription/resource group/namespace",
                "⬅️  Press ESC to go back",
                "",
                "Note: Connection string authentication requires manual queue entry",
            ];

            let paragraph = Paragraph::new(help_text.join("\n"))
                .block(popup_block)
                .style(Style::default().fg(ThemeManager::status_info()))
                .alignment(Alignment::Center);

            frame.render_widget(paragraph, area);
        } else {
            let list = List::new(items)
                .block(popup_block)
                .highlight_style(
                    Style::default()
                        .fg(ThemeManager::status_info())
                        .bg(ThemeManager::surface())
                        .add_modifier(TextModifiers::BOLD),
                )
                .highlight_symbol("▶ ");
            frame.render_widget(list, area);
        }
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
                code: Key::Down, ..
            }) => {
                if !self.manual_entry_mode && self.selected + 1 < self.queues.len() {
                    self.selected += 1;
                    CmdResult::Changed(tuirealm::State::One(tuirealm::StateValue::Usize(
                        self.selected,
                    )))
                } else {
                    CmdResult::None
                }
            }
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => {
                if !self.manual_entry_mode && self.selected > 0 {
                    self.selected -= 1;
                    CmdResult::Changed(tuirealm::State::One(tuirealm::StateValue::Usize(
                        self.selected,
                    )))
                } else {
                    CmdResult::None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Enter, ..
            }) => {
                if self.manual_entry_mode {
                    if !self.manual_queue_name.trim().is_empty() {
                        // Exit manual entry mode before selecting queue
                        self.manual_entry_mode = false;
                        CmdResult::Custom(
                            "QueueSelectedAndExitManualMode",
                            tuirealm::State::One(tuirealm::StateValue::String(
                                self.manual_queue_name.clone(),
                            )),
                        )
                    } else {
                        CmdResult::None
                    }
                } else if let Some(queue) = self.queues.get(self.selected).cloned() {
                    CmdResult::Custom(
                        CMD_RESULT_QUEUE_SELECTED,
                        tuirealm::State::One(tuirealm::StateValue::String(queue)),
                    )
                } else {
                    CmdResult::None
                }
            }
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => {
                if self.manual_entry_mode {
                    self.manual_entry_mode = false;
                    self.manual_queue_name.clear();
                    CmdResult::Custom(
                        "SetEditingMode",
                        tuirealm::State::One(tuirealm::StateValue::Bool(false)),
                    )
                } else {
                    CmdResult::Custom(
                        CMD_RESULT_NAMESPACE_UNSELECTED,
                        tuirealm::State::One(tuirealm::StateValue::String("".to_string())),
                    )
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Backspace,
                ..
            }) => {
                if self.manual_entry_mode {
                    self.manual_queue_name.pop();
                    CmdResult::Changed(tuirealm::State::None)
                } else {
                    CmdResult::None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c), ..
            }) => {
                if self.manual_entry_mode {
                    // In manual entry mode, add character to queue name
                    if self.manual_queue_name.len() < 64 {
                        // Reasonable limit
                        self.manual_queue_name.push(c);
                        CmdResult::Changed(tuirealm::State::None)
                    } else {
                        CmdResult::None
                    }
                } else {
                    let keys = config::get_config_or_panic().keys();
                    if c == 'm' {
                        // Enter manual entry mode
                        self.manual_entry_mode = true;
                        self.manual_queue_name.clear();
                        CmdResult::Custom(
                            "SetEditingMode",
                            tuirealm::State::One(tuirealm::StateValue::Bool(true)),
                        )
                    } else if c == keys.down() {
                        if self.selected + 1 < self.queues.len() {
                            self.selected += 1;
                        }
                        CmdResult::Changed(tuirealm::State::One(tuirealm::StateValue::Usize(
                            self.selected,
                        )))
                    } else if c == keys.up() {
                        if self.selected > 0 {
                            self.selected -= 1;
                        }
                        CmdResult::Changed(tuirealm::State::One(tuirealm::StateValue::Usize(
                            self.selected,
                        )))
                    } else if c == keys.queue_select() {
                        if let Some(queue) = self.queues.get(self.selected).cloned() {
                            CmdResult::Custom(
                                CMD_RESULT_QUEUE_SELECTED,
                                tuirealm::State::One(tuirealm::StateValue::String(queue)),
                            )
                        } else {
                            CmdResult::None
                        }
                    } else if c == 'd' {
                        // Enter Azure discovery mode to select subscription/resource group/namespace
                        log::info!("User pressed 'd' key - starting Azure discovery");
                        CmdResult::Custom("StartAzureDiscovery", tuirealm::State::None)
                    } else {
                        CmdResult::None
                    }
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
            CmdResult::Custom("QueueSelectedAndExitManualMode", state) => {
                if let tuirealm::State::One(tuirealm::StateValue::String(queue)) = state {
                    Some(Msg::QueueActivity(
                        QueueActivityMsg::QueueSelectedFromManualEntry(queue),
                    ))
                } else {
                    None
                }
            }
            CmdResult::Custom(CMD_RESULT_NAMESPACE_UNSELECTED, _) => Some(Msg::NamespaceActivity(
                NamespaceActivityMsg::NamespaceUnselected,
            )),
            CmdResult::Custom("SetEditingMode", state) => {
                if let tuirealm::State::One(tuirealm::StateValue::Bool(editing)) = state {
                    Some(Msg::SetEditingMode(editing))
                } else {
                    None
                }
            }
            CmdResult::Custom("StartAzureDiscovery", _) => Some(Msg::AzureDiscovery(
                AzureDiscoveryMsg::StartInteractiveDiscovery,
            )),
            CmdResult::Changed(_) => Some(Msg::ForceRedraw),
            _ => None,
        }
    }
}
