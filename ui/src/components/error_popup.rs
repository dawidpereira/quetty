use crate::components::common::{Msg, PopupActivityMsg};
use crate::error::AppError;
use crate::theme::ThemeManager;
use tui_realm_stdlib::Paragraph;
use tuirealm::{
    Component, Event, MockComponent, NoUserEvent,
    event::{Key, KeyEvent},
    props::{Alignment, BorderType, Borders, TextModifiers, TextSpan},
};

#[derive(MockComponent)]
pub struct ErrorPopup {
    component: Paragraph,
}

impl ErrorPopup {
    pub fn new(error: &AppError) -> Self {
        // Format error message
        let error_msg = format!("{}", error);

        // Split the error message by newlines and create TextSpan for each line
        let text_spans: Vec<TextSpan> = error_msg.lines().map(TextSpan::from).collect();

        Self {
            component: Paragraph::default()
                .borders(
                    Borders::default()
                        .color(ThemeManager::status_error())
                        .modifiers(BorderType::Rounded),
                )
                .title(" ❌ Error ", Alignment::Center)
                .foreground(ThemeManager::status_error())
                .modifiers(TextModifiers::BOLD)
                .alignment(Alignment::Center)
                .text(text_spans),
        }
    }
}

impl Component<Msg, NoUserEvent> for ErrorPopup {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Enter | Key::Esc,
                ..
            }) => Some(Msg::PopupActivity(PopupActivityMsg::CloseError)),
            _ => None,
        }
    }
}
