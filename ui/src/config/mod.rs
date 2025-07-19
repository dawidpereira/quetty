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
//! ```ignore
//! use quetty::config::{get_config_or_panic, reload_config};
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
/// Default embedded configurations and themes
pub mod defaults;
/// Key bindings and shortcut configuration
pub mod keys;
/// Application limits and constraints
pub mod limits;
/// Configuration directory setup and management
pub mod setup;
/// User interface configuration
pub mod ui;
/// Configuration validation logic
pub mod validation;
/// Interactive setup wizard for first-time configuration
pub mod wizard;
// Re-export main types for convenient access
pub use app::AppConfig;
pub use setup::{find_config_file, get_config_dir, initialize_config_dir, is_config_initialized};
pub use validation::{ConfigLoadResult, ConfigValidationError};

/// Global configuration storage - initialized once at startup
static CONFIG: std::sync::OnceLock<ConfigLoadResult> = std::sync::OnceLock::new();

/// Runtime-reloadable configuration that can be updated without restart
static RELOADABLE_CONFIG: std::sync::OnceLock<std::sync::RwLock<Option<ConfigLoadResult>>> =
    std::sync::OnceLock::new();

/// Profile information for caching
#[derive(Debug, Clone)]
struct ProfileInfo {
    exists: bool,
    last_checked: std::time::SystemTime,
}

/// Cache for profile information to avoid repeated filesystem scans
struct ProfileCache {
    profiles: std::sync::RwLock<std::collections::HashMap<String, ProfileInfo>>,
    cached_list: std::sync::RwLock<Option<(Vec<String>, std::time::SystemTime)>>,
    cache_ttl: std::time::Duration,
}

impl ProfileCache {
    fn new() -> Self {
        Self {
            profiles: std::sync::RwLock::new(std::collections::HashMap::new()),
            cached_list: std::sync::RwLock::new(None),
            cache_ttl: std::time::Duration::from_secs(30), // 30 second TTL
        }
    }

    fn is_cached_valid(&self, last_checked: std::time::SystemTime) -> bool {
        std::time::SystemTime::now()
            .duration_since(last_checked)
            .map(|duration| duration < self.cache_ttl)
            .unwrap_or(false)
    }

    fn get_profile_exists(&self, profile_name: &str) -> Option<bool> {
        let profiles = self.profiles.read().ok()?;
        let profile_info = profiles.get(profile_name)?;

        if self.is_cached_valid(profile_info.last_checked) {
            Some(profile_info.exists)
        } else {
            None
        }
    }

    fn cache_profile_exists(&self, profile_name: &str, exists: bool) {
        if let Ok(mut profiles) = self.profiles.write() {
            profiles.insert(
                profile_name.to_string(),
                ProfileInfo {
                    exists,
                    last_checked: std::time::SystemTime::now(),
                },
            );
        }
    }

    fn get_cached_profile_list(&self) -> Option<Vec<String>> {
        let cached_list = self.cached_list.read().ok()?;
        if let Some((ref list, last_checked)) = *cached_list {
            if self.is_cached_valid(last_checked) {
                return Some(list.clone());
            }
        }
        None
    }

    fn cache_profile_list(&self, profiles: Vec<String>) {
        if let Ok(mut cached_list) = self.cached_list.write() {
            *cached_list = Some((profiles, std::time::SystemTime::now()));
        }
    }

    fn invalidate(&self) {
        if let Ok(mut profiles) = self.profiles.write() {
            profiles.clear();
        }
        if let Ok(mut cached_list) = self.cached_list.write() {
            *cached_list = None;
        }
    }
}

/// Global profile cache
static PROFILE_CACHE: std::sync::OnceLock<ProfileCache> = std::sync::OnceLock::new();

/// Get or initialize the profile cache
fn get_profile_cache() -> &'static ProfileCache {
    PROFILE_CACHE.get_or_init(ProfileCache::new)
}

/// Current page size that can be dynamically changed by the user
static CURRENT_PAGE_SIZE: std::sync::OnceLock<std::sync::Mutex<Option<u32>>> =
    std::sync::OnceLock::new();

