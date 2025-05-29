use tui_realm_stdlib::Paragraph;
use tuirealm::{
    Component, Event, MockComponent, NoUserEvent,
    event::{Key, KeyEvent},
    props::{Alignment, BorderType, Borders, Color, TextModifiers, TextSpan},
    ratatui::{
        Frame,
        layout::Rect,
        text::{Line, Span, Text},
        widgets::{Block, Paragraph as RatatuiParagraph},
    },
};

use crate::components::common::{Msg, PopupActivityMsg};

pub struct ConfirmationPopup {
    component: Paragraph,
    title: String,
    message: String,
}

impl ConfirmationPopup {
    pub fn new(title: &str, message: &str) -> Self {
        // Store the title and message for custom rendering
        Self {
            component: Paragraph::default()
                .borders(
                    Borders::default()
                        .color(Color::Yellow)
                        .modifiers(BorderType::Rounded),
                )
                .title(format!(" {} ", title), Alignment::Center)
                .foreground(Color::Yellow)
                .modifiers(TextModifiers::BOLD)
                .alignment(Alignment::Center)
                .text(&[TextSpan::from(message)]),
            title: title.to_string(),
            message: message.to_string(),
        }
    }
}

impl MockComponent for ConfirmationPopup {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Create the border block with dynamic title
        let block = Block::default()
            .borders(tuirealm::ratatui::widgets::Borders::ALL)
            .border_type(tuirealm::ratatui::widgets::BorderType::Rounded)
            .border_style(
                tuirealm::ratatui::style::Style::default()
                    .fg(tuirealm::ratatui::style::Color::Yellow),
            )
            .title(format!(" {} ", self.title))
            .title_alignment(tuirealm::ratatui::layout::Alignment::Center);

        // Split the message into lines and create text
        let mut lines = Vec::new();
        for line in self.message.lines() {
            lines.push(Line::from(line));
        }
        
        // Add empty line for spacing
        lines.push(Line::from(""));
        
        // Add the options
        lines.push(Line::from(vec![
            Span::styled(
                "[Y] Yes",
                tuirealm::ratatui::style::Style::default()
                    .fg(tuirealm::ratatui::style::Color::Green)
                    .add_modifier(tuirealm::ratatui::style::Modifier::BOLD),
            ),
            Span::raw("    "),
            Span::styled(
                "[N] No",
                tuirealm::ratatui::style::Style::default()
                    .fg(tuirealm::ratatui::style::Color::Red)
                    .add_modifier(tuirealm::ratatui::style::Modifier::BOLD),
            ),
        ]));

        let text = Text::from(lines);

        // Create the paragraph with custom text
        let paragraph = RatatuiParagraph::new(text)
            .block(block)
            .alignment(tuirealm::ratatui::layout::Alignment::Center)
            .style(
                tuirealm::ratatui::style::Style::default()
                    .fg(tuirealm::ratatui::style::Color::Yellow)
                    .add_modifier(tuirealm::ratatui::style::Modifier::BOLD),
            );

        frame.render_widget(paragraph, area);
    }

    fn query(&self, attr: tuirealm::Attribute) -> Option<tuirealm::AttrValue> {
        self.component.query(attr)
    }

    fn attr(&mut self, attr: tuirealm::Attribute, value: tuirealm::AttrValue) {
        self.component.attr(attr, value);
    }

    fn state(&self) -> tuirealm::State {
        self.component.state()
    }

    fn perform(&mut self, cmd: tuirealm::command::Cmd) -> tuirealm::command::CmdResult {
        self.component.perform(cmd)
    }
}

impl Component<Msg, NoUserEvent> for ConfirmationPopup {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('y') | Key::Char('Y'),
                ..
            }) => Some(Msg::PopupActivity(
                PopupActivityMsg::ConfirmationResult(true),
            )),
            Event::Keyboard(KeyEvent {
                code: Key::Char('n') | Key::Char('N') | Key::Esc,
                ..
            }) => Some(Msg::PopupActivity(
                PopupActivityMsg::ConfirmationResult(false),
            )),
            _ => None,
        }
    }
}
