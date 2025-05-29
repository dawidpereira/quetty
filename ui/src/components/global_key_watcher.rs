use tui_realm_stdlib::Phantom;
use tuirealm::event::{Key, KeyEvent, KeyModifiers};
use tuirealm::{Component, Event, MockComponent, NoUserEvent};

use crate::components::common::Msg;

#[derive(MockComponent, Default)]
pub struct GlobalKeyWatcher {
    component: Phantom,
}

impl Component<Msg, NoUserEvent> for GlobalKeyWatcher {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char('q'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Msg::AppClose),
            Event::Keyboard(KeyEvent {
                code: Key::Char('h'),
                modifiers: KeyModifiers::NONE,
            }) => {
                // Show help screen when h is pressed
                Some(Msg::ToggleHelpScreen)
            }
            _ => None,
        }
    }
}
