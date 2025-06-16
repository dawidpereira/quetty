use crate::components::common::{Msg, PopupActivityMsg};
use crate::components::state::ComponentState;
use crate::theme::ThemeManager;
use tui_realm_stdlib::Paragraph;
use tuirealm::{
    Component, Event, MockComponent, NoUserEvent,
    event::{Key, KeyEvent},
    props::{Alignment, BorderType, Borders, TextModifiers, TextSpan},
    ratatui::{
        Frame,
        layout::Rect,
        text::{Line, Text},
        widgets::{Block, Paragraph as RatatuiParagraph, Wrap},
    },
};

pub struct SuccessPopup {
    component: Paragraph,
    message: String,
    is_mounted: bool,
}

impl SuccessPopup {
    pub fn new(message: &str) -> Self {
        Self {
            component: Paragraph::default()
                .borders(
                    Borders::default()
                        .color(ThemeManager::status_success())
                        .modifiers(BorderType::Rounded),
                )
                .title(" ✅ Success ", Alignment::Center)
                .foreground(ThemeManager::status_success())
                .modifiers(TextModifiers::BOLD)
                .alignment(Alignment::Center)
                .text([TextSpan::from(message)]),
            message: message.to_string(),
            is_mounted: false,
        }
    }
}

impl MockComponent for SuccessPopup {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Create the border block
        let block = Block::default()
            .borders(tuirealm::ratatui::widgets::Borders::ALL)
            .border_type(tuirealm::ratatui::widgets::BorderType::Rounded)
            .border_style(
                tuirealm::ratatui::style::Style::default().fg(ThemeManager::status_success()),
            )
            .title(" ✅ Success ")
            .title_alignment(tuirealm::ratatui::layout::Alignment::Center)
            .title_style(
                tuirealm::ratatui::style::Style::default()
                    .fg(ThemeManager::status_success())
                    .add_modifier(tuirealm::ratatui::style::Modifier::BOLD),
            );

        // Split the message into lines and create centered text
        let mut lines = Vec::new();

        // Add empty line at the top for better spacing
        lines.push(Line::from(""));

        for line in self.message.lines() {
            lines.push(Line::from(line).alignment(tuirealm::ratatui::layout::Alignment::Center));
        }

        let text = Text::from(lines);

        // Create the paragraph with custom text and word wrapping
        let paragraph = RatatuiParagraph::new(text)
            .block(block)
            .alignment(tuirealm::ratatui::layout::Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(
                tuirealm::ratatui::style::Style::default()
                    .fg(ThemeManager::status_success())
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

impl Component<Msg, NoUserEvent> for SuccessPopup {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Enter | Key::Esc,
                ..
            }) => Some(Msg::PopupActivity(PopupActivityMsg::CloseSuccess)),
            _ => None,
        }
    }
}

impl ComponentState for SuccessPopup {
    fn mount(&mut self) -> crate::error::AppResult<()> {
        log::debug!("Mounting SuccessPopup component");

        if self.is_mounted {
            log::warn!("SuccessPopup is already mounted");
            return Ok(());
        }

        self.is_mounted = true;

        log::debug!("SuccessPopup component mounted successfully");
        Ok(())
    }
}

impl Drop for SuccessPopup {
    fn drop(&mut self) {
        log::debug!("Dropping SuccessPopup component");
        self.is_mounted = false;
        log::debug!("SuccessPopup component dropped");
    }
}
