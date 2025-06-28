use crate::components::validation_patterns::{CommonValidationError, StringLengthValidator};
use crate::error::AppError;
use crate::validation::Validator;
use serde_json::Value;

/// Specialized validation errors for message operations that extend common patterns
#[derive(Debug, Clone)]
pub enum MessageValidationError {
    /// Common validation errors (delegated to CommonValidationError)
    Common(CommonValidationError),
    /// JSON-specific validation error
    InvalidJson { reason: String },
    /// Text encoding validation error
    InvalidCharacters { characters: String },
}

impl MessageValidationError {
    pub fn user_message(&self) -> String {
        match self {
            MessageValidationError::Common(common_error) => common_error.user_message(),
            MessageValidationError::InvalidJson { reason } => {
                format!(
                    "Invalid JSON format!\n\n\
                    Error: {}\n\n\
                    Please check your JSON syntax and try again.",
                    reason
                )
            }
            MessageValidationError::InvalidCharacters { characters } => {
                format!(
                    "Message contains invalid characters!\n\n\
                    Invalid characters: {}\n\n\
                    Please remove these characters and try again.",
                    characters
                )
            }
        }
    }

    /// Create a JSON validation error
    pub fn invalid_json(reason: impl Into<String>) -> Self {
        Self::InvalidJson {
            reason: reason.into(),
        }
    }

    /// Create an invalid characters error
    pub fn invalid_characters(characters: impl Into<String>) -> Self {
        Self::InvalidCharacters {
            characters: characters.into(),
        }
    }
}

impl From<CommonValidationError> for MessageValidationError {
    fn from(error: CommonValidationError) -> Self {
        MessageValidationError::Common(error)
    }
}

impl From<MessageValidationError> for AppError {
    fn from(error: MessageValidationError) -> Self {
        AppError::Config(error.user_message())
    }
}

/// Validator for message content using common patterns
pub struct MessageContentValidator {
    validator: StringLengthValidator,
}

impl Default for MessageContentValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageContentValidator {
    pub fn new() -> Self {
        Self {
            validator: StringLengthValidator::new("message content").with_min_length(1), // Must have at least 1 character after trimming
        }
    }
}

impl Validator<str> for MessageContentValidator {
    type Error = MessageValidationError;

    fn validate(&self, input: &str) -> Result<(), Self::Error> {
        // Trim input for validation but don't modify the original
        let trimmed = input.trim();
        self.validator.validate(trimmed).map_err(Into::into)
    }
}

/// Validator for message size limits using common patterns
pub struct MessageSizeValidator {
    validator: StringLengthValidator,
}

impl MessageSizeValidator {
    pub fn new(max_size: usize) -> Self {
        Self {
            validator: StringLengthValidator::new("message content").with_max_length(max_size),
        }
    }
}

impl Validator<str> for MessageSizeValidator {
    type Error = MessageValidationError;

    fn validate(&self, input: &str) -> Result<(), Self::Error> {
        self.validator.validate(input).map_err(Into::into)
    }
}

/// Validator for JSON format - ALL messages must be valid JSON when sending/updating
/// Uses `serde_json` - the industry standard JSON library for Rust
/// Perfect for format validation (no need for schema validation packages)
pub struct JsonFormatValidator;

impl Validator<str> for JsonFormatValidator {
    type Error = MessageValidationError;

    fn validate(&self, input: &str) -> Result<(), Self::Error> {
        // All messages must be valid JSON when sending/updating
        match serde_json::from_str::<Value>(input) {
            Ok(_) => Ok(()),
            Err(e) => Err(MessageValidationError::invalid_json(e.to_string())),
        }
    }
}

/// Validator for text encoding (UTF-8)
pub struct MessageEncodingValidator;

impl Validator<str> for MessageEncodingValidator {
    type Error = MessageValidationError;

    fn validate(&self, input: &str) -> Result<(), Self::Error> {
        // Check for invalid UTF-8 sequences or control characters
        for (i, ch) in input.char_indices() {
            if ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t' {
                return Err(MessageValidationError::invalid_characters(format!(
                    "Control character at position {}: {:?}",
                    i, ch
                )));
            }
        }
        Ok(())
    }
}

/// Composite validator for complete message validation
pub struct CompleteMessageValidator {
    content_validator: MessageContentValidator,
    size_validator: MessageSizeValidator,
    json_validator: JsonFormatValidator,
    encoding_validator: MessageEncodingValidator,
}

impl CompleteMessageValidator {
    pub fn new(max_size: usize) -> Self {
        Self {
            content_validator: MessageContentValidator::new(),
            size_validator: MessageSizeValidator::new(max_size),
            json_validator: JsonFormatValidator,
            encoding_validator: MessageEncodingValidator,
        }
    }

    pub fn azure_default() -> Self {
        Self::new(256 * 1024) // 256 KB Azure limit
    }
}

impl Validator<str> for CompleteMessageValidator {
    type Error = MessageValidationError;

    fn validate(&self, input: &str) -> Result<(), Self::Error> {
        // Run all validations in order
        self.content_validator.validate(input)?;
        self.size_validator.validate(input)?;
        self.encoding_validator.validate(input)?;
        self.json_validator.validate(input)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_content_validator() {
        let validator = MessageContentValidator::new();

        // Valid content
        assert!(validator.validate("Hello world").is_ok());
        assert!(validator.validate("   Valid content   ").is_ok());

        // Invalid content
        assert!(validator.validate("").is_err());
        assert!(validator.validate("   ").is_err());
        assert!(validator.validate("\n\t").is_err());
    }

    #[test]
    fn test_message_size_validator() {
        let validator = MessageSizeValidator::new(10);

        // Valid sizes
        assert!(validator.validate("hello").is_ok());
        assert!(validator.validate("1234567890").is_ok());

        // Invalid size
        assert!(validator.validate("12345678901").is_err());
    }

    #[test]
    fn test_json_format_validator() {
        let validator = JsonFormatValidator;

        // Valid JSON
        assert!(validator.validate(r#"{"key": "value"}"#).is_ok());
        assert!(validator.validate("[]").is_ok());
        assert!(validator.validate("null").is_ok());
        assert!(validator.validate("true").is_ok());
        assert!(validator.validate("42").is_ok());

        // Invalid JSON
        assert!(validator.validate("{key: value}").is_err()); // Unquoted keys
        assert!(validator.validate("{'key': 'value'}").is_err()); // Single quotes
        assert!(validator.validate("undefined").is_err()); // JavaScript undefined
    }

    #[test]
    fn test_message_encoding_validator() {
        let validator = MessageEncodingValidator;

        // Valid encoding
        assert!(validator.validate("Hello world").is_ok());
        assert!(validator.validate("Unicode: 🚀 ñ ü").is_ok());
        assert!(validator.validate("Newlines\nand\ttabs\rare\rok").is_ok());

        // Invalid encoding (control characters)
        assert!(validator.validate("Bell\x07character").is_err());
        assert!(validator.validate("Null\x00character").is_err());
    }

    #[test]
    fn test_complete_message_validator() {
        let validator = CompleteMessageValidator::new(100);

        // Valid message
        assert!(validator.validate(r#"{"message": "Hello world"}"#).is_ok());

        // Invalid: empty
        assert!(validator.validate("").is_err());

        // Invalid: too large
        let large_json = format!(r#"{{"data": "{}"}}"#, "x".repeat(200));
        assert!(validator.validate(&large_json).is_err());

        // Invalid: not JSON
        assert!(validator.validate("not json").is_err());
    }
}
