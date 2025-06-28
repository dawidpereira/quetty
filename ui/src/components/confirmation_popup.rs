use crate::components::base_popup::PopupBuilder;
use crate::components::common::{Msg, PopupActivityMsg};
use crate::components::state::ComponentState;
use crate::config;
use tuirealm::{
    Component, Event, MockComponent, NoUserEvent,
    command::{Cmd, CmdResult},
    event::{Key, KeyEvent},
    ratatui::{Frame, layout::Rect},
};

/// Confirmation popup component that displays yes/no prompts to the user.
///
/// This component provides a consistent confirmation interface using the
/// PopupBuilder pattern for standardized styling while preserving dynamic
/// key binding functionality from the configuration.
///
/// # Usage
///
/// ```rust
/// use quetty::components::confirmation_popup::ConfirmationPopup;
///
/// let popup = ConfirmationPopup::new("Save Changes", "Do you want to save your changes?");
/// ```
///
/// # Events
///
/// - Configured yes key (default 'Y') - Confirms the action
/// - Configured no key (default 'N') - Cancels the action  
/// - `KeyEvent::Esc` - Cancels the action
///
/// # Messages
///
/// Emits `Msg::PopupActivity(PopupActivityMsg::ConfirmationResult(bool))` with the result.
pub struct ConfirmationPopup {
    title: String,
    message: String,
    is_mounted: bool,
}

impl ConfirmationPopup {
    /// Creates a new confirmation popup with the specified title and message.
    ///
    /// # Arguments
    ///
    /// * `title` - The popup title displayed in the border
    /// * `message` - The confirmation message to display
    ///
    /// # Returns
    ///
    /// A new `ConfirmationPopup` instance ready for mounting.
    pub fn new(title: &str, message: &str) -> Self {
        Self {
            title: title.to_string(),
            message: message.to_string(),
            is_mounted: false,
        }
    }
}

impl MockComponent for ConfirmationPopup {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let keys = config::get_config_or_panic().keys();
        PopupBuilder::new(&self.title)
            .add_multiline_text(&self.message)
            .with_confirmation_instructions(
                &keys.confirm_yes().to_string(),
                &keys.confirm_no().to_string(),
            )
            .render(frame, area);
    }

    fn query(&self, _attr: tuirealm::Attribute) -> Option<tuirealm::AttrValue> {
        None
    }

    fn attr(&mut self, _attr: tuirealm::Attribute, _value: tuirealm::AttrValue) {
        // No attributes supported
    }

    fn state(&self) -> tuirealm::State {
        tuirealm::State::None
    }

    fn perform(&mut self, _cmd: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for ConfirmationPopup {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Char(c), ..
            }) => {
                let keys = config::get_config_or_panic().keys();
                let c_lower = c.to_lowercase().next().unwrap_or(c);
                let yes_key = keys
                    .confirm_yes()
                    .to_lowercase()
                    .next()
                    .unwrap_or(keys.confirm_yes());
                let no_key = keys
                    .confirm_no()
                    .to_lowercase()
                    .next()
                    .unwrap_or(keys.confirm_no());

                if c_lower == yes_key {
                    Some(Msg::PopupActivity(PopupActivityMsg::ConfirmationResult(
                        true,
                    )))
                } else if c_lower == no_key {
                    Some(Msg::PopupActivity(PopupActivityMsg::ConfirmationResult(
                        false,
                    )))
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => Some(Msg::PopupActivity(
                PopupActivityMsg::ConfirmationResult(false),
            )),
            _ => None,
        }
    }
}

impl ComponentState for ConfirmationPopup {
    fn mount(&mut self) -> crate::error::AppResult<()> {
        log::debug!("Mounting ConfirmationPopup component");

        if self.is_mounted {
            log::warn!("ConfirmationPopup is already mounted");
            return Ok(());
        }

        self.is_mounted = true;
        log::debug!("ConfirmationPopup component mounted successfully");
        Ok(())
    }
}

impl Drop for ConfirmationPopup {
    fn drop(&mut self) {
        log::debug!("Dropping ConfirmationPopup component");
        self.is_mounted = false;
        log::debug!("ConfirmationPopup component dropped");
    }
}
