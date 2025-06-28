use crate::components::base_popup::PopupBuilder;
use crate::components::common::{Msg, PopupActivityMsg};
use crate::components::state::ComponentState;
use tuirealm::{
    Component, Event, MockComponent, NoUserEvent,
    command::{Cmd, CmdResult},
    event::{Key, KeyEvent},
    ratatui::{Frame, layout::Rect},
};

/// Success popup component that displays success messages to the user.
///
/// This component provides a consistent success display interface using the
/// PopupBuilder pattern for standardized styling and behavior.
///
/// # Usage
///
/// ```rust
/// use quetty::components::success_popup::SuccessPopup;
///
/// let popup = SuccessPopup::new("Operation completed successfully!");
/// ```
///
/// # Events
///
/// - `KeyEvent::Enter` - Closes the success popup
/// - `KeyEvent::Esc` - Closes the success popup
///
/// # Messages
///
/// Emits `Msg::PopupActivity(PopupActivityMsg::CloseSuccess)` when closed.
pub struct SuccessPopup {
    message: String,
    is_mounted: bool,
}

impl SuccessPopup {
    /// Creates a new success popup with the specified message.
    ///
    /// # Arguments
    ///
    /// * `message` - The success message to display
    ///
    /// # Returns
    ///
    /// A new `SuccessPopup` instance ready for mounting.
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            is_mounted: false,
        }
    }
}

impl MockComponent for SuccessPopup {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        PopupBuilder::success("✅ Success")
            .add_multiline_text(&self.message)
            .with_instructions("Press Enter or Esc to close")
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

impl Component<Msg, NoUserEvent> for SuccessPopup {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Enter | Key::Esc,
                ..
            }) => Some(Msg::PopupActivity(PopupActivityMsg::CloseSuccess)),
            _ => None,
        }
    }
}

impl ComponentState for SuccessPopup {
    fn mount(&mut self) -> crate::error::AppResult<()> {
        log::debug!("Mounting SuccessPopup component");

        if self.is_mounted {
            log::warn!("SuccessPopup is already mounted");
            return Ok(());
        }

        self.is_mounted = true;
        log::debug!("SuccessPopup component mounted successfully");
        Ok(())
    }
}

impl Drop for SuccessPopup {
    fn drop(&mut self) {
        log::debug!("Dropping SuccessPopup component");
        self.is_mounted = false;
        log::debug!("SuccessPopup component dropped");
    }
}
