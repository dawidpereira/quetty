use config::{Config, Environment, File};
use lazy_static::lazy_static;
use serde::Deserialize;
use server::bulk_operations::BatchConfig;
use server::service_bus_manager::AzureAdConfig;
use std::time::Duration;

lazy_static! {
    pub static ref CONFIG: AppConfig = {
        dotenv::dotenv().ok();
        let env_source = Environment::default().separator("__");
        let file_source = File::with_name("config.toml");

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

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    max_messages: Option<u32>,
    crossterm_input_listener_interval_ms: Option<u64>,
    crossterm_input_listener_retries: Option<usize>,
    poll_timeout_ms: Option<u64>,
    tick_interval_millis: Option<u64>,
    #[serde(flatten)]
    dlq: DLQConfig,
    #[serde(flatten)]
    batch: BatchConfig,
    #[serde(flatten)]
    ui: UIConfig,
    servicebus: ServicebusConfig,
    azure_ad: AzureAdConfig,
    logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    level: Option<String>,
    file: Option<String>,
}

/// Configuration for UI elements
#[derive(Debug, Clone, Deserialize)]
pub struct UIConfig {
    /// Duration between animation frames for loading indicators (default: 100ms)
    ui_loading_frame_duration_ms: Option<u64>,
}

/// Configuration for Dead Letter Queue (DLQ) operations
#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
pub struct ServicebusConfig {
    connection_string: Option<String>,
}

impl AppConfig {
    pub fn max_messages(&self) -> u32 {
        self.max_messages.unwrap_or(10)
    }

    pub fn crossterm_input_listener_interval(&self) -> Duration {
        Duration::from_millis(self.crossterm_input_listener_interval_ms.unwrap_or(20))
    }
    pub fn crossterm_input_listener_retries(&self) -> usize {
        self.crossterm_input_listener_retries.unwrap_or(5)
    }
    pub fn poll_timeout(&self) -> Duration {
        Duration::from_millis(self.poll_timeout_ms.unwrap_or(10))
    }
    pub fn tick_interval(&self) -> Duration {
        Duration::from_millis(self.tick_interval_millis.unwrap_or(250))
    }
    pub fn dlq(&self) -> &DLQConfig {
        &self.dlq
    }
    pub fn batch(&self) -> &BatchConfig {
        &self.batch
    }
    pub fn ui(&self) -> &UIConfig {
        &self.ui
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
        self.connection_string.as_deref()
            .expect("SERVICEBUS_CONNECTION_STRING is required but not found in configuration or environment variables. Please set this value in .env file or environment.")
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

impl UIConfig {
    /// Get the duration between animation frames for loading indicators
    pub fn loading_frame_duration_ms(&self) -> u64 {
        self.ui_loading_frame_duration_ms.unwrap_or(100)
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
