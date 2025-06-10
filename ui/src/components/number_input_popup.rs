use crate::components::common::{Msg, PopupActivityMsg};
use crate::error::AppError;
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

/// Validation errors for number inputs
#[derive(Debug, Clone)]
pub enum NumberValidationError {
    Empty,
    InvalidFormat {
        input: String,
    },
    OutOfRange {
        value: usize,
        min: usize,
        max: usize,
    },
}

impl NumberValidationError {
    pub fn user_message(&self) -> String {
        match self {
            NumberValidationError::Empty => {
                "Please enter a number or press Enter for default value.".to_string()
            }
            NumberValidationError::InvalidFormat { input } => {
                format!(
                    "'{}' is not a valid number. Please enter only digits.",
                    input
                )
            }
            NumberValidationError::OutOfRange { value, min, max } => {
                format!(
                    "Number {} is out of range. Please enter a value between {} and {}.",
                    value, min, max
                )
            }
        }
    }
}

impl From<NumberValidationError> for AppError {
    fn from(error: NumberValidationError) -> Self {
        AppError::Config(error.user_message())
    }
}

/// Validator for numeric range validation
pub struct NumericRangeValidator {
    min_value: usize,
    max_value: usize,
}

impl NumericRangeValidator {
    pub fn new(min_value: usize, max_value: usize) -> Self {
        Self {
            min_value,
            max_value,
        }
    }
}

impl Validator<str> for NumericRangeValidator {
    type Error = NumberValidationError;

    fn validate(&self, input: &str) -> Result<(), Self::Error> {
        if input.trim().is_empty() {
            return Err(NumberValidationError::Empty);
        }

        match input.parse::<usize>() {
            Ok(value) => {
                if value >= self.min_value && value <= self.max_value {
                    Ok(())
                } else {
                    Err(NumberValidationError::OutOfRange {
                        value,
                        min: self.min_value,
                        max: self.max_value,
                    })
                }
            }
            Err(_) => Err(NumberValidationError::InvalidFormat {
                input: input.to_string(),
            }),
        }
    }
}

pub struct NumberInputPopup {
    title: String,
    message: String,
    min_value: usize,
    max_value: usize,
    current_input: String,
    validator: NumericRangeValidator,
}

impl NumberInputPopup {
    pub fn new(title: String, message: String, min_value: usize, max_value: usize) -> Self {
        Self {
            title,
            message,
            min_value,
            max_value,
            current_input: String::new(),
            validator: NumericRangeValidator::new(min_value, max_value),
        }
    }

    fn validate_and_get_number(&self) -> Option<usize> {
        if self.current_input.is_empty() {
            // Return default value if empty
            return Some(10.max(self.min_value).min(self.max_value));
        }

        // Use the validator to check the input
        match self.validator.validate(&self.current_input) {
            Ok(_) => self.current_input.parse().ok(),
            Err(_) => None,
        }
    }

