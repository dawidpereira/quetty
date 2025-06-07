use crate::components::common::Msg;
use crate::config;
use tui_realm_stdlib::Phantom;
use tuirealm::event::{Key, KeyEvent, KeyModifiers};
use tuirealm::{Component, Event, MockComponent, NoUserEvent};

#[derive(MockComponent)]
pub struct GlobalKeyWatcher {
    component: Phantom,
    is_editing: bool,
}

impl Default for GlobalKeyWatcher {
    fn default() -> Self {
        Self::new(false)
    }
}

impl GlobalKeyWatcher {
    pub fn new(is_editing: bool) -> Self {
        Self {
            component: Phantom::default(),
            is_editing,
        }
    }
}

impl Component<Msg, NoUserEvent> for GlobalKeyWatcher {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) => {
                // When editing, ignore all global keys with no modifiers
                if self.is_editing {
                    return None;
                }

                let keys = config::CONFIG.keys();
                if c == keys.quit() {
                    Some(Msg::AppClose)
                } else if c == keys.help() {
                    Some(Msg::ToggleHelpScreen)
                } else if c == keys.theme() {
                    Some(Msg::ToggleThemePicker)
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(_),
                modifiers: KeyModifiers::SHIFT,
            }) => {
                // When editing, ignore all keys with shift modifiers
                if self.is_editing {
                    return None;
                }
                // Currently no global keys use shift modifier, so return None
                None
            }
            _ => None,
        }
    }
}
