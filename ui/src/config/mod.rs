//! # Configuration Module
//!
//! Centralized configuration management for the Quetty application.
//! Handles loading, validation, and runtime management of application settings
//! from TOML files and environment variables.
//!
//! ## Features
//!
//! - **Multi-Source Loading** - Loads from config.toml and environment variables
//! - **Runtime Reloading** - Hot reload configuration without restart
//! - **Type Safety** - Strongly typed configuration with validation
//! - **Global Access** - Thread-safe global configuration access
//! - **Environment Override** - Environment variables override file settings
//! - **Validation** - Comprehensive configuration validation
//!
//! ## Configuration Structure
//!
//! The configuration is organized into logical modules:
//! - [`app`] - Core application settings
//! - [`auth`] - Authentication configuration
//! - [`azure`] - Azure-specific settings
//! - [`keys`] - Key bindings and shortcuts
//! - [`limits`] - Application limits and constraints
//! - [`ui`] - User interface settings
//!
//! ## Usage
//!
//! ```no_run
//! use ui::config::{get_config_or_panic, reload_config};
//!
//! // Get configuration (loads on first access)
//! let config = get_config_or_panic();
//! let page_size = config.max_messages();
//!
//! // Reload configuration at runtime
//! reload_config()?;
//! ```
//!
//! ## Environment Variable Override
//!
//! All configuration values can be overridden with environment variables using
//! double underscore (`__`) as the separator:
//!
//! ```bash
//! export UI__PAGE_SIZE=50
//! export AUTH__METHOD="device_code"
//! export AZURE__TENANT_ID="your-tenant-id"
//! ```

use config::{Config, Environment, File};
use serde::Deserialize;

/// Core application configuration types and loading
pub mod app;
/// Authentication-related configuration
pub mod auth;
/// Azure-specific configuration settings
pub mod azure;
/// Key bindings and shortcut configuration
pub mod keys;
/// Application limits and constraints
pub mod limits;
/// User interface configuration
pub mod ui;
/// Configuration validation logic
pub mod validation;

// Re-export main types for convenient access
pub use app::AppConfig;
pub use validation::{ConfigLoadResult, ConfigValidationError};

/// Global configuration storage - initialized once at startup
static CONFIG: std::sync::OnceLock<ConfigLoadResult> = std::sync::OnceLock::new();

/// Runtime-reloadable configuration that can be updated without restart
static RELOADABLE_CONFIG: std::sync::OnceLock<std::sync::RwLock<Option<ConfigLoadResult>>> =
    std::sync::OnceLock::new();

/// Current page size that can be dynamically changed by the user
static CURRENT_PAGE_SIZE: std::sync::OnceLock<std::sync::Mutex<Option<u32>>> =
    std::sync::OnceLock::new();

/// Loads configuration from config.toml and environment variables.
///
/// This function loads configuration using a layered approach:
/// 1. Loads base configuration from config.toml
/// 2. Overlays environment variables (with `__` separator)
/// 3. Deserializes and validates the final configuration
///
/// # Returns
///
/// [`ConfigLoadResult`] indicating success or specific failure type
fn load_config() -> ConfigLoadResult {
    dotenv::dotenv().ok();
    let env_source = Environment::default().separator("__");

    // Configuration file is mandatory – fail fast when missing
    let file_source = File::with_name("../config.toml");

    let config = match Config::builder()
        .add_source(file_source)
        .add_source(env_source) // environment entries override file values
        .build()
    {
        Ok(config) => config,
        Err(e) => {
            return ConfigLoadResult::LoadError(format!(
                "Configuration loading failed: {e}. Please check your config.toml file and environment variables."
            ));
        }
    };

    match config.try_deserialize::<AppConfig>() {
        Ok(app_config) => ConfigLoadResult::Success(Box::new(app_config)),
        Err(e) => ConfigLoadResult::DeserializeError(format!("Failed to deserialize config: {e}")),
    }
}

/// Gets the current configuration, preferring reloaded config over initial config.
///
/// This function provides access to the application configuration with support
/// for runtime reloading. It first checks for a reloaded configuration, then
/// falls back to the initial configuration loaded at startup.
///
/// # Returns
///
/// A reference to the [`ConfigLoadResult`] with static lifetime
///
/// # Note
///
/// The static lifetime is achieved through intentional memory leaking for
/// reloaded configurations. This is acceptable since configuration instances
/// are meant to live for the application lifetime.
pub fn get_config() -> &'static ConfigLoadResult {
    // First check if we have a reloaded config
    let reloadable_lock = RELOADABLE_CONFIG.get_or_init(|| std::sync::RwLock::new(None));
    if let Ok(guard) = reloadable_lock.read() {
        if let Some(reloaded_config) = guard.as_ref() {
            // We have a reloaded config, return it by leaking it
            // This is intentional for the static lifetime requirement
            return Box::leak(Box::new(reloaded_config.clone()));
        }
    }

    // Fall back to the original config
    CONFIG.get_or_init(load_config)
}

/// Gets the application configuration, panicking if loading failed.
///
/// This is a convenience function for cases where configuration loading
/// failure should be treated as a fatal error. Use [`get_config`] if you
/// need to handle configuration errors gracefully.
///
/// # Returns
///
/// A reference to the successfully loaded [`AppConfig`]
///
/// # Panics
///
/// Panics if configuration loading or deserialization failed
///
/// # Examples
///
/// ```no_run
/// use ui::config::get_config_or_panic;
///
/// let config = get_config_or_panic();
/// let page_size = config.max_messages();
/// ```
pub fn get_config_or_panic() -> &'static AppConfig {
    match get_config() {
        ConfigLoadResult::Success(config) => config,
        ConfigLoadResult::LoadError(e) => {
            panic!("Failed to load config: {e}");
        }
        ConfigLoadResult::DeserializeError(e) => {
            panic!("Failed to deserialize config: {e}");
        }
    }
}