    fn get_validation_status(&self) -> (bool, Option<String>) {
        if self.current_input.is_empty() {
            return (true, None); // Empty is valid (uses default)
        }

        match self.validator.validate(&self.current_input) {
            Ok(_) => (true, None),
            Err(error) => (false, Some(error.user_message())),
        }
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
            10.max(self.min_value).min(self.max_value)
        )));

        lines.push(Line::from(""));

        // Add input field
        let input_text = if self.current_input.is_empty() {
            "Type a number..."
        } else {
            &self.current_input
        };

        let (is_valid, validation_message) = self.get_validation_status();

        let input_style = if self.current_input.is_empty() {
            Style::default().fg(Color::Gray)
        } else if is_valid {
            Style::default().fg(ThemeManager::status_success())
        } else {
            Style::default().fg(ThemeManager::status_error())
        };

        lines.push(Line::from(vec![
            Span::raw("Input: ["),
            Span::styled(input_text, input_style.add_modifier(Modifier::BOLD)),
            Span::raw("]"),
        ]));

        // Add validation error if present
        if let Some(error_msg) = validation_message {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("⚠️ ", Style::default().fg(ThemeManager::status_error())),
                Span::styled(error_msg, Style::default().fg(ThemeManager::status_error())),
            ]));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numeric_range_validator_valid_inputs() {
        let validator = NumericRangeValidator::new(5, 20);

        // Valid values within range
        assert!(validator.validate("5").is_ok());
        assert!(validator.validate("10").is_ok());
        assert!(validator.validate("20").is_ok());
        assert!(validator.validate("15").is_ok());
    }

    #[test]
    fn test_numeric_range_validator_empty_input() {
        let validator = NumericRangeValidator::new(1, 100);

        // Empty input should return specific error
        let result = validator.validate("");
        assert!(result.is_err());
        if let Err(NumberValidationError::Empty) = result {
            // Expected error type
        } else {
            panic!("Expected Empty error for empty input");
        }

        // Whitespace-only input should also be empty
        let result = validator.validate("   ");
        assert!(result.is_err());
        if let Err(NumberValidationError::Empty) = result {
            // Expected error type
        } else {
            panic!("Expected Empty error for whitespace input");
        }
    }

    #[test]
    fn test_numeric_range_validator_invalid_format() {
        let validator = NumericRangeValidator::new(1, 100);

        // Non-numeric inputs
        assert!(validator.validate("abc").is_err());
        assert!(validator.validate("12.5").is_err());
        assert!(validator.validate("1a2").is_err());
        assert!(validator.validate("-5").is_err());

        // Verify error type and message
        let result = validator.validate("abc");
        if let Err(NumberValidationError::InvalidFormat { input }) = result {
            assert_eq!(input, "abc");
        } else {
            panic!("Expected InvalidFormat error");
        }
    }

    #[test]
    fn test_numeric_range_validator_out_of_range() {
        let validator = NumericRangeValidator::new(10, 50);

        // Below minimum
        let result = validator.validate("5");
        assert!(result.is_err());
        if let Err(NumberValidationError::OutOfRange { value, min, max }) = result {
            assert_eq!(value, 5);
            assert_eq!(min, 10);
            assert_eq!(max, 50);
        } else {
            panic!("Expected OutOfRange error for value below minimum");
        }

        // Above maximum
        let result = validator.validate("100");
        assert!(result.is_err());
        if let Err(NumberValidationError::OutOfRange { value, min, max }) = result {
            assert_eq!(value, 100);
            assert_eq!(min, 10);
            assert_eq!(max, 50);
        } else {
            panic!("Expected OutOfRange error for value above maximum");
        }
    }

    #[test]
    fn test_numeric_range_validator_edge_cases() {
        let validator = NumericRangeValidator::new(0, 1);

        // Test minimum and maximum edge values
        assert!(validator.validate("0").is_ok());
        assert!(validator.validate("1").is_ok());

        // Test just outside boundaries
        let result = validator.validate("2");
        assert!(result.is_err());
    }

    #[test]
    fn test_number_validation_error_user_messages() {
        // Test Empty error message
        let error = NumberValidationError::Empty;
        let message = error.user_message();
        assert!(message.contains("Enter for default"));

        // Test InvalidFormat error message
        let error = NumberValidationError::InvalidFormat {
            input: "abc".to_string(),
        };
        let message = error.user_message();
        assert!(message.contains("abc"));
        assert!(message.contains("not a valid number"));

        // Test OutOfRange error message
        let error = NumberValidationError::OutOfRange {
            value: 150,
            min: 10,
            max: 100,
        };
        let message = error.user_message();
        assert!(message.contains("150"));
        assert!(message.contains("10"));
        assert!(message.contains("100"));
        assert!(message.contains("out of range"));
    }

    #[test]
    fn test_validator_integration_with_app_error() {
        let validator = NumericRangeValidator::new(1, 10);

        // Test that validation errors can be converted to AppError
        let validation_result = validator.validate("abc");
        assert!(validation_result.is_err());

        if let Err(validation_error) = validation_result {
            let app_error: AppError = validation_error.into();
            // Verify the conversion works and contains user-friendly message
            match app_error {
                AppError::Config(message) => {
                    assert!(message.contains("not a valid number"));
                }
                _ => panic!("Expected Config error type"),
            }
        }
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
