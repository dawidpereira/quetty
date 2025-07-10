//! Environment variable utilities for safe and validated access.
//!
//! This module provides utilities for safely accessing environment variables
//! with proper validation and error handling. It ensures that environment
//! variables are not only present but also contain valid, non-empty values.

use thiserror::Error;

/// Errors that can occur when accessing environment variables.
///
/// Provides detailed error information for different failure scenarios
/// when working with environment variables, including missing variables,
/// encoding issues, and empty values.
#[derive(Debug, Error)]
pub enum EnvVarError {
    /// Environment variable is not set
    #[error(
        "Environment variable '{name}' not found. Please set this variable in your .env file or environment."
    )]
    NotFound { name: String },

    /// Environment variable contains invalid UTF-8 characters
    #[error(
        "Environment variable '{name}' contains invalid UTF-8 characters. Please check the value."
    )]
    InvalidUtf8 { name: String },

    /// Environment variable is set but contains only whitespace or is empty
    #[error("Environment variable '{name}' is empty. Please provide a valid value.")]
    Empty { name: String },
}

/// Utility functions for safe environment variable handling.
///
/// Provides methods for safely accessing environment variables with proper
/// validation and error handling. All methods trim whitespace and validate
/// that values are not empty.
///
/// # Examples
///
/// ```no_run
/// use server::utils::EnvUtils;
///
/// // Check if a variable exists and has a value
/// if EnvUtils::has_non_empty_var("DATABASE_URL") {
///     // Get the validated value
///     let url = EnvUtils::get_validated_var("DATABASE_URL")?;
///     println!("Database URL: {}", url);
/// }
///
/// // Get an optional variable
/// if let Some(debug_level) = EnvUtils::get_optional_var("DEBUG_LEVEL") {
///     println!("Debug level: {}", debug_level);
/// }
/// ```
pub struct EnvUtils;

impl EnvUtils {
    /// Checks if an environment variable exists and has a non-empty value.
    ///
    /// This method checks both that the variable is set and that it contains
    /// non-whitespace content after trimming.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the environment variable to check
    ///
    /// # Returns
    ///
    /// `true` if the variable exists and has a non-empty value, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use server::utils::EnvUtils;
    ///
    /// if EnvUtils::has_non_empty_var("API_KEY") {
    ///     println!("API key is configured");
    /// } else {
    ///     println!("API key is missing or empty");
    /// }
    /// ```
    pub fn has_non_empty_var(name: &str) -> bool {
        match std::env::var(name) {
            Ok(value) => !value.trim().is_empty(),
            Err(_) => false,
        }
    }

    /// Gets an environment variable with validation.
    ///
    /// Retrieves the environment variable, trims whitespace, and validates
    /// that it contains a non-empty value. Returns detailed error information
    /// if the variable is missing, empty, or contains invalid UTF-8.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the environment variable to retrieve
    ///
    /// # Returns
    ///
    /// The trimmed, validated environment variable value
    ///
    /// # Errors
    ///
    /// Returns [`EnvVarError`] if:
    /// - The variable is not set ([`EnvVarError::NotFound`])
    /// - The variable is empty or contains only whitespace ([`EnvVarError::Empty`])
    /// - The variable contains invalid UTF-8 ([`EnvVarError::InvalidUtf8`])
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use server::utils::EnvUtils;
    ///
    /// match EnvUtils::get_validated_var("DATABASE_URL") {
    ///     Ok(url) => println!("Database URL: {}", url),
    ///     Err(e) => eprintln!("Configuration error: {}", e),
    /// }
    /// ```
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

    /// Gets an optional environment variable.
    ///
    /// Returns the validated environment variable value if it exists and is valid,
    /// or `None` if it's missing, empty, or invalid. This is a convenience method
    /// for cases where the absence of an environment variable is acceptable.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the environment variable to retrieve
    ///
    /// # Returns
    ///
    /// `Some(value)` if the variable exists and is valid, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use server::utils::EnvUtils;
    ///
    /// let debug_mode = EnvUtils::get_optional_var("DEBUG_MODE")
    ///     .unwrap_or_else(|| "false".to_string());
    ///
    /// if let Some(custom_config) = EnvUtils::get_optional_var("CUSTOM_CONFIG") {
    ///     println!("Using custom config: {}", custom_config);
    /// } else {
    ///     println!("Using default configuration");
    /// }
    /// ```
    pub fn get_optional_var(name: &str) -> Option<String> {
        Self::get_validated_var(name).ok()
    }
}