/// Gets the current page size for message display.
///
/// Returns the user-configured page size if set, otherwise falls back
/// to the default configured maximum messages value.
///
/// # Returns
///
/// Current page size as a u32
///
/// # Examples
///
/// ```no_run
/// use ui::config::get_current_page_size;
///
/// let page_size = get_current_page_size();
/// println!("Displaying {} messages per page", page_size);
/// ```
pub fn get_current_page_size() -> u32 {
    let current_page_size = CURRENT_PAGE_SIZE.get_or_init(|| std::sync::Mutex::new(None));
    if let Ok(guard) = current_page_size.lock() {
        if let Some(size) = *guard {
            return size;
        }
    }
    // Fall back to config value
    get_config_or_panic().max_messages()
}

/// Sets the current page size for message display.
///
/// This setting overrides the configured default and persists until
/// the application is restarted or the page size is changed again.
///
/// # Arguments
///
/// * `page_size` - New page size to use for message display
///
/// # Examples
///
/// ```no_run
/// use ui::config::set_current_page_size;
///
/// // Change to show 100 messages per page
/// set_current_page_size(100);
/// ```
pub fn set_current_page_size(page_size: u32) {
    let current_page_size = CURRENT_PAGE_SIZE.get_or_init(|| std::sync::Mutex::new(None));
    if let Ok(mut guard) = current_page_size.lock() {
        *guard = Some(page_size);
    }
}

/// Reloads configuration from files and environment variables.
///
/// This function forces a complete reload of the configuration from disk
/// and environment variables, allowing runtime configuration changes without
/// application restart. The new configuration is validated before being applied.
///
/// # Returns
///
/// `Ok(())` if reload succeeds, `Err(String)` with error details if it fails
///
/// # Errors
///
/// Returns an error if:
/// - Configuration files cannot be read
/// - Configuration format is invalid
/// - Configuration validation fails
/// - Lock acquisition fails
///
/// # Examples
///
/// ```no_run
/// use ui::config::reload_config;
///
/// match reload_config() {
///     Ok(()) => println!("Configuration reloaded successfully"),
///     Err(e) => eprintln!("Failed to reload config: {}", e),
/// }
/// ```
pub fn reload_config() -> Result<(), String> {
    log::info!("Reloading configuration from files and environment variables");

    // Load fresh configuration
    let fresh_config = load_config_fresh();

    // Update the reloadable configuration
    let reloadable_lock = RELOADABLE_CONFIG.get_or_init(|| std::sync::RwLock::new(None));
    match reloadable_lock.write() {
        Ok(mut guard) => {
            *guard = Some(fresh_config.clone());

            match fresh_config {
                ConfigLoadResult::Success(_) => {
                    log::info!("Configuration reloaded successfully");
                    Ok(())
                }
                ConfigLoadResult::LoadError(msg) => {
                    log::error!("Configuration reload failed: {msg}");
                    Err(msg)
                }
                ConfigLoadResult::DeserializeError(msg) => {
                    log::error!("Configuration reload failed during deserialization: {msg}");
                    Err(msg)
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to acquire write lock for configuration reload: {e}");
            log::error!("{error_msg}");
            Err(error_msg)
        }
    }
}

/// Loads a fresh configuration, bypassing all caches.
///
/// This function performs a complete configuration reload from disk and
/// environment variables, including re-reading the .env file and
/// validating the resulting configuration.
///
/// # Returns
///
/// [`ConfigLoadResult`] with the freshly loaded configuration
fn load_config_fresh() -> ConfigLoadResult {
    // Reload environment variables from .env file
    dotenv::dotenv().ok();

    let env_source = Environment::default().separator("__");
    let file_source = File::with_name("../config.toml");

    let config = match Config::builder()
        .add_source(file_source)
        .add_source(env_source)
        .build()
    {
        Ok(config) => config,
        Err(e) => {
            return ConfigLoadResult::LoadError(format!(
                "Configuration loading failed: {e}. Please check your config.toml file and environment variables."
            ));
        }
    };

    match config.try_deserialize::<AppConfig>() {
        Ok(app_config) => {
            // Validate the reloaded configuration
            if let Err(validation_errors) = app_config.validate() {
                let error_messages: Vec<String> =
                    validation_errors.iter().map(|e| e.user_message()).collect();
                return ConfigLoadResult::DeserializeError(format!(
                    "Configuration validation failed:\n{}",
                    error_messages.join("\n\n")
                ));
            }
            ConfigLoadResult::Success(Box::new(app_config))
        }
        Err(e) => ConfigLoadResult::DeserializeError(format!("Failed to deserialize config: {e}")),
    }
}

/// Configuration for application logging behavior.
///
/// Controls log level and output file settings for the application logger.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct LoggingConfig {
    level: Option<String>,
    file: Option<String>,
}

impl LoggingConfig {
    /// Gets the configured log level, defaulting to "info".
    ///
    /// # Returns
    ///
    /// Log level as a string ("trace", "debug", "info", "warn", "error")
    pub fn level(&self) -> &str {
        self.level.as_deref().unwrap_or("info")
    }

    /// Gets the configured log file path, if any.
    ///
    /// # Returns
    ///
    /// Optional log file path, `None` if logging to stdout/stderr
    pub fn file(&self) -> Option<&str> {
        self.file.as_deref()
    }
}
