use tuirealm::props::{Alignment, Color};
use tuirealm::ratatui::layout::Rect;
use tuirealm::ratatui::style::Style;
use tuirealm::ratatui::text::{Line, Span, Text};
use tuirealm::{Component, Event, Frame, MockComponent, NoUserEvent};

use crate::components::common::{ComponentId, Msg, QueueType};

/// Help bar that shows keyboard shortcuts based on the current active component
pub struct HelpBar {
    style: Style,
}

impl HelpBar {
    pub fn new() -> Self {
        Self {
            style: Style::default().fg(Color::White).bg(Color::DarkGray),
        }
    }

    /// Get the global shortcuts that are always available
    fn get_global_shortcuts(&self) -> Vec<(String, bool)> {
        vec![
            ("[h]".to_string(), true),
            (" Help ".to_string(), false),
            ("[q]".to_string(), true),
            (" Quit".to_string(), false),
        ]
    }

    /// Get context-specific shortcuts for a given component
    fn get_context_shortcuts(
        &self,
        active_component: &ComponentId,
        queue_type: Option<&QueueType>,
        bulk_mode: Option<bool>,
        selected_count: Option<usize>,
    ) -> Vec<(String, bool)> {
        match active_component {
            ComponentId::Messages => {
                let mut shortcuts = vec![
                    ("[↑/↓]".to_string(), true),
                    (" Navigate ".to_string(), false),
                    ("[Enter]".to_string(), true),
                    (" View ".to_string(), false),
                ];

                // Add bulk mode shortcuts - simplified
                if bulk_mode.unwrap_or(false) {
                    shortcuts.push(("[Space]".to_string(), true));
                    shortcuts.push((" Select ".to_string(), false));
                    
                    if selected_count.unwrap_or(0) > 0 {
                        shortcuts.push(("[Del]".to_string(), true));
                        shortcuts.push((" Delete ".to_string(), false));
                        shortcuts.push(("[Esc]".to_string(), true));
                        shortcuts.push((" Clear ".to_string(), false));
                    } else {
                        shortcuts.push(("[Ctrl+A]".to_string(), true));
                        shortcuts.push((" All ".to_string(), false));
                        shortcuts.push(("[Esc]".to_string(), true));
                        shortcuts.push((" Exit ".to_string(), false));
                    }
                } else {
                    shortcuts.push(("[Space]".to_string(), true));
                    shortcuts.push((" Bulk ".to_string(), false));
                    shortcuts.push(("[n/p]".to_string(), true));
                    shortcuts.push((" Page ".to_string(), false));
                }

                // Add queue toggle - simplified
                if let Some(queue_type) = queue_type {
                    match queue_type {
                        QueueType::Main => {
                            shortcuts.push(("[d]".to_string(), true));
                            shortcuts.push((" DLQ".to_string(), false));
                        }
                        QueueType::DeadLetter => {
                            shortcuts.push(("[d]".to_string(), true));
                            shortcuts.push((" Main".to_string(), false));
                        }
                    }
                }

                shortcuts
            }
            ComponentId::MessageDetails => vec![
                ("[←/→]".to_string(), true),
                (" Move ".to_string(), false),
                ("[↑/↓]".to_string(), true),
                (" Scroll ".to_string(), false),
                ("[Esc]".to_string(), true),
                (" Back ".to_string(), false),
            ],
            ComponentId::QueuePicker => vec![
                ("[↑/↓]".to_string(), true),
                (" Navigate ".to_string(), false),
                ("[Enter]".to_string(), true),
                (" Select ".to_string(), false),
                ("[Esc]".to_string(), true),
                (" Back ".to_string(), false),
            ],
            ComponentId::NamespacePicker => vec![
                ("[↑/↓]".to_string(), true),
                (" Navigate ".to_string(), false),
                ("[Enter]".to_string(), true),
                (" Select ".to_string(), false),
            ],
            ComponentId::ErrorPopup => vec![
                ("[Enter/Esc]".to_string(), true),
                (" Close ".to_string(), false),
            ],
            ComponentId::ConfirmationPopup => vec![
                ("[Y]".to_string(), true),
                (" Yes ".to_string(), false),
                ("[N/Esc]".to_string(), true),
                (" No ".to_string(), false),
            ],
            _ => vec![],
        }
    }

    /// Combine context-specific and global shortcuts
    fn get_help_text(
        &self,
        active_component: &ComponentId,
        queue_type: Option<&QueueType>,
        bulk_mode: Option<bool>,
        selected_count: Option<usize>,
    ) -> Vec<(String, bool)> {
        let mut shortcuts =
            self.get_context_shortcuts(active_component, queue_type, bulk_mode, selected_count);
        let global_shortcuts = self.get_global_shortcuts();

        // Add global shortcuts
        shortcuts.extend(global_shortcuts);

        shortcuts
    }

    pub fn view_with_active(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        active_component: &ComponentId,
    ) {
        self.view_with_active_and_queue_type(frame, area, active_component, None, None, None);
    }

    pub fn view_with_active_and_queue_type(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        active_component: &ComponentId,
        queue_type: Option<&QueueType>,
        bulk_mode: Option<bool>,
        selected_count: Option<usize>,
    ) {
        let help_text = self.get_help_text(active_component, queue_type, bulk_mode, selected_count);
        let mut spans: Vec<Span> = Vec::new();

        // Add each shortcut pair with separators
        for (i, (text, highlight)) in help_text.iter().enumerate() {
            // Add separator before each pair (except the first one)
            if i > 0 && i % 2 == 0 {
                spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
            }

            // Add the shortcut text
            if *highlight {
                spans.push(Span::styled(
                    text.clone(),
                    Style::default().fg(Color::Yellow),
                ));
            } else {
                spans.push(Span::raw(text.clone()));
            }
        }

        let paragraph = tuirealm::ratatui::widgets::Paragraph::new(Text::from(Line::from(spans)))
            .style(self.style)
            .alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
    }
}

impl MockComponent for HelpBar {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Default view with no active component
        self.view_with_active(frame, area, &ComponentId::Label);
    }

    fn query(&self, _attr: tuirealm::Attribute) -> Option<tuirealm::AttrValue> {
        None
    }

    fn attr(&mut self, _attr: tuirealm::Attribute, _value: tuirealm::AttrValue) {}

    fn state(&self) -> tuirealm::State {
        tuirealm::State::None
    }

    fn perform(&mut self, _cmd: tuirealm::command::Cmd) -> tuirealm::command::CmdResult {
        tuirealm::command::CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for HelpBar {
    fn on(&mut self, _ev: Event<NoUserEvent>) -> Option<Msg> {
        None
    }
}
