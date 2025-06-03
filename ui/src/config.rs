use config::{Config, Environment};
use lazy_static::lazy_static;
use serde::Deserialize;
use server::service_bus_manager::AzureAdConfig;
use std::time::Duration;

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
    tick_interval_millis: u64,
    #[serde(flatten)]
    dlq: DLQConfig,
    servicebus: ServicebusConfig,
    azure_ad: AzureAdConfig,
    logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    level: Option<String>,
    file: Option<String>,
}

/// Configuration for Dead Letter Queue (DLQ) operations
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DLQConfig {
    /// Timeout for receiving messages from DLQ (default: 10 seconds)
    dlq_receive_timeout_secs: Option<u64>,
    /// Timeout for sending messages to main queue (default: 10 seconds)
    dlq_send_timeout_secs: Option<u64>,
    /// Maximum attempts to find a message in DLQ (default: 10)
    dlq_max_attempts: Option<usize>,
    /// Maximum total time for entire resend operation (default: 60 seconds)
    dlq_overall_timeout_cap_secs: Option<u64>,
    /// Hard cap for receive timeouts (default: 10 seconds)
    dlq_receive_timeout_cap_secs: Option<u64>,
    /// Hard cap for send timeouts (default: 15 seconds)
    dlq_send_timeout_cap_secs: Option<u64>,
    /// Delay between retry attempts when no messages found (default: 500ms)
    dlq_retry_delay_ms: Option<u64>,
    /// Batch size for receiving messages in DLQ operations (default: 10)
    dlq_batch_size: Option<u32>,
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
        Duration::from_millis(self.tick_interval_millis)
    }
    pub fn dlq(&self) -> &DLQConfig {
        &self.dlq
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

impl DLQConfig {
    /// Get the timeout for receiving messages from DLQ
    pub fn receive_timeout_secs(&self) -> u64 {
        self.dlq_receive_timeout_secs.unwrap_or(10)
    }

    /// Get the timeout for sending messages to main queue
    pub fn send_timeout_secs(&self) -> u64 {
        self.dlq_send_timeout_secs.unwrap_or(10)
    }

    /// Get the maximum attempts to find a message in DLQ
    pub fn max_attempts(&self) -> usize {
        self.dlq_max_attempts.unwrap_or(10)
    }

    /// Get the maximum total time for entire resend operation
    pub fn overall_timeout_cap_secs(&self) -> u64 {
        self.dlq_overall_timeout_cap_secs.unwrap_or(60)
    }

    /// Get the hard cap for receive timeouts
    pub fn receive_timeout_cap_secs(&self) -> u64 {
        self.dlq_receive_timeout_cap_secs.unwrap_or(10)
    }

    /// Get the hard cap for send timeouts
    pub fn send_timeout_cap_secs(&self) -> u64 {
        self.dlq_send_timeout_cap_secs.unwrap_or(15)
    }

    /// Get the delay between retry attempts when no messages found
    pub fn retry_delay_ms(&self) -> u64 {
        self.dlq_retry_delay_ms.unwrap_or(500)
    }

    /// Get the batch size for receiving messages in DLQ operations
    pub fn batch_size(&self) -> u32 {
        self.dlq_batch_size.unwrap_or(10)
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
