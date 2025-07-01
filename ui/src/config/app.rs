use super::{
    LoggingConfig, azure::ServicebusConfig, keys::KeyBindingsConfig, limits::*, ui::UIConfig,
    validation::ConfigValidationError,
};
use crate::theme::types::ThemeConfig;
use serde::Deserialize;
use server::bulk_operations::BatchConfig;
use server::service_bus_manager::AzureAdConfig;
use std::time::Duration;

/// Main application configuration
#[derive(Debug, Deserialize)]
pub struct AppConfig {
    page_size: Option<u32>,
    crossterm_input_listener_interval_ms: Option<u64>,
    crossterm_input_listener_retries: Option<usize>,
    poll_timeout_ms: Option<u64>,
    tick_interval_millis: Option<u64>,
    // Queue statistics configuration
    queue_stats_display_enabled: Option<bool>,
    queue_stats_cache_ttl_seconds: Option<u64>,
    queue_stats_use_management_api: Option<bool>,

    #[serde(flatten, default)]
    batch: BatchConfig,
    #[serde(flatten, default)]
    ui: UIConfig,
    #[serde(default)]
    keys: KeyBindingsConfig,
    #[serde(default)]
    servicebus: ServicebusConfig,
    #[serde(default)]
    azure_ad: AzureAdConfig,
    #[serde(default)]
    logging: LoggingConfig,
    theme: Option<ThemeConfig>,
}

impl AppConfig {
    /// Validate the configuration against defined limits
    pub fn validate(&self) -> Result<(), Vec<ConfigValidationError>> {
        let mut errors = Vec::new();

        // Check batch configuration limits
        if self.batch.max_batch_size() > AZURE_SERVICE_BUS_MAX_BATCH_SIZE {
            errors.push(ConfigValidationError::BatchSize {
                configured: self.batch.max_batch_size(),
                limit: AZURE_SERVICE_BUS_MAX_BATCH_SIZE,
            });
        }

        if self.batch.operation_timeout_secs() > MAX_OPERATION_TIMEOUT_SECS {
            errors.push(ConfigValidationError::OperationTimeout {
                configured: self.batch.operation_timeout_secs(),
                limit: MAX_OPERATION_TIMEOUT_SECS,
            });
        }

        if self.batch.bulk_chunk_size() > MAX_BULK_CHUNK_SIZE {
            errors.push(ConfigValidationError::BulkChunkSize {
                configured: self.batch.bulk_chunk_size(),
                limit: MAX_BULK_CHUNK_SIZE,
            });
        }

        if self.batch.bulk_processing_time_secs() > MAX_BULK_PROCESSING_TIME_SECS {
            errors.push(ConfigValidationError::BulkProcessingTime {
                configured: self.batch.bulk_processing_time_secs(),
                limit: MAX_BULK_PROCESSING_TIME_SECS,
            });
        }

        if self.batch.lock_timeout_secs() > MAX_LOCK_TIMEOUT_SECS {
            errors.push(ConfigValidationError::LockTimeout {
                configured: self.batch.lock_timeout_secs(),
                limit: MAX_LOCK_TIMEOUT_SECS,
            });
        }

        if self.batch.max_messages_to_process() > MAX_MESSAGES_TO_PROCESS_LIMIT {
            errors.push(ConfigValidationError::MaxMessagesToProcess {
                configured: self.batch.max_messages_to_process(),
                limit: MAX_MESSAGES_TO_PROCESS_LIMIT,
            });
        }

        // Validate queue statistics cache TTL
        let ttl = self.queue_stats_cache_ttl_seconds();
        if !(MIN_QUEUE_STATS_CACHE_TTL_SECONDS..=MAX_QUEUE_STATS_CACHE_TTL_SECONDS).contains(&ttl) {
            errors.push(ConfigValidationError::QueueStatsCacheTtl {
                configured: ttl,
                min_limit: MIN_QUEUE_STATS_CACHE_TTL_SECONDS,
                max_limit: MAX_QUEUE_STATS_CACHE_TTL_SECONDS,
            });
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    // App-specific configuration accessors
    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(100)
    }

    // Backward compatibility - max_messages now refers to page_size
    pub fn max_messages(&self) -> u32 {
        self.page_size()
    }

    pub fn crossterm_input_listener_interval(&self) -> Duration {
        Duration::from_millis(self.crossterm_input_listener_interval_ms.unwrap_or(10))
    }

    pub fn crossterm_input_listener_retries(&self) -> usize {
        self.crossterm_input_listener_retries.unwrap_or(10)
    }

    pub fn poll_timeout(&self) -> Duration {
        Duration::from_millis(self.poll_timeout_ms.unwrap_or(50))
    }

    pub fn tick_interval(&self) -> Duration {
        Duration::from_millis(self.tick_interval_millis.unwrap_or(50))
    }

    // Queue statistics configuration accessors
    pub fn queue_stats_display_enabled(&self) -> bool {
        self.queue_stats_display_enabled.unwrap_or(true)
    }

    pub fn queue_stats_cache_ttl_seconds(&self) -> u64 {
        self.queue_stats_cache_ttl_seconds.unwrap_or(60)
    }

    pub fn queue_stats_use_management_api(&self) -> bool {
        self.queue_stats_use_management_api.unwrap_or(true)
    }

    // Configuration section accessors
    pub fn batch(&self) -> &BatchConfig {
        &self.batch
    }

    pub fn ui(&self) -> &UIConfig {
        &self.ui
    }

    pub fn keys(&self) -> &KeyBindingsConfig {
        &self.keys
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

    pub fn theme(&self) -> ThemeConfig {
        self.theme.clone().unwrap_or_default()
    }
}
