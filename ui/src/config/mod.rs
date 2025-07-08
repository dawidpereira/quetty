use config::{Config, Environment, File};
use serde::Deserialize;

// Re-export all submodules
pub mod app;
pub mod auth;
pub mod azure;
pub mod keys;
pub mod limits;
pub mod ui;
pub mod validation;

// Re-export main types for backward compatibility
pub use app::AppConfig;
pub use validation::{ConfigLoadResult, ConfigValidationError};

/// Global configuration loading and access
static CONFIG: std::sync::OnceLock<ConfigLoadResult> = std::sync::OnceLock::new();

/// Reloadable configuration that can be updated at runtime
static RELOADABLE_CONFIG: std::sync::OnceLock<std::sync::RwLock<Option<ConfigLoadResult>>> =
    std::sync::OnceLock::new();

/// Global current page size that can be changed during runtime
static CURRENT_PAGE_SIZE: std::sync::OnceLock<std::sync::Mutex<Option<u32>>> =
    std::sync::OnceLock::new();

fn load_config() -> ConfigLoadResult {
    dotenv::dotenv().ok();
    let env_source = Environment::default().separator("__");

    // Configuration file is mandatory now – fail fast when it is missing so the
    // user is clearly informed that a valid `config.toml` must be provided.
    let file_source = File::with_name("config.toml");

    let config = match Config::builder()
        .add_source(file_source)
        .add_source(env_source) // environment entries still override file values when present
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

/// Get the current page size, falling back to config if not set
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

/// Set the current page size
pub fn set_current_page_size(page_size: u32) {
    let current_page_size = CURRENT_PAGE_SIZE.get_or_init(|| std::sync::Mutex::new(None));
    if let Ok(mut guard) = current_page_size.lock() {
        *guard = Some(page_size);
    }
}

/// Reload the configuration from files and environment variables
/// This function forces a reload of configuration from disk and environment variables
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

/// Load configuration fresh, bypassing any cache
fn load_config_fresh() -> ConfigLoadResult {
    // Reload environment variables from .env file
    dotenv::dotenv().ok();

    let env_source = Environment::default().separator("__");
    let file_source = File::with_name("config.toml");

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

/// Additional logging configuration
#[derive(Debug, Deserialize, Default, Clone)]
pub struct LoggingConfig {
    level: Option<String>,
    file: Option<String>,
}

impl LoggingConfig {
    pub fn level(&self) -> &str {
        self.level.as_deref().unwrap_or("info")
    }

    pub fn file(&self) -> Option<&str> {
        self.file.as_deref()
    }
}
