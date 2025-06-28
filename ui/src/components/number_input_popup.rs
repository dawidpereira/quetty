use crate::components::common::{Msg, PopupActivityMsg};
use crate::components::state::ComponentState;
use crate::components::validation_patterns::{NumericRangeValidator, ValidationState};
use crate::theme::ThemeManager;
use crate::validation::Validator;
use tuirealm::{
    Component, Event, MockComponent, NoUserEvent, State, StateValue,
    command::{Cmd, CmdResult},
    event::{Key, KeyEvent, KeyModifiers},
    ratatui::{
        Frame,
        layout::{Alignment, Rect},
        style::{Color, Modifier, Style},
        text::{Line, Span, Text},
        widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    },
};

/// A popup component for numeric input with validation.
///
/// This component provides a user-friendly interface for entering numeric values
/// within a specified range, with real-time validation feedback and default value support.
///
/// # Usage
///
/// ```rust
/// use quetty::components::number_input_popup::NumberInputPopup;
///
/// let popup = NumberInputPopup::new(
///     "Enter Count".to_string(),
///     "How many messages to process?".to_string(),
///     1,
///     100
/// );
/// ```
///
/// # Events
///
/// - `KeyEvent::Enter` - Submits the input (uses default if empty)
/// - `KeyEvent::Esc` - Cancels the input
/// - Character keys - Updates the input value
/// - `KeyEvent::Backspace` - Removes last character
///
/// # Messages
///
/// Emits `Msg::PopupActivity(PopupActivityMsg::NumberInputResult(value))` on successful input.
pub struct NumberInputPopup {
    title: String,
    message: String,
    min_value: usize,
    max_value: usize,
    current_input: String,
    validator: NumericRangeValidator,
    is_mounted: bool,
}

impl NumberInputPopup {
    /// Creates a new number input popup with the specified parameters.
    ///
    /// # Arguments
    ///
    /// * `title` - The popup title displayed in the border
    /// * `message` - Descriptive text explaining what to enter
    /// * `min_value` - Minimum allowed value (inclusive)
    /// * `max_value` - Maximum allowed value (inclusive)
    ///
    /// # Returns
    ///
    /// A new `NumberInputPopup` instance ready for mounting.
    pub fn new(title: String, message: String, min_value: usize, max_value: usize) -> Self {
        Self {
            title,
            message,
            min_value,
            max_value,
            current_input: String::new(),
            validator: NumericRangeValidator::new("number")
                .with_range(min_value as i64, max_value as i64),
            is_mounted: false,
        }
    }

    /// Validates the current input and returns the parsed number if valid.
    ///
    /// Returns the default value (clamped to range) if input is empty.
    ///
    /// # Returns
    ///
    /// `Some(number)` if valid, `None` if invalid.
    fn validate_and_get_number(&self) -> Option<usize> {
        if self.current_input.is_empty() {
            // Return default value if empty (10 clamped to valid range)
            return Some(10.max(self.min_value).min(self.max_value));
        }

        match self.validator.validate(&self.current_input) {
            Ok(_) => self.current_input.parse().ok(),
            Err(_) => None,
        }
    }

    /// Gets the current validation state for UI feedback.
    ///
    /// # Returns
    ///
    /// `ValidationState` containing validity and error message.
    fn get_validation_state(&self) -> ValidationState {
        if self.current_input.is_empty() {
            return ValidationState::valid(); // Empty is valid (uses default)
        }

        ValidationState::from_result(self.validator.validate(&self.current_input))
    }

    /// Calculates the default value within the specified range.
    fn get_default_value(&self) -> usize {
        10.max(self.min_value).min(self.max_value)
    }

    /// Renders the input field with appropriate styling based on validation state.
    fn render_input_field(&self) -> Line {
        let input_text = if self.current_input.is_empty() {
            "Type a number..."
        } else {
            &self.current_input
        };

        let validation_state = self.get_validation_state();

        let input_style = if self.current_input.is_empty() {
            Style::default().fg(Color::Gray)
        } else if validation_state.is_valid {
            Style::default().fg(ThemeManager::status_success())
        } else {
            Style::default().fg(ThemeManager::status_error())
        };

        Line::from(vec![
            Span::styled("Input: ", Style::default().fg(ThemeManager::text_primary())),
            Span::styled(input_text, input_style),
        ])
    }

