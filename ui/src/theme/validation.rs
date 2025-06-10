use crate::error::AppError;
use crate::theme::types::Theme;
use crate::validation::Validator;
use std::path::PathBuf;

/// Validation errors specific to theme operations
#[derive(Debug, Clone)]
pub enum ThemeValidationError {
    InvalidThemeName { name: String, reason: String },
    InvalidFlavorName { flavor: String, reason: String },
    InvalidThemePath { path: String, reason: String },
    MissingMetadata { field: String },
    InvalidFileExtension { path: String, expected: String },
}

impl ThemeValidationError {
    pub fn user_message(&self) -> String {
        match self {
            ThemeValidationError::InvalidThemeName { name, reason } => {
                format!(
                    "Invalid theme name: '{}'\n\n\
                    Reason: {}\n\n\
                    Please use valid theme names (alphanumeric, hyphens, underscores only).",
                    name, reason
                )
            }
            ThemeValidationError::InvalidFlavorName { flavor, reason } => {
                format!(
                    "Invalid flavor name: '{}'\n\n\
                    Reason: {}\n\n\
                    Please use valid flavor names (alphanumeric, hyphens, underscores only).",
                    flavor, reason
                )
            }
            ThemeValidationError::InvalidThemePath { path, reason } => {
                format!(
                    "Invalid theme path: '{}'\n\n\
                    Reason: {}\n\n\
                    Please ensure the path exists and is accessible.",
                    path, reason
                )
            }
            ThemeValidationError::MissingMetadata { field } => {
                format!(
                    "Missing theme metadata: '{}'\n\n\
                    Please ensure the theme file contains all required metadata fields.",
                    field
                )
            }
            ThemeValidationError::InvalidFileExtension { path, expected } => {
                format!(
                    "Invalid file extension for: '{}'\n\n\
                    Expected: '{}' files\n\n\
                    Please ensure theme files have the correct extension.",
                    path, expected
                )
            }
        }
    }
}

impl From<ThemeValidationError> for AppError {
    fn from(error: ThemeValidationError) -> Self {
        AppError::Config(error.user_message())
    }
}

/// Validator for theme names
pub struct ThemeNameValidator;

impl Validator<str> for ThemeNameValidator {
    type Error = ThemeValidationError;

    fn validate(&self, input: &str) -> Result<(), Self::Error> {
        // Check for empty name
        if input.is_empty() {
            return Err(ThemeValidationError::InvalidThemeName {
                name: input.to_string(),
                reason: "Name cannot be empty".to_string(),
            });
        }

        // Check length (reasonable limits)
        if input.len() > 50 {
            return Err(ThemeValidationError::InvalidThemeName {
                name: input.to_string(),
                reason: "Name too long (max 50 characters)".to_string(),
            });
        }

        // Check for valid characters (alphanumeric, hyphens, underscores)
        if !input
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ThemeValidationError::InvalidThemeName {
                name: input.to_string(),
                reason: "Name contains invalid characters (only alphanumeric, hyphens, and underscores allowed)".to_string(),
            });
        }

        // Check that it doesn't start or end with special characters
        if input.starts_with('-')
            || input.starts_with('_')
            || input.ends_with('-')
            || input.ends_with('_')
        {
            return Err(ThemeValidationError::InvalidThemeName {
                name: input.to_string(),
                reason: "Name cannot start or end with hyphens or underscores".to_string(),
            });
        }

        Ok(())
    }
}

/// Validator for flavor names (similar rules to theme names)
pub struct FlavorNameValidator;

impl Validator<str> for FlavorNameValidator {
    type Error = ThemeValidationError;

    fn validate(&self, input: &str) -> Result<(), Self::Error> {
        // Check for empty name
        if input.is_empty() {
            return Err(ThemeValidationError::InvalidFlavorName {
                flavor: input.to_string(),
                reason: "Flavor name cannot be empty".to_string(),
            });
        }

        // Check length
        if input.len() > 30 {
            return Err(ThemeValidationError::InvalidFlavorName {
                flavor: input.to_string(),
                reason: "Flavor name too long (max 30 characters)".to_string(),
            });
        }

        // Check for valid characters
        if !input
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ThemeValidationError::InvalidFlavorName {
                flavor: input.to_string(),
                reason: "Flavor name contains invalid characters (only alphanumeric, hyphens, and underscores allowed)".to_string(),
            });
        }

        // Check that it doesn't start or end with special characters
        if input.starts_with('-')
            || input.starts_with('_')
            || input.ends_with('-')
            || input.ends_with('_')
        {
            return Err(ThemeValidationError::InvalidFlavorName {
                flavor: input.to_string(),
                reason: "Flavor name cannot start or end with hyphens or underscores".to_string(),
            });
        }

        Ok(())
    }
}

