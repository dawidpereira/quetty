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

                let keys = config::get_config_or_panic().keys();
                if c == keys.quit() {
                    Some(Msg::AppClose)
                } else if c == keys.help() {
                    Some(Msg::ToggleHelpScreen)
                } else if c == keys.theme() {
                    Some(Msg::ToggleThemePicker)
                } else if c == keys.config() {
                    Some(Msg::ToggleConfigScreen)
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::SHIFT,
            }) => {
                // When editing, ignore all keys with shift modifiers
                if self.is_editing {
                    return None;
                }

                let keys = config::get_config_or_panic().keys();
                // Handle Shift+C for config (uppercase C)
                if c.eq_ignore_ascii_case(&keys.config()) {
                    Some(Msg::ToggleConfigScreen)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
