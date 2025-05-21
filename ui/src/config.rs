use lazy_static::lazy_static;
use std::time::Duration;

use config::{Config, Environment};
use serde::Deserialize;
use server::service_bus_manager::AzureAdConfig;

lazy_static! {
    pub static ref CONFIG: AppConfig = {
        dotenv::dotenv().ok();
        let env_source = Environment::default().separator("__");
        let file_source = config::File::with_name("config.toml");

        let config = Config::builder()
            .add_source(file_source)
            .add_source(env_source)
            .build()
            .expect("Failed to load configuration");

        config
            .try_deserialize::<AppConfig>()
            .expect("Failed to deserialize configuration")
    };
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AppConfig {
    max_messages: u32,
    crossterm_input_listener_interval_ms: u64,
    crossterm_input_listener_retries: usize,
    poll_timeout_ms: u64,
    tick_interval_secs: u64,
    servicebus: ServicebusConfig,
    azure_ad: AzureAdConfig,
    logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    level: Option<String>,
    file: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: Some("info".to_string()),
            file: None,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ServicebusConfig {
    connection_string: String,
}

impl AppConfig {
    pub fn max_messages(&self) -> u32 {
        self.max_messages
    }
    pub fn crossterm_input_listener_interval(&self) -> Duration {
        Duration::from_millis(self.crossterm_input_listener_interval_ms)
    }
    pub fn crossterm_input_listener_retries(&self) -> usize {
        self.crossterm_input_listener_retries
    }
    pub fn poll_timeout(&self) -> Duration {
        Duration::from_millis(self.poll_timeout_ms)
    }
    pub fn tick_interval(&self) -> Duration {
        Duration::from_secs(self.tick_interval_secs)
    }
    pub fn servicebus(&self) -> &ServicebusConfig {
        &self.servicebus
    }
    pub fn azure_ad(&self) -> &AzureAdConfig {
        &self.azure_ad
    }
    pub fn logging(&self) -> &LoggingConfig {
        &self.logging
    }
}

impl ServicebusConfig {
    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }
}

impl LoggingConfig {
    pub fn level(&self) -> &str {
        self.level.as_deref().unwrap_or("info")
    }
    
    pub fn file(&self) -> Option<&str> {
        self.file.as_deref()
    }
}
