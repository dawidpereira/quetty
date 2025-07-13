use crate::validation::Validator;

/// Common validation errors with user-friendly messages
#[derive(Debug, Clone)]
pub enum CommonValidationError {
    Empty {
        field_name: String,
    },
    InvalidFormat {
        field_name: String,
        expected_format: String,
    },
    OutOfRange {
        field_name: String,
        min: Option<String>,
        max: Option<String>,
    },
    TooLong {
        field_name: String,
        max_length: usize,
        actual_length: usize,
    },
    TooShort {
        field_name: String,
        min_length: usize,
        actual_length: usize,
    },
}

impl CommonValidationError {
    pub fn empty(field_name: impl Into<String>) -> Self {
        Self::Empty {
            field_name: field_name.into(),
        }
    }

    pub fn invalid_format(
        field_name: impl Into<String>,
        expected_format: impl Into<String>,
    ) -> Self {
        Self::InvalidFormat {
            field_name: field_name.into(),
            expected_format: expected_format.into(),
        }
    }

    pub fn out_of_range(
        field_name: impl Into<String>,
        min: Option<impl Into<String>>,
        max: Option<impl Into<String>>,
    ) -> Self {
        Self::OutOfRange {
            field_name: field_name.into(),
            min: min.map(|m| m.into()),
            max: max.map(|m| m.into()),
        }
    }

    pub fn too_long(
        field_name: impl Into<String>,
        max_length: usize,
        actual_length: usize,
    ) -> Self {
        Self::TooLong {
            field_name: field_name.into(),
            max_length,
            actual_length,
        }
    }

    pub fn too_short(
        field_name: impl Into<String>,
        min_length: usize,
        actual_length: usize,
    ) -> Self {
        Self::TooShort {
            field_name: field_name.into(),
            min_length,
            actual_length,
        }
    }

    /// Get user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            Self::Empty { field_name } => format!("{field_name} cannot be empty"),
            Self::InvalidFormat {
                field_name,
                expected_format,
            } => {
                format!("{field_name} must be in {expected_format} format")
            }
            Self::OutOfRange {
                field_name,
                min,
                max,
            } => match (min, max) {
                (Some(min), Some(max)) => {
                    format!("{field_name} must be between {min} and {max}")
                }
                (Some(min), None) => format!("{field_name} must be at least {min}"),
                (None, Some(max)) => format!("{field_name} must be at most {max}"),
                (None, None) => format!("{field_name} is out of range"),
            },
            Self::TooLong {
                field_name,
                max_length,
                actual_length,
            } => {
                format!(
                    "{field_name} is too long ({actual_length} characters, maximum {max_length})"
                )
            }
            Self::TooShort {
                field_name,
                min_length,
                actual_length,
            } => {
                format!(
                    "{field_name} is too short ({actual_length} characters, minimum {min_length})"
                )
            }
        }
    }
}

impl std::fmt::Display for CommonValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.user_message())
    }
}

impl std::error::Error for CommonValidationError {}

/// Numeric range validator for input fields
pub struct NumericRangeValidator {
    min: Option<i64>,
    max: Option<i64>,
    field_name: String,
}

impl NumericRangeValidator {
    pub fn new(field_name: impl Into<String>) -> Self {
        Self {
            min: None,
            max: None,
            field_name: field_name.into(),
        }
    }

    pub fn with_range(mut self, min: i64, max: i64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }
}

impl Validator<str> for NumericRangeValidator {
    type Error = CommonValidationError;

    fn validate(&self, input: &str) -> Result<(), Self::Error> {
        if input.trim().is_empty() {
            return Err(CommonValidationError::empty(&self.field_name));
        }

        let number: i64 = input
            .trim()
            .parse()
            .map_err(|_| CommonValidationError::invalid_format(&self.field_name, "valid number"))?;

        if let Some(min) = self.min {
            if number < min {
                return Err(CommonValidationError::out_of_range(
                    &self.field_name,
                    Some(min.to_string()),
                    self.max.map(|m| m.to_string()),
                ));
            }
        }

        if let Some(max) = self.max {
            if number > max {
                return Err(CommonValidationError::out_of_range(
                    &self.field_name,
                    self.min.map(|m| m.to_string()),
                    Some(max.to_string()),
                ));
            }
        }

        Ok(())
    }
}

/// String length validator for text input fields
pub struct StringLengthValidator {
    min_length: Option<usize>,
    max_length: Option<usize>,
    field_name: String,
}

impl StringLengthValidator {
    pub fn new(field_name: impl Into<String>) -> Self {
        Self {
            min_length: None,
            max_length: None,
            field_name: field_name.into(),
        }
    }

    pub fn with_min_length(mut self, min_length: usize) -> Self {
        self.min_length = Some(min_length);
        self
    }

    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }
}

impl Validator<str> for StringLengthValidator {
    type Error = CommonValidationError;

    fn validate(&self, input: &str) -> Result<(), Self::Error> {
        let length = input.len();

        if let Some(min_length) = self.min_length {
            if length < min_length {
                return Err(CommonValidationError::too_short(
                    &self.field_name,
                    min_length,
                    length,
                ));
            }
        }

        if let Some(max_length) = self.max_length {
            if length > max_length {
                return Err(CommonValidationError::too_long(
                    &self.field_name,
                    max_length,
                    length,
                ));
            }
        }

        Ok(())
    }
}

/// Validation state for UI feedback
#[derive(Debug, Clone)]
pub struct ValidationState {
    pub is_valid: bool,
    pub error_message: Option<String>,
}

impl ValidationState {
    pub fn valid() -> Self {
        Self {
            is_valid: true,
            error_message: None,
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self {
            is_valid: false,
            error_message: Some(message.into()),
        }
    }

    pub fn from_result<E: std::fmt::Display>(result: Result<(), E>) -> Self {
        match result {
            Ok(()) => Self::valid(),
            Err(e) => Self::invalid(e.to_string()),
        }
    }
}

/// Helper trait for components that need validation
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numeric_range_validator() {
        let validator = NumericRangeValidator::new("test field").with_range(1, 10);

        assert!(validator.validate("5").is_ok());
        assert!(validator.validate("1").is_ok());
        assert!(validator.validate("10").is_ok());

        assert!(validator.validate("0").is_err());
        assert!(validator.validate("11").is_err());
        assert!(validator.validate("abc").is_err());
        assert!(validator.validate("").is_err());
    }

    #[test]
    fn test_string_length_validator() {
        let validator = StringLengthValidator::new("test field")
            .with_min_length(2)
            .with_max_length(5);

        assert!(validator.validate("ab").is_ok());
        assert!(validator.validate("abc").is_ok());
        assert!(validator.validate("abcde").is_ok());

        assert!(validator.validate("a").is_err());
        assert!(validator.validate("abcdef").is_err());
    }

    #[test]
    fn test_validation_state() {
        let valid_state = ValidationState::valid();
        assert!(valid_state.is_valid);
        assert!(valid_state.error_message.is_none());

        let invalid_state = ValidationState::invalid("Error message");
        assert!(!invalid_state.is_valid);
        assert_eq!(
            invalid_state.error_message,
            Some("Error message".to_string())
        );

        let from_result_ok = ValidationState::from_result(Ok::<(), String>(()));
        assert!(from_result_ok.is_valid);

        let from_result_err =
            ValidationState::from_result(Err::<(), String>("Test error".to_string()));
        assert!(!from_result_err.is_valid);
        assert_eq!(
            from_result_err.error_message,
            Some("Test error".to_string())
        );
    }
}