/// Load configuration with optional custom config file path
fn load_config_with_custom_path(custom_config_path: Option<&str>) -> ConfigLoadResult {
    // Load .env file from the default profile directory
    if let Ok(config_dir) = setup::get_config_dir() {
        let profile_env_path = config_dir.join("profiles").join("default").join(".env");
        if profile_env_path.exists() {
            dotenv::from_path(profile_env_path).ok();
        }
    }

    // Also try loading .env from current directory (for backward compatibility)
    dotenv::from_path(".env").ok();

    let env_source = Environment::default().separator("__");

    // Determine config file path using discovery or custom path
    let config_path = if let Some(custom_path) = custom_config_path {
        Some(custom_path.to_string())
    } else {
        // Look for config file in profile directory first, then fall back to general config
        if let Ok(config_dir) = setup::get_config_dir() {
            let profile_config_path = config_dir
                .join("profiles")
                .join("default")
                .join("config.toml");
            if profile_config_path.exists() {
                Some(profile_config_path.to_string_lossy().to_string())
            } else {
                // Fall back to general config discovery
                find_config_file().map(|p| p.to_string_lossy().to_string())
            }
        } else {
            // If we can't get config dir, try general discovery
            find_config_file().map(|p| p.to_string_lossy().to_string())
        }
    };

    let config = match config_path {
        Some(path) => {
            use std::path::Path;

            // Check if the custom config file exists
            if !Path::new(&path).exists() && custom_config_path.is_some() {
                // Custom config path specified but file doesn't exist
                log::warn!("Custom config file not found: {path}");

                // Try to create the config file with defaults
                match create_config_file_with_defaults(&path) {
                    Ok(()) => {
                        log::info!("Created default config file at: {path}");
                    }
                    Err(e) => {
                        return ConfigLoadResult::LoadError(format!(
                            "Custom config file '{path}' does not exist and failed to create it with defaults: {e}\n\
                            \nSuggestions:\n\
                            1. Create the config file manually\n\
                            2. Use 'quetty --setup' for interactive configuration\n\
                            3. Run without --config to use default locations"
                        ));
                    }
                }
            }

            log::info!("Loading configuration from: {path}");
            let file_source = File::with_name(&path);

            match Config::builder()
                .add_source(file_source)
                .add_source(env_source) // environment entries override file values
                .build()
            {
                Ok(config) => config,
                Err(e) => {
                    return ConfigLoadResult::LoadError(format!(
                        "Configuration loading failed from {path}: {e}. Please check your config file and environment variables."
                    ));
                }
            }
        }
        None => {
            // No config file found, try to initialize with defaults
            log::warn!("No configuration file found. Attempting to initialize with defaults...");

            match initialize_config_dir() {
                Ok(config_dir) => {
                    log::info!("Initialized config directory: {}", config_dir.display());

                    // Try loading the newly created config
                    if let Some(new_config_path) = find_config_file() {
                        let file_source = File::with_name(&new_config_path.to_string_lossy());
                        match Config::builder()
                            .add_source(file_source)
                            .add_source(env_source)
                            .build()
                        {
                            Ok(config) => config,
                            Err(e) => {
                                return ConfigLoadResult::LoadError(format!(
                                    "Failed to load newly created config: {e}"
                                ));
                            }
                        }
                    } else {
                        return ConfigLoadResult::LoadError(
                            "Failed to find config file after initialization".to_string(),
                        );
                    }
                }
                Err(e) => {
                    return ConfigLoadResult::LoadError(format!(
                        "No configuration file found and failed to initialize defaults: {e}. \
                        Please create a config.toml file or run with --setup flag."
                    ));
                }
            }
        }
    };

    match config.try_deserialize::<AppConfig>() {
        Ok(app_config) => ConfigLoadResult::Success(Box::new(app_config)),
        Err(e) => ConfigLoadResult::DeserializeError(format!("Failed to deserialize config: {e}")),
    }
}

