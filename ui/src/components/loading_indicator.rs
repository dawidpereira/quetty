use tui_realm_stdlib::Label;
use tuirealm::{
    Component, Event, MockComponent,
    event::NoUserEvent,
    props::{Alignment, Color, TextModifiers},
};

use crate::components::common::Msg;

#[derive(MockComponent)]
pub struct LoadingIndicator {
    component: Label,
    message: String,
}

impl LoadingIndicator {
    pub fn new(message: &str, _indeterminate: bool) -> Self {
        let loading_text = format!("⏳ {} ⏳", message);
        let component = Label::default()
            .text(loading_text)
            .alignment(Alignment::Center)
            .foreground(Color::LightBlue)
            .background(Color::Reset)
            .modifiers(TextModifiers::BOLD);

        Self {
            component,
            message: message.to_string(),
        }
    }

    pub fn set_message(&mut self, message: &str) {
        self.message = message.to_string();
        let loading_text = format!("⏳ {} ⏳", message);
        self.component = Label::default()
            .text(loading_text)
            .alignment(Alignment::Center)
            .foreground(Color::LightBlue)
            .background(Color::Reset)
            .modifiers(TextModifiers::BOLD);
    }
}

impl Component<Msg, NoUserEvent> for LoadingIndicator {
    fn on(&mut self, _: Event<NoUserEvent>) -> Option<Msg> {
        // Loading indicator doesn't respond to events
        None
    }
}

