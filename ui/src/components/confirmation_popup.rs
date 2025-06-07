use crate::components::common::{Msg, PopupActivityMsg};
use crate::config;
use crate::theme::ThemeManager;
use tui_realm_stdlib::Paragraph;
use tuirealm::{
    Component, Event, MockComponent, NoUserEvent,
    event::{Key, KeyEvent},
    props::{Alignment, BorderType, Borders, TextModifiers, TextSpan},
    ratatui::{
        Frame,
        layout::Rect,
        text::{Line, Span, Text},
        widgets::{Block, Paragraph as RatatuiParagraph},
    },
};

pub struct ConfirmationPopup {
    component: Paragraph,
    title: String,
    message: String,
}

impl ConfirmationPopup {
    pub fn new(title: &str, message: &str) -> Self {
        let theme = ThemeManager::global();

        Self {
            component: Paragraph::default()
                .borders(
                    Borders::default()
                        .color(theme.popup_border())
                        .modifiers(BorderType::Rounded),
                )
                .title(format!(" {} ", title), Alignment::Center)
                .foreground(theme.popup_text())
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
        let theme = ThemeManager::global();

        // Create the border block with dynamic title
        let block = Block::default()
            .borders(tuirealm::ratatui::widgets::Borders::ALL)
            .border_type(tuirealm::ratatui::widgets::BorderType::Rounded)
            .border_style(tuirealm::ratatui::style::Style::default().fg(theme.popup_border()))
            .title(format!(" {} ", self.title))
            .title_alignment(tuirealm::ratatui::layout::Alignment::Center);

        // Split the message into lines and create text
        let mut lines = Vec::new();
        for line in self.message.lines() {
            lines.push(Line::from(line));
        }

        // Add empty line for spacing
        lines.push(Line::from(""));

        let keys = config::CONFIG.keys();
        lines.push(Line::from(vec![
            Span::styled(
                format!("[{}] Yes", keys.confirm_yes().to_uppercase()),
                tuirealm::ratatui::style::Style::default()
                    .fg(theme.status_success())
                    .add_modifier(tuirealm::ratatui::style::Modifier::BOLD),
            ),
            Span::raw("    "),
            Span::styled(
                format!("[{}] No", keys.confirm_no().to_uppercase()),
                tuirealm::ratatui::style::Style::default()
                    .fg(theme.status_error())
                    .add_modifier(tuirealm::ratatui::style::Modifier::BOLD),
            ),
        ]));

        let text = Text::from(lines);

        // Create the paragraph with custom text and word wrapping
        let paragraph = RatatuiParagraph::new(text)
            .block(block)
            .alignment(tuirealm::ratatui::layout::Alignment::Center)
            .wrap(tuirealm::ratatui::widgets::Wrap { trim: true })
            .style(
                tuirealm::ratatui::style::Style::default()
                    .fg(theme.popup_text())
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
                code: Key::Char(c), ..
            }) => {
                let keys = config::CONFIG.keys();
                let c_lower = c.to_lowercase().next().unwrap_or(c);
                let yes_key = keys
                    .confirm_yes()
                    .to_lowercase()
                    .next()
                    .unwrap_or(keys.confirm_yes());
                let no_key = keys
                    .confirm_no()
                    .to_lowercase()
                    .next()
                    .unwrap_or(keys.confirm_no());

                if c_lower == yes_key {
                    Some(Msg::PopupActivity(PopupActivityMsg::ConfirmationResult(
                        true,
                    )))
                } else if c_lower == no_key {
                    Some(Msg::PopupActivity(PopupActivityMsg::ConfirmationResult(
                        false,
                    )))
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => Some(Msg::PopupActivity(
                PopupActivityMsg::ConfirmationResult(false),
            )),
            _ => None,
        }
    }
}