    /// Renders validation feedback if there's an error.
    fn render_validation_feedback(&self) -> Option<Line> {
        let validation_state = self.get_validation_state();

        if !validation_state.is_valid {
            if let Some(error_message) = validation_state.error_message {
                return Some(Line::from(Span::styled(
                    format!("⚠ {}", error_message),
                    Style::default().fg(ThemeManager::status_error()),
                )));
            }
        }
        None
    }
}

impl MockComponent for NumberInputPopup {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Create the border block with dynamic title
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(ThemeManager::primary_accent()))
            .title(format!(" {} ", self.title))
            .title_alignment(Alignment::Center);

        // Create content lines
        let mut lines = Vec::new();

        // Add empty line at the top for better spacing
        lines.push(Line::from(""));

        // Add message lines
        for line in self.message.lines() {
            lines.push(Line::from(line));
        }

        lines.push(Line::from(""));

        // Add range info
        lines.push(Line::from(format!(
            "Range: {} to {} (Enter for default: {})",
            self.min_value,
            self.max_value,
            self.get_default_value()
        )));

        lines.push(Line::from(""));

        // Add input field
        let input_field = self.render_input_field();
        lines.push(input_field);

        // Add validation error if present
        if let Some(error_msg) = self.render_validation_feedback() {
            lines.push(Line::from(""));
            lines.push(error_msg);
        }

        lines.push(Line::from(""));

        // Add instructions
        lines.push(Line::from(vec![
            Span::styled(
                "[Enter]",
                Style::default()
                    .fg(ThemeManager::status_success())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Accept    "),
            Span::styled(
                "[Esc]",
                Style::default()
                    .fg(ThemeManager::status_error())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Cancel"),
        ]));

        let text = Text::from(lines);

        // Create the paragraph
        let paragraph = Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(
                Style::default()
                    .fg(ThemeManager::popup_text())
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(paragraph, area);
    }

    fn query(&self, _attr: tuirealm::Attribute) -> Option<tuirealm::AttrValue> {
        None
    }

    fn attr(&mut self, _attr: tuirealm::Attribute, _value: tuirealm::AttrValue) {
        // No attributes supported
    }

    fn state(&self) -> State {
        State::One(StateValue::String(self.current_input.clone()))
    }

    fn perform(&mut self, _cmd: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for NumberInputPopup {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Esc,
                modifiers: KeyModifiers::NONE,
            }) => {
                // Cancel input
                Some(Msg::PopupActivity(PopupActivityMsg::NumberInputResult(0))) // 0 indicates cancel
            }

            Event::Keyboard(KeyEvent {
                code: Key::Enter,
                modifiers: KeyModifiers::NONE,
            }) => {
                // Accept input
                self.validate_and_get_number()
                    .map(|number| Msg::PopupActivity(PopupActivityMsg::NumberInputResult(number)))
            }

            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) => {
                // Add character if it's a digit and we haven't exceeded reasonable length
                if c.is_ascii_digit() && self.current_input.len() < 4 {
                    self.current_input.push(c);
                    Some(Msg::ForceRedraw)
                } else {
                    None
                }
            }

            Event::Keyboard(KeyEvent {
                code: Key::Backspace,
                modifiers: KeyModifiers::NONE,
            }) => {
                // Remove last character
                self.current_input.pop();
                Some(Msg::ForceRedraw)
            }

            _ => None,
        }
    }
}

impl ComponentState for NumberInputPopup {
    fn mount(&mut self) -> crate::error::AppResult<()> {
        log::debug!("Mounting NumberInputPopup component");

        if self.is_mounted {
            log::warn!("NumberInputPopup is already mounted");
            return Ok(());
        }

        // Initialize component state
        self.current_input.clear();

        self.is_mounted = true;
        log::debug!("NumberInputPopup component mounted successfully");
        Ok(())
    }
}