/// Gets the current configuration using the unified profile-based system.
///
/// This function provides access to the application configuration with support
/// for runtime reloading. It first checks for reloaded configuration, then falls
/// back to the initial configuration if no reload has occurred.
///
/// # Returns
///
/// A reference to the [`ConfigLoadResult`] with static lifetime
pub fn get_config() -> &'static ConfigLoadResult {
    // Check if we have reloaded configuration first
    if let Some(reloadable_lock) = RELOADABLE_CONFIG.get() {
        if let Ok(guard) = reloadable_lock.read() {
            if let Some(ref reloaded_config) = *guard {
                log::debug!("Using reloaded configuration instead of cached config");
                // We have reloaded configuration - convert it to a static reference
                // This is safe because we're returning a reference to data that lives
                // as long as the static RELOADABLE_CONFIG
                return unsafe {
                    std::mem::transmute::<&ConfigLoadResult, &ConfigLoadResult>(reloaded_config)
                };
            }
        }
    }

    // Fall back to initial configuration loading
    get_config_for_profile("default")
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
/// use quetty::config::get_config_or_panic;
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
/// use quetty::config::get_current_page_size;
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
/// use quetty::config::set_current_page_size;
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
/// use quetty::config::reload_config;
///
/// match reload_config() {
///     Ok(()) => println!("Configuration reloaded successfully"),
///     Err(e) => eprintln!("Failed to reload config: {}", e),
/// }
/// ```
pub fn reload_config() -> Result<(), String> {
    log::info!("Reloading configuration from files and environment variables");

    // Load fresh configuration
    let fresh_config = load_config_with_custom_path(None);

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
/// Get configuration from custom path and store it globally
pub fn init_config_from_path(config_path: &str) -> &'static ConfigLoadResult {
    CONFIG.get_or_init(|| load_config_with_custom_path(Some(config_path)))
}

/// Get configuration for specified profile and store it globally
pub fn get_config_for_profile(profile_name: &str) -> &'static ConfigLoadResult {
    CONFIG.get_or_init(|| load_config_for_profile(profile_name))
}

/// Validate profile name for security and correctness
///
/// Ensures the profile name:
/// - Is not empty and not longer than 64 characters
/// - Does not contain path traversal sequences (checked first for security)
/// - Contains only alphanumeric characters, dashes, and underscores
pub fn validate_profile_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Profile name cannot be empty".to_string());
    }

    if name.len() > 64 {
        return Err("Profile name cannot be longer than 64 characters".to_string());
    }

    // Check for path traversal attacks FIRST (security priority)
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err(
            "Profile name cannot contain path separators or traversal sequences".to_string(),
        );
    }

    // Prevent reserved names
    if name == "." || name == ".." {
        return Err("Profile name cannot be '.' or '..'".to_string());
    }

    // Check for valid characters only (alphanumeric, dash, underscore)
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(
            "Profile name can only contain letters, numbers, dashes, and underscores".to_string(),
        );
    }

    Ok(())
}

/// Safely construct a profile directory path with validation
///
/// This function validates the profile name before constructing the path
/// to prevent path traversal attacks.
fn safe_profile_path(profile_name: &str) -> Result<std::path::PathBuf, String> {
    use crate::config::setup::get_config_dir;

    // Validate the profile name first
    validate_profile_name(profile_name)?;

    // Get config directory
    let config_dir =
        get_config_dir().map_err(|e| format!("Failed to determine config directory: {e}"))?;

    // Construct safe path
    Ok(config_dir.join("profiles").join(profile_name))
}

/// Check if a profile exists (with caching for performance)
pub fn profile_exists(profile_name: &str) -> bool {
    let cache = get_profile_cache();

    // Check cache first
    if let Some(cached_result) = cache.get_profile_exists(profile_name) {
        return cached_result;
    }

    // If not cached or expired, check filesystem
    let exists = match safe_profile_path(profile_name) {
        Ok(profile_dir) => profile_dir.exists() && profile_dir.join(".env").exists(),
        Err(_) => false, // Invalid profile name
    };

    // Cache the result
    cache.cache_profile_exists(profile_name, exists);

    exists
}

