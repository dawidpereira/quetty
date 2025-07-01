use config::{Config, Environment, File};
use serde::Deserialize;

// Re-export all submodules
pub mod app;
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
                "Configuration loading failed: {}. Please check your config.toml file and environment variables.",
                e
            ));
        }
    };

    match config.try_deserialize::<AppConfig>() {
        Ok(app_config) => ConfigLoadResult::Success(Box::new(app_config)),
        Err(e) => {
            ConfigLoadResult::DeserializeError(format!("Failed to deserialize config: {}", e))
        }
    }
}

pub fn get_config() -> &'static ConfigLoadResult {
    CONFIG.get_or_init(load_config)
}

pub fn get_config_or_panic() -> &'static AppConfig {
    match get_config() {
        ConfigLoadResult::Success(config) => config,
        ConfigLoadResult::LoadError(e) => {
            panic!("Failed to load config: {}", e);
        }
        ConfigLoadResult::DeserializeError(e) => {
            panic!("Failed to deserialize config: {}", e);
        }
    }
}

/// Additional logging configuration
#[derive(Debug, Deserialize, Default)]
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
