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
        let theme = ThemeManager::global();

        Self {
            component: Paragraph::default()
                .borders(
                    Borders::default()
                        .color(theme.status_error())
                        .modifiers(BorderType::Rounded),
                )
                .title(" ❌ Error ", Alignment::Center)
                .foreground(theme.status_error())
                .modifiers(TextModifiers::BOLD)
                .alignment(Alignment::Center)
                .text(&[TextSpan::from(error_msg)]),
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
