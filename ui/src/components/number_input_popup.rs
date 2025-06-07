use crate::components::common::{Msg, PopupActivityMsg};
use crate::theme::ThemeManager;
use tuirealm::{
    Component, Event, MockComponent, NoUserEvent, State, StateValue,
    command::{Cmd, CmdResult},
    event::{Key, KeyEvent, KeyModifiers},
    ratatui::{
        Frame,
        layout::{Alignment, Rect},
        style::{Color, Modifier, Style},
        text::{Line, Span, Text},
        widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    },
};

pub struct NumberInputPopup {
    title: String,
    message: String,
    min_value: usize,
    max_value: usize,
    current_input: String,
}

impl NumberInputPopup {
    pub fn new(title: String, message: String, min_value: usize, max_value: usize) -> Self {
        Self {
            title,
            message,
            min_value,
            max_value,
            current_input: String::new(),
        }
    }

    fn validate_and_get_number(&self) -> Option<usize> {
        if self.current_input.is_empty() {
            // Return default value if empty
            return Some(10.max(self.min_value).min(self.max_value));
        }

        if let Ok(num) = self.current_input.parse::<usize>() {
            if num >= self.min_value && num <= self.max_value {
                Some(num)
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl MockComponent for NumberInputPopup {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Create the border block with dynamic title
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(ThemeManager::primary_accent()))
            .title(format!(" {} ", self.title))
            .title_alignment(Alignment::Center);

        // Create content lines
        let mut lines = Vec::new();

        // Add message lines
        for line in self.message.lines() {
            lines.push(Line::from(line));
        }

        lines.push(Line::from(""));

        // Add range info
        lines.push(Line::from(format!(
            "Range: {} to {} (Enter for default: {})",
            self.min_value,
            self.max_value,
            10.max(self.min_value).min(self.max_value)
        )));

        lines.push(Line::from(""));

        // Add input field
        let input_text = if self.current_input.is_empty() {
            "Type a number..."
        } else {
            &self.current_input
        };

        let input_style = if self.current_input.is_empty() {
            Style::default().fg(Color::Gray)
        } else if self.validate_and_get_number().is_some() {
            Style::default().fg(ThemeManager::status_success())
        } else {
            Style::default().fg(ThemeManager::status_error())
        };

        lines.push(Line::from(vec![
            Span::raw("Input: ["),
            Span::styled(input_text, input_style.add_modifier(Modifier::BOLD)),
            Span::raw("]"),
        ]));

        lines.push(Line::from(""));

        // Add instructions
        lines.push(Line::from(vec![
            Span::styled(
                "[Enter]",
                Style::default()
                    .fg(ThemeManager::status_success())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Accept    "),
            Span::styled(
                "[Esc]",
                Style::default()
                    .fg(ThemeManager::status_error())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Cancel"),
        ]));

        let text = Text::from(lines);

        // Create the paragraph
        let paragraph = Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(
                Style::default()
                    .fg(ThemeManager::popup_text())
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(paragraph, area);
    }

    fn query(&self, _attr: tuirealm::Attribute) -> Option<tuirealm::AttrValue> {
        None
    }

    fn attr(&mut self, _attr: tuirealm::Attribute, _value: tuirealm::AttrValue) {
        // No attributes supported
    }

    fn state(&self) -> State {
        State::One(StateValue::String(self.current_input.clone()))
    }

    fn perform(&mut self, _cmd: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for NumberInputPopup {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Esc,
                modifiers: KeyModifiers::NONE,
            }) => {
                // Cancel input
                Some(Msg::PopupActivity(PopupActivityMsg::NumberInputResult(0))) // 0 indicates cancel
            }

            Event::Keyboard(KeyEvent {
                code: Key::Enter,
                modifiers: KeyModifiers::NONE,
            }) => {
                // Accept input
                self.validate_and_get_number()
                    .map(|number| Msg::PopupActivity(PopupActivityMsg::NumberInputResult(number)))
            }

            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) => {
                // Add character if it's a digit and we haven't exceeded reasonable length
                if c.is_ascii_digit() && self.current_input.len() < 4 {
                    self.current_input.push(c);
                    Some(Msg::ForceRedraw)
                } else {
                    None
                }
            }

            Event::Keyboard(KeyEvent {
                code: Key::Backspace,
                modifiers: KeyModifiers::NONE,
            }) => {
                // Remove last character
                self.current_input.pop();
                Some(Msg::ForceRedraw)
            }

            _ => None,
        }
    }
}

