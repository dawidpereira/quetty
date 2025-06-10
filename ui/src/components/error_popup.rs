use crate::components::common::{Msg, PopupActivityMsg};
use crate::components::state::ComponentState;
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
    is_mounted: bool,
}

impl ErrorPopup {
    pub fn new(error: &AppError) -> Self {
        let error_message = format!("{}", error);
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
                .text([TextSpan::from(&error_message)]),
            is_mounted: false,
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

impl ComponentState for ErrorPopup {
    fn mount(&mut self) -> crate::error::AppResult<()> {
        log::debug!("Mounting ErrorPopup component");

        if self.is_mounted {
            log::warn!("ErrorPopup is already mounted");
            return Ok(());
        }

        self.is_mounted = true;

        log::debug!("ErrorPopup component mounted successfully");
        Ok(())
    }
}

impl Drop for ErrorPopup {
    fn drop(&mut self) {
        log::debug!("Dropping ErrorPopup component");
        self.is_mounted = false;
        log::debug!("ErrorPopup component dropped");
    }
}
