use crate::components::common::Msg;
use crate::config;
use tui_realm_stdlib::Phantom;
use tuirealm::event::{Key, KeyEvent, KeyModifiers};
use tuirealm::{Component, Event, MockComponent, NoUserEvent};

#[derive(MockComponent, Default)]
pub struct GlobalKeyWatcher {
    component: Phantom,
}

impl Component<Msg, NoUserEvent> for GlobalKeyWatcher {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) => {
                let keys = config::CONFIG.keys();
                if c == keys.quit() {
                    Some(Msg::AppClose)
                } else if c == keys.help() {
                    Some(Msg::ToggleHelpScreen)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
