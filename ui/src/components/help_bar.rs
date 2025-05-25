use tuirealm::props::{Alignment, Color, Style};
use tuirealm::ratatui::layout::Rect;
use tuirealm::ratatui::text::{Line, Span, Text};
use tuirealm::{Component, Event, Frame, MockComponent, NoUserEvent};

use super::common::{ComponentId, Msg};

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
    
    fn get_help_text(&self, active_component: &ComponentId) -> Vec<(String, bool)> {
        match active_component {
            ComponentId::Messages => vec![
                ("↑/k".to_string(), true),
                (" Up ".to_string(), false),
                ("↓/j".to_string(), true),
                (" Down ".to_string(), false),
                ("Enter".to_string(), true),
                (" Select ".to_string(), false),
                ("Esc".to_string(), true),
                (" Back ".to_string(), false),
                ("PgUp/PgDn".to_string(), true),
                (" Scroll ".to_string(), false),
                ("q".to_string(), true),
                (" Quit".to_string(), false),
            ],
            ComponentId::MessageDetails => vec![
                ("↑/k".to_string(), true),
                (" Up ".to_string(), false),
                ("↓/j".to_string(), true),
                (" Down ".to_string(), false),
                ("←/→".to_string(), true),
                (" Move cursor ".to_string(), false),
                ("Esc".to_string(), true),
                (" Back ".to_string(), false),
                ("PgUp/PgDn".to_string(), true),
                (" Scroll ".to_string(), false),
                ("q".to_string(), true),
                (" Quit".to_string(), false),
            ],
            ComponentId::QueuePicker => vec![
                ("↑/k".to_string(), true),
                (" Up ".to_string(), false),
                ("↓/j".to_string(), true),
                (" Down ".to_string(), false),
                ("Enter/o".to_string(), true),
                (" Select ".to_string(), false),
                ("Esc".to_string(), true),
                (" Back ".to_string(), false),
                ("q".to_string(), true),
                (" Quit".to_string(), false),
            ],
            ComponentId::NamespacePicker => vec![
                ("↑/k".to_string(), true),
                (" Up ".to_string(), false),
                ("↓/j".to_string(), true),
                (" Down ".to_string(), false),
                ("Enter/o".to_string(), true),
                (" Select ".to_string(), false),
                ("q".to_string(), true),
                (" Quit".to_string(), false),
            ],
            ComponentId::ErrorPopup => vec![
                ("Enter/Esc".to_string(), true),
                (" Close ".to_string(), false),
            ],
            _ => vec![("q".to_string(), true), (" Quit".to_string(), false)],
        }
    }
    
    pub fn view_with_active(&mut self, frame: &mut Frame, area: Rect, active_component: &ComponentId) {
        let help_text = self.get_help_text(active_component);
        let spans: Vec<Span> = help_text
            .iter()
            .map(|(text, highlight)| {
                if *highlight {
                    Span::styled(text.clone(), Style::default().fg(Color::Yellow))
                } else {
                    Span::raw(text.clone())
                }
            })
            .collect();

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