impl Drop for NumberInputPopup {
    fn drop(&mut self) {
        log::debug!("Dropping NumberInputPopup component");
        // Clean up component state
        self.current_input.clear();
        self.is_mounted = false;
        log::debug!("NumberInputPopup component dropped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_number_input_popup_creation() {
        let popup =
            NumberInputPopup::new("Test Title".to_string(), "Test message".to_string(), 1, 100);

        assert_eq!(popup.title, "Test Title");
        assert_eq!(popup.message, "Test message");
        assert_eq!(popup.min_value, 1);
        assert_eq!(popup.max_value, 100);
        assert!(popup.current_input.is_empty());
        assert!(!popup.is_mounted);
    }

    #[test]
    fn test_default_value_calculation() {
        let popup = NumberInputPopup::new("Test".to_string(), "Test".to_string(), 5, 15);
        assert_eq!(popup.get_default_value(), 10); // 10 is within range

        let popup_low_range = NumberInputPopup::new("Test".to_string(), "Test".to_string(), 20, 30);
        assert_eq!(popup_low_range.get_default_value(), 20); // Clamped to min

        let popup_high_range = NumberInputPopup::new("Test".to_string(), "Test".to_string(), 1, 5);
        assert_eq!(popup_high_range.get_default_value(), 5); // Clamped to max
    }

    #[test]
    fn test_validation_with_empty_input() {
        let popup = NumberInputPopup::new("Test".to_string(), "Test".to_string(), 1, 100);

        // Empty input should be valid and return default value
        let validation_state = popup.get_validation_state();
        assert!(validation_state.is_valid);
        assert!(validation_state.error_message.is_none());

        assert_eq!(popup.validate_and_get_number(), Some(10));
    }

    #[test]
    fn test_validation_with_valid_input() {
        let mut popup = NumberInputPopup::new("Test".to_string(), "Test".to_string(), 1, 100);
        popup.current_input = "50".to_string();

        let validation_state = popup.get_validation_state();
        assert!(validation_state.is_valid);
        assert!(validation_state.error_message.is_none());

        assert_eq!(popup.validate_and_get_number(), Some(50));
    }

    #[test]
    fn test_validation_with_invalid_input() {
        let mut popup = NumberInputPopup::new("Test".to_string(), "Test".to_string(), 1, 100);
        popup.current_input = "abc".to_string();

        let validation_state = popup.get_validation_state();
        assert!(!validation_state.is_valid);
        assert!(validation_state.error_message.is_some());

        assert_eq!(popup.validate_and_get_number(), None);
    }

    #[test]
    fn test_validation_with_out_of_range_input() {
        let mut popup = NumberInputPopup::new("Test".to_string(), "Test".to_string(), 10, 20);
        popup.current_input = "5".to_string();

        let validation_state = popup.get_validation_state();
        assert!(!validation_state.is_valid);
        assert!(validation_state.error_message.is_some());

        assert_eq!(popup.validate_and_get_number(), None);
    }

    #[test]
    fn test_render_input_field() {
        let popup = NumberInputPopup::new("Test".to_string(), "Test".to_string(), 1, 100);
        let input_line = popup.render_input_field();

        // Should contain placeholder text for empty input
        assert!(format!("{:?}", input_line).contains("Type a number"));
    }

    #[test]
    fn test_render_validation_feedback() {
        let mut popup = NumberInputPopup::new("Test".to_string(), "Test".to_string(), 1, 100);

        // Valid input should have no feedback
        assert!(popup.render_validation_feedback().is_none());

        // Invalid input should have feedback
        popup.current_input = "invalid".to_string();
        assert!(popup.render_validation_feedback().is_some());
    }

    #[test]
    fn test_number_input_popup_default_value_logic() {
        let popup = NumberInputPopup::new("Test".to_string(), "Enter number".to_string(), 5, 15);

        // Test with empty input (should return default value of 10, clamped to range)
        assert_eq!(popup.validate_and_get_number(), Some(10));

        // Test with range where 10 is below minimum
        let popup2 = NumberInputPopup::new("Test".to_string(), "Enter number".to_string(), 20, 30);
        // Should return minimum value (20) when default (10) is below range
        assert_eq!(popup2.validate_and_get_number(), Some(20));

        // Test with range where 10 is above maximum
        let popup3 = NumberInputPopup::new("Test".to_string(), "Enter number".to_string(), 1, 5);
        // Should return maximum value (5) when default (10) is above range
        assert_eq!(popup3.validate_and_get_number(), Some(5));
    }
}