/// List all available profiles (with caching for performance)
pub fn list_available_profiles() -> Vec<String> {
    use crate::config::setup::get_config_dir;

    let cache = get_profile_cache();

    // Check cache first
    if let Some(cached_profiles) = cache.get_cached_profile_list() {
        return cached_profiles;
    }

    // If not cached or expired, scan filesystem
    let mut profiles = Vec::new();

    if let Ok(config_dir) = get_config_dir() {
        let profiles_dir = config_dir.join("profiles");
        if let Ok(entries) = std::fs::read_dir(profiles_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_dir() {
                        if let Some(name) = entry.file_name().to_str() {
                            // Validate profile name for security
                            if validate_profile_name(name).is_ok() {
                                // Check if it has a .env file (basic validation that it's a real profile)
                                let env_path = entry.path().join(".env");
                                if env_path.exists() {
                                    profiles.push(name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    profiles.sort();

    // Cache the result
    cache.cache_profile_list(profiles.clone());

    profiles
}

/// Invalidate profile cache (call after creating/deleting profiles)
pub fn invalidate_profile_cache() {
    get_profile_cache().invalidate();
}

/// Load configuration for a specific profile
fn load_config_for_profile(profile_name: &str) -> ConfigLoadResult {
    // Validate profile name and get safe path
    let profile_dir = match safe_profile_path(profile_name) {
        Ok(path) => path,
        Err(validation_error) => {
            return ConfigLoadResult::LoadError(format!(
                "Invalid profile name '{profile_name}': {validation_error}"
            ));
        }
    };

    // Check if profile exists first
    if !profile_exists(profile_name) {
        let available_profiles = list_available_profiles();
        let profile_list = if available_profiles.is_empty() {
            "No profiles found. Run 'quetty --setup' to create your first profile.".to_string()
        } else {
            format!("Available profiles: {}", available_profiles.join(", "))
        };

        return ConfigLoadResult::LoadError(format!(
            "Profile '{profile_name}' does not exist.\n\n{profile_list}\n\nTo create a new profile, run: quetty -p {profile_name} --setup"
        ));
    }

    // Load .env file from profile
    if let Ok(env_path) = profile_dir.join(".env").canonicalize() {
        dotenv::from_path(env_path).ok();
    }

    // Try to find profile-specific config file, fall back to embedded defaults
    let profile_config_path = profile_dir.join("config.toml");
    let config_path = if profile_config_path.exists() {
        Some(profile_config_path.to_string_lossy().to_string())
    } else {
        None
    };

    load_config_with_custom_path(config_path.as_deref())
}

/// Create a config file at the specified path with embedded defaults
fn create_config_file_with_defaults(config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use crate::config::defaults::get_complete_default_config;
    use std::path::Path;

    let path = Path::new(config_path);

    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write the complete default configuration (base config + keys)
    std::fs::write(path, get_complete_default_config())?;

    // Set restrictive permissions on Unix systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(0o600); // rw-------
        std::fs::set_permissions(path, permissions)?;
    }

    Ok(())
}

/// Configuration for application logging behavior.
///
/// Controls log level, output file settings, and log rotation for the application logger.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct LoggingConfig {
    level: Option<String>,
    file: Option<String>,
    max_file_size_mb: Option<u64>,
    max_backup_files: Option<u32>,
    cleanup_on_startup: Option<bool>,
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

    /// Gets the maximum log file size in MB before rotation, defaulting to 10MB.
    ///
    /// # Returns
    ///
    /// Maximum file size in megabytes
    pub fn max_file_size_mb(&self) -> u64 {
        self.max_file_size_mb.unwrap_or(10)
    }

    /// Gets the maximum number of backup files to keep, defaulting to 5.
    ///
    /// # Returns
    ///
    /// Maximum number of backup log files
    pub fn max_backup_files(&self) -> u32 {
        self.max_backup_files.unwrap_or(5)
    }

    /// Gets whether to clean up old log files on startup, defaulting to true.
    ///
    /// # Returns
    ///
    /// `true` if old log files should be cleaned up on startup
    pub fn cleanup_on_startup(&self) -> bool {
        self.cleanup_on_startup.unwrap_or(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_profile_name_valid_names() {
        // Valid profile names
        assert!(validate_profile_name("default").is_ok());
        assert!(validate_profile_name("dev").is_ok());
        assert!(validate_profile_name("production").is_ok());
        assert!(validate_profile_name("test-env").is_ok());
        assert!(validate_profile_name("test_env").is_ok());
        assert!(validate_profile_name("dev123").is_ok());
        assert!(validate_profile_name("a").is_ok());
        assert!(validate_profile_name("A-B_C1").is_ok());
    }

    #[test]
    fn test_validate_profile_name_invalid_names() {
        // Empty name
        assert!(validate_profile_name("").is_err());

        // Too long (> 64 characters)
        let long_name = "a".repeat(65);
        assert!(validate_profile_name(&long_name).is_err());

        // Path traversal attacks
        assert!(validate_profile_name("../etc/passwd").is_err());
        assert!(validate_profile_name("../../root").is_err());
        assert!(validate_profile_name("..\\windows").is_err());
        assert!(validate_profile_name("test/../etc").is_err());
        assert!(validate_profile_name("test\\..\\etc").is_err());

        // Path separators
        assert!(validate_profile_name("test/profile").is_err());
        assert!(validate_profile_name("test\\profile").is_err());
        assert!(validate_profile_name("/etc/passwd").is_err());
        assert!(validate_profile_name("C:\\Windows").is_err());

        // Reserved names
        assert!(validate_profile_name(".").is_err());
        assert!(validate_profile_name("..").is_err());

        // Invalid characters
        assert!(validate_profile_name("test profile").is_err()); // space
        assert!(validate_profile_name("test@profile").is_err()); // @
        assert!(validate_profile_name("test#profile").is_err()); // #
        assert!(validate_profile_name("test$profile").is_err()); // $
        assert!(validate_profile_name("test%profile").is_err()); // %
        assert!(validate_profile_name("test^profile").is_err()); // ^
        assert!(validate_profile_name("test&profile").is_err()); // &
        assert!(validate_profile_name("test*profile").is_err()); // *
        assert!(validate_profile_name("test(profile").is_err()); // (
        assert!(validate_profile_name("test)profile").is_err()); // )
        assert!(validate_profile_name("test+profile").is_err()); // +
        assert!(validate_profile_name("test=profile").is_err()); // =
        assert!(validate_profile_name("test[profile").is_err()); // [
        assert!(validate_profile_name("test]profile").is_err()); // ]
        assert!(validate_profile_name("test{profile").is_err()); // {
        assert!(validate_profile_name("test}profile").is_err()); // }
        assert!(validate_profile_name("test|profile").is_err()); // |
        assert!(validate_profile_name("test:profile").is_err()); // :
        assert!(validate_profile_name("test;profile").is_err()); // ;
        assert!(validate_profile_name("test\"profile").is_err()); // "
        assert!(validate_profile_name("test'profile").is_err()); // '
        assert!(validate_profile_name("test<profile").is_err()); // <
        assert!(validate_profile_name("test>profile").is_err()); // >
        assert!(validate_profile_name("test,profile").is_err()); // ,
        assert!(validate_profile_name("test?profile").is_err()); // ?
        assert!(validate_profile_name("test`profile").is_err()); // `
        assert!(validate_profile_name("test~profile").is_err()); // ~
        assert!(validate_profile_name("test!profile").is_err()); // !
    }

    #[test]
    fn test_safe_profile_path_security() {
        // Valid profile should work
        let result = safe_profile_path("valid-profile");
        assert!(result.is_ok());
        if let Ok(path) = result {
            let path_str = path.to_string_lossy();
            assert!(path_str.contains("profiles"));
            assert!(path_str.contains("valid-profile"));
            assert!(!path_str.contains(".."));
        }

        // Path traversal should fail
        assert!(safe_profile_path("../etc/passwd").is_err());
        assert!(safe_profile_path("../../root").is_err());
        assert!(safe_profile_path("..\\windows").is_err());

        // Empty profile should fail
        assert!(safe_profile_path("").is_err());

        // Invalid characters should fail
        assert!(safe_profile_path("test/profile").is_err());
        assert!(safe_profile_path("test\\profile").is_err());
    }

    #[test]
    fn test_profile_cache_functionality() {
        let cache = ProfileCache::new();

        // Initially no cache entry
        assert!(cache.get_profile_exists("test").is_none());

        // Cache a result
        cache.cache_profile_exists("test", true);
        assert_eq!(cache.get_profile_exists("test"), Some(true));

        // Cache a different result
        cache.cache_profile_exists("test2", false);
        assert_eq!(cache.get_profile_exists("test2"), Some(false));

        // Test profile list cache
        assert!(cache.get_cached_profile_list().is_none());

        let test_profiles = vec!["profile1".to_string(), "profile2".to_string()];
        cache.cache_profile_list(test_profiles.clone());
        assert_eq!(cache.get_cached_profile_list(), Some(test_profiles));

        // Test invalidation
        cache.invalidate();
        assert!(cache.get_profile_exists("test").is_none());
        assert!(cache.get_cached_profile_list().is_none());
    }

    #[test]
    fn test_profile_validation_error_messages() {
        // Test specific error messages
        let result = validate_profile_name("");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot be empty"));

        let result = validate_profile_name("../etc/passwd");
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(error_msg.contains("traversal"));

        let result = validate_profile_name("test profile");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("letters, numbers, dashes, and underscores")
        );

        let long_name = "a".repeat(65);
        let result = validate_profile_name(&long_name);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("longer than 64 characters"));
    }
}
