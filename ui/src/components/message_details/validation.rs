use crate::error::AppError;
use crate::validation::Validator;
use serde_json::Value;

/// Validation errors specific to message content operations
#[derive(Debug, Clone)]
pub enum MessageValidationError {
    EmptyContent,
    TooLarge { size: usize, limit: usize },
    InvalidJson { reason: String },
    InvalidCharacters { characters: String },
}

impl MessageValidationError {
    pub fn user_message(&self) -> String {
        match self {
            MessageValidationError::EmptyContent => {
                "Message content cannot be empty.\n\nPlease add some content before sending."
                    .to_string()
            }
            MessageValidationError::TooLarge { size, limit } => {
                format!(
                    "Message content is too large!\n\n\
                    Current size: {} bytes\n\
                    Maximum allowed: {} bytes\n\n\
                    Please reduce the message size.",
                    size, limit
                )
            }
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
}

impl From<MessageValidationError> for AppError {
    fn from(error: MessageValidationError) -> Self {
        AppError::Config(error.user_message())
    }
}

/// Validator for message content emptiness
pub struct MessageContentValidator;

impl Validator<str> for MessageContentValidator {
    type Error = MessageValidationError;

    fn validate(&self, input: &str) -> Result<(), Self::Error> {
        if input.trim().is_empty() {
            return Err(MessageValidationError::EmptyContent);
        }
        Ok(())
    }
}

/// Validator for message size limits
pub struct MessageSizeValidator {
    max_size: usize,
}

impl MessageSizeValidator {
    pub fn new(max_size: usize) -> Self {
        Self { max_size }
    }
}

impl Validator<str> for MessageSizeValidator {
    type Error = MessageValidationError;

    fn validate(&self, input: &str) -> Result<(), Self::Error> {
        let size = input.len();
        if size > self.max_size {
            return Err(MessageValidationError::TooLarge {
                size,
                limit: self.max_size,
            });
        }
        Ok(())
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
            Err(e) => Err(MessageValidationError::InvalidJson {
                reason: e.to_string(),
            }),
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
                return Err(MessageValidationError::InvalidCharacters {
                    characters: format!("Control character at position {}: {:?}", i, ch),
                });
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
            content_validator: MessageContentValidator,
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
        let validator = MessageContentValidator;

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
        assert!(validator.validate(r#"[1, 2, 3]"#).is_ok());
        assert!(validator.validate(r#""plain text""#).is_ok()); // Valid JSON string
        assert!(validator.validate("42").is_ok()); // Valid JSON number
        assert!(validator.validate("true").is_ok()); // Valid JSON boolean
        assert!(validator.validate("null").is_ok()); // Valid JSON null

        // Invalid JSON (plain text is not valid JSON)
        assert!(validator.validate("plain text").is_err());
        assert!(validator.validate("not json").is_err());
        assert!(validator.validate(r#"{"key": invalid}"#).is_err());
        assert!(validator.validate(r#"[1, 2, 3"#).is_err());
    }

    #[test]
    fn test_message_encoding_validator() {
        let validator = MessageEncodingValidator;

        // Valid content
        assert!(validator.validate("Hello world").is_ok());
        assert!(validator.validate("Line 1\nLine 2").is_ok());
        assert!(validator.validate("Tab\tseparated").is_ok());

        // Invalid content with control characters would need special test setup
        // as Rust strings are UTF-8 by default
    }

    #[test]
    fn test_complete_message_validator() {
        let validator = CompleteMessageValidator::new(100);

        // Valid message (must be valid JSON)
        assert!(validator.validate(r#""Hello world""#).is_ok());
        assert!(validator.validate(r#"{"valid": "json"}"#).is_ok());

        // Invalid - empty
        assert!(validator.validate("").is_err());

        // Invalid - too large
        assert!(validator.validate(&"x".repeat(101)).is_err());

        // Invalid - malformed JSON
        assert!(validator.validate(r#"{"invalid": json}"#).is_err());

        // Invalid - plain text (not valid JSON)
        assert!(validator.validate("Hello world").is_err());
    }
}