/// Validator for theme paths
pub struct ThemePathValidator;

impl Validator<PathBuf> for ThemePathValidator {
    type Error = ThemeValidationError;

    fn validate(&self, input: &PathBuf) -> Result<(), Self::Error> {
        // Check if path exists
        if !input.exists() {
            return Err(ThemeValidationError::InvalidThemePath {
                path: input.display().to_string(),
                reason: "Path does not exist".to_string(),
            });
        }

        // Check if it's a file (not a directory)
        if !input.is_file() {
            return Err(ThemeValidationError::InvalidThemePath {
                path: input.display().to_string(),
                reason: "Path is not a file".to_string(),
            });
        }

        // Check file extension
        if input.extension().and_then(|s| s.to_str()) != Some("toml") {
            return Err(ThemeValidationError::InvalidFileExtension {
                path: input.display().to_string(),
                expected: "toml".to_string(),
            });
        }

        Ok(())
    }
}

/// Validator for loaded theme content
pub struct ThemeValidator;

impl Validator<Theme> for ThemeValidator {
    type Error = ThemeValidationError;

    fn validate(&self, input: &Theme) -> Result<(), Self::Error> {
        // Validate technical theme name in metadata if present
        if let Some(ref theme_name) = input.metadata.theme_name {
            let theme_name_validator = ThemeNameValidator;
            theme_name_validator
                .validate(theme_name)
                .map_err(|e| match e {
                    ThemeValidationError::InvalidThemeName { .. } => e,
                    _ => ThemeValidationError::InvalidThemeName {
                        name: theme_name.clone(),
                        reason: "Invalid theme name in metadata".to_string(),
                    },
                })?;
        }

        // Validate flavor name in metadata
        if let Some(ref flavor_name) = input.metadata.flavor_name {
            let flavor_validator = FlavorNameValidator;
            flavor_validator
                .validate(flavor_name)
                .map_err(|e| match e {
                    ThemeValidationError::InvalidFlavorName { .. } => e,
                    _ => ThemeValidationError::InvalidFlavorName {
                        flavor: flavor_name.clone(),
                        reason: "Invalid flavor name in metadata".to_string(),
                    },
                })?;
        }

        // Validate required metadata fields (display name can contain spaces, so we don't validate format)
        if input.metadata.name.is_empty() {
            return Err(ThemeValidationError::MissingMetadata {
                field: "name".to_string(),
            });
        }

        if input.metadata.description.is_empty() {
            return Err(ThemeValidationError::MissingMetadata {
                field: "description".to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_name_validator() {
        let validator = ThemeNameValidator;

        // Valid names
        assert!(validator.validate("valid_theme").is_ok());
        assert!(validator.validate("theme-name").is_ok());
        assert!(validator.validate("theme123").is_ok());

        // Invalid names
        assert!(validator.validate("").is_err());
        assert!(validator.validate("_invalid").is_err());
        assert!(validator.validate("invalid-").is_err());
        assert!(validator.validate("invalid@theme").is_err());
        assert!(validator.validate(&"a".repeat(51)).is_err());
    }

    #[test]
    fn test_flavor_name_validator() {
        let validator = FlavorNameValidator;

        // Valid names
        assert!(validator.validate("valid_flavor").is_ok());
        assert!(validator.validate("flavor-name").is_ok());
        assert!(validator.validate("flavor123").is_ok());

        // Invalid names
        assert!(validator.validate("").is_err());
        assert!(validator.validate("_invalid").is_err());
        assert!(validator.validate("invalid-").is_err());
        assert!(validator.validate("invalid@flavor").is_err());
        assert!(validator.validate(&"a".repeat(31)).is_err());
    }
}
