use super::{
    LoggingConfig, azure::ServicebusConfig, bulk_operations::DLQConfig, keys::KeyBindingsConfig,
    limits::*, ui::UIConfig, validation::ConfigValidationError,
};
use crate::theme::types::ThemeConfig;
use serde::Deserialize;
use server::bulk_operations::BatchConfig;
use server::service_bus_manager::AzureAdConfig;
use std::time::Duration;

/// Main application configuration
#[derive(Debug, Deserialize)]
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
    keys: KeyBindingsConfig,
    servicebus: ServicebusConfig,
    azure_ad: AzureAdConfig,
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

        if self.dlq.batch_size() > MAX_DLQ_BATCH_SIZE {
            errors.push(ConfigValidationError::DlqBatchSize {
                configured: self.dlq.batch_size(),
                limit: MAX_DLQ_BATCH_SIZE,
            });
        }

        if self.batch.buffer_percentage() > MAX_BUFFER_PERCENTAGE {
            errors.push(ConfigValidationError::BufferPercentage {
                configured: self.batch.buffer_percentage(),
                limit: MAX_BUFFER_PERCENTAGE,
            });
        }

        if self.batch.min_buffer_size() > MAX_MIN_BUFFER_SIZE {
            errors.push(ConfigValidationError::MinBufferSize {
                configured: self.batch.min_buffer_size(),
                limit: MAX_MIN_BUFFER_SIZE,
            });
        }

        if self.batch.bulk_operation_max_count() > BULK_OPERATION_MAX_COUNT {
            errors.push(ConfigValidationError::BulkOperationMaxCount {
                configured: self.batch.bulk_operation_max_count(),
                limit: BULK_OPERATION_MAX_COUNT,
            });
        }

        if self.batch.auto_reload_threshold() > MAX_AUTO_RELOAD_THRESHOLD {
            errors.push(ConfigValidationError::AutoReloadThreshold {
                configured: self.batch.auto_reload_threshold(),
                limit: MAX_AUTO_RELOAD_THRESHOLD,
            });
        }

        if self.batch.small_deletion_threshold() > MAX_SMALL_DELETION_THRESHOLD {
            errors.push(ConfigValidationError::SmallDeletionThreshold {
                configured: self.batch.small_deletion_threshold(),
                limit: MAX_SMALL_DELETION_THRESHOLD,
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

        if self.batch.max_messages_multiplier() > MAX_MESSAGES_MULTIPLIER {
            errors.push(ConfigValidationError::MessagesMultiplier {
                configured: self.batch.max_messages_multiplier(),
                limit: MAX_MESSAGES_MULTIPLIER,
            });
        }

        if self.batch.min_messages_to_process() < MIN_MESSAGES_TO_PROCESS_LIMIT {
            errors.push(ConfigValidationError::MinMessagesToProcess {
                configured: self.batch.min_messages_to_process(),
                limit: MIN_MESSAGES_TO_PROCESS_LIMIT,
            });
        }

        if self.batch.max_messages_to_process() > MAX_MESSAGES_TO_PROCESS_LIMIT {
            errors.push(ConfigValidationError::MaxMessagesToProcess {
                configured: self.batch.max_messages_to_process(),
                limit: MAX_MESSAGES_TO_PROCESS_LIMIT,
            });
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    // App-specific configuration accessors
    pub fn max_messages(&self) -> u32 {
        self.max_messages.unwrap_or(100)
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
