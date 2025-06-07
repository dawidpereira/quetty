use tui_realm_stdlib::Label;
use tuirealm::{
    Component, Event, MockComponent, NoUserEvent,
    props::{Alignment, Color, TextModifiers},
};

use crate::components::common::Msg;
use crate::theme::ThemeManager;

#[derive(MockComponent)]
pub struct TextLabel {
    component: Label,
}

impl TextLabel {
    pub fn new(text: String) -> Self {
        let theme = ThemeManager::global();
        let component = Label::default()
            .text(text)
            .alignment(Alignment::Center)
            .foreground(theme.help_section_title())
            .background(Color::Reset)
            .modifiers(TextModifiers::BOLD);

        Self { component }
    }
}

impl Component<Msg, NoUserEvent> for TextLabel {
    fn on(&mut self, _: Event<NoUserEvent>) -> Option<Msg> {
        None
    }
}
