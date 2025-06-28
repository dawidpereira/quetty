use crate::components::base_popup::PopupBuilder;
use crate::components::common::{Msg, PopupActivityMsg};
use crate::components::state::ComponentState;
use crate::error::AppError;
use tuirealm::{
    Component, Event, MockComponent, NoUserEvent,
    command::{Cmd, CmdResult},
    event::{Key, KeyEvent},
    ratatui::{Frame, layout::Rect},
};

/// Error popup component that displays error messages to the user.
///
/// This component provides a consistent error display interface using the
/// PopupBuilder pattern for standardized styling and behavior.
///
/// # Usage
///
/// ```rust
/// use quetty::error::AppError;
/// use quetty::components::error_popup::ErrorPopup;
///
/// let error = AppError::Config("Invalid configuration".to_string());
/// let popup = ErrorPopup::new(&error);
/// ```
///
/// # Events
///
/// - `KeyEvent::Enter` - Closes the error popup
/// - `KeyEvent::Esc` - Closes the error popup
///
/// # Messages
///
/// Emits `Msg::PopupActivity(PopupActivityMsg::CloseError)` when closed.
pub struct ErrorPopup {
    message: String,
    is_mounted: bool,
}

impl ErrorPopup {
    /// Creates a new error popup with the specified error.
    ///
    /// # Arguments
    ///
    /// * `error` - The error to display, already formatted by ErrorReporter
    ///
    /// # Returns
    ///
    /// A new `ErrorPopup` instance ready for mounting.
    pub fn new(error: &AppError) -> Self {
        Self {
            message: error.to_string(),
            is_mounted: false,
        }
    }
}

impl MockComponent for ErrorPopup {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        PopupBuilder::error("❌ Error")
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
