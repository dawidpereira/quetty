use thiserror::Error;

#[derive(Debug, Error)]
pub enum EnvVarError {
    #[error(
        "Environment variable '{name}' not found. Please set this variable in your .env file or environment."
    )]
    NotFound { name: String },

    #[error(
        "Environment variable '{name}' contains invalid UTF-8 characters. Please check the value."
    )]
    InvalidUtf8 { name: String },

    #[error("Environment variable '{name}' is empty. Please provide a valid value.")]
    Empty { name: String },
}

/// Utility functions for safe environment variable handling
pub struct EnvUtils;

impl EnvUtils {
    /// Check if an environment variable exists and has a non-empty value
    pub fn has_non_empty_var(name: &str) -> bool {
        match std::env::var(name) {
            Ok(value) => !value.trim().is_empty(),
            Err(_) => false,
        }
    }

    /// Get an environment variable with validation
    pub fn get_validated_var(name: &str) -> Result<String, EnvVarError> {
        match std::env::var(name) {
            Ok(value) => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    Err(EnvVarError::Empty {
                        name: name.to_string(),
                    })
                } else {
                    Ok(trimmed.to_string())
                }
            }
            Err(std::env::VarError::NotPresent) => Err(EnvVarError::NotFound {
                name: name.to_string(),
            }),
            Err(std::env::VarError::NotUnicode(_)) => Err(EnvVarError::InvalidUtf8 {
                name: name.to_string(),
            }),
        }
    }

    /// Get an optional environment variable (returns None if not found or empty)
    pub fn get_optional_var(name: &str) -> Option<String> {
        Self::get_validated_var(name).ok()
    }
}
