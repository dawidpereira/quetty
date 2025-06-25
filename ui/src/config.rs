use crate::error::{AppError, AppResult};
use crate::theme::types::ThemeConfig;
use config::{Config, Environment, File};

use serde::Deserialize;
use server::bulk_operations::BatchConfig;
use server::service_bus_manager::AzureAdConfig;
use std::time::Duration;

/// Global hard limits for Azure Service Bus operations
pub mod limits {
    /// Azure Service Bus hard limit for batch operations
    pub const AZURE_SERVICE_BUS_MAX_BATCH_SIZE: u32 = 2048;

    /// Maximum reasonable timeout for operations (10 minutes)
    pub const MAX_OPERATION_TIMEOUT_SECS: u64 = 600;

    /// Maximum reasonable DLQ batch size
    pub const MAX_DLQ_BATCH_SIZE: u32 = 100;

    /// Maximum reasonable buffer percentage (50% = 0.5)
    pub const MAX_BUFFER_PERCENTAGE: f64 = 0.5;

    /// Maximum reasonable minimum buffer size
    pub const MAX_MIN_BUFFER_SIZE: usize = 500;

    /// Bulk operation limits (min count is now handled by server's BatchConfig)
    pub const BULK_OPERATION_MAX_COUNT: usize = 1000;

    /// Maximum threshold for triggering auto-reload after bulk operations
    pub const MAX_AUTO_RELOAD_THRESHOLD: usize = 100;

    /// Maximum small deletion threshold for backfill operations
    pub const MAX_SMALL_DELETION_THRESHOLD: usize = 20;

    /// Maximum chunk size for bulk processing
    pub const MAX_BULK_CHUNK_SIZE: usize = 500;

    /// Maximum processing time for bulk operations (seconds)
    pub const MAX_BULK_PROCESSING_TIME_SECS: u64 = 120;

    /// Maximum lock timeout for lock operations
    pub const MAX_LOCK_TIMEOUT_SECS: u64 = 30;

    /// Maximum multiplier for calculating max messages to process
    pub const MAX_MESSAGES_MULTIPLIER: usize = 10;

    /// Minimum messages to process in bulk operations
    pub const MIN_MESSAGES_TO_PROCESS_LIMIT: usize = 10;

    /// Maximum messages to process in bulk operations
    pub const MAX_MESSAGES_TO_PROCESS_LIMIT: usize = 5000;
}

/// Configuration validation errors
#[derive(Debug)]
pub enum ConfigValidationError {
    BatchSize { configured: u32, limit: u32 },
    OperationTimeout { configured: u64, limit: u64 },
    DlqBatchSize { configured: u32, limit: u32 },
    BufferPercentage { configured: f64, limit: f64 },
    MinBufferSize { configured: usize, limit: usize },
    BulkOperationMaxCount { configured: usize, limit: usize },
    AutoReloadThreshold { configured: usize, limit: usize },
    SmallDeletionThreshold { configured: usize, limit: usize },
    BulkChunkSize { configured: usize, limit: usize },
    BulkProcessingTime { configured: u64, limit: u64 },
    LockTimeout { configured: u64, limit: u64 },
    MessagesMultiplier { configured: usize, limit: usize },
    MinMessagesToProcess { configured: usize, limit: usize },
    MaxMessagesToProcess { configured: usize, limit: usize },
}

impl ConfigValidationError {
    pub fn user_message(&self) -> String {
        match self {
            ConfigValidationError::BatchSize { configured, limit } => {
                format!(
                    "Bulk batch size configuration error!\n\n\
                    Your configured value: {}\n\
                    Azure Service Bus limit: {}\n\n\
                    Please update max_batch_size in config.toml to {} or less.",
                    configured, limit, limit
                )
            }
            ConfigValidationError::OperationTimeout { configured, limit } => {
                format!(
                    "Operation timeout too high!\n\n\
                    Your configured value: {} seconds\n\
                    Recommended maximum: {} seconds\n\n\
                    Please update operation_timeout_secs in config.toml.",
                    configured, limit
                )
            }
            ConfigValidationError::DlqBatchSize { configured, limit } => {
                format!(
                    "DLQ batch size too high!\n\n\
                    Your configured value: {}\n\
                    Recommended maximum: {}\n\n\
                    Please update dlq_batch_size in config.toml.",
                    configured, limit
                )
            }
            ConfigValidationError::BufferPercentage { configured, limit } => {
                format!(
                    "Buffer percentage too high!\n\n\
                    Your configured value: {:.1}%\n\
                    Recommended maximum: {:.1}%\n\n\
                    Please update buffer_percentage in config.toml.",
                    configured * 100.0,
                    limit * 100.0
                )
            }
            ConfigValidationError::MinBufferSize { configured, limit } => {
                format!(
                    "Minimum buffer size too high!\n\n\
                    Your configured value: {}\n\
                    Recommended maximum: {}\n\n\
                    Please update min_buffer_size in config.toml.",
                    configured, limit
                )
            }
            ConfigValidationError::BulkOperationMaxCount { configured, limit } => {
                format!(
                    "Bulk operation limit too high!\n\n\
                    Your configured value: {}\n\
                    Hard limit: {}\n\n\
                    Please update bulk_operation_max_count in config.toml to {} or less.",
                    configured, limit, limit
                )
            }
            ConfigValidationError::AutoReloadThreshold { configured, limit } => {
                format!(
                    "Auto-reload threshold too high!\n\n\
                    Your configured value: {}\n\
                    Recommended maximum: {}\n\n\
                    Please update auto_reload_threshold in config.toml.",
                    configured, limit
                )
            }
            ConfigValidationError::SmallDeletionThreshold { configured, limit } => {
                format!(
                    "Small deletion threshold too high!\n\n\
                    Your configured value: {}\n\
                    Recommended maximum: {}\n\n\
                    Please update small_deletion_threshold in config.toml.",
                    configured, limit
                )
            }
            ConfigValidationError::BulkChunkSize { configured, limit } => {
                format!(
                    "Bulk chunk size too high!\n\n\
                    Your configured value: {}\n\
                    Recommended maximum: {}\n\n\
                    Please update bulk_chunk_size in config.toml.",
                    configured, limit
                )
            }
            ConfigValidationError::BulkProcessingTime { configured, limit } => {
                format!(
                    "Bulk processing time too high!\n\n\
                    Your configured value: {} seconds\n\
                    Recommended maximum: {} seconds\n\n\
                    Please update bulk_processing_time_secs in config.toml.",
                    configured, limit
                )
            }
            ConfigValidationError::LockTimeout { configured, limit } => {
                format!(
                    "Lock timeout too high!\n\n\
                    Your configured value: {} seconds\n\
                    Recommended maximum: {} seconds\n\n\
                    Please update lock_timeout_secs in config.toml.",
                    configured, limit
                )
            }
            ConfigValidationError::MessagesMultiplier { configured, limit } => {
                format!(
                    "Messages multiplier too high!\n\n\
                    Your configured value: {}\n\
                    Recommended maximum: {}\n\n\
                    Please update max_messages_multiplier in config.toml.",
                    configured, limit
                )
            }
            ConfigValidationError::MinMessagesToProcess { configured, limit } => {
                format!(
                    "Minimum messages to process too low!\n\n\
                    Your configured value: {}\n\
                    Recommended minimum: {}\n\n\
                    Please update min_messages_to_process in config.toml.",
                    configured, limit
                )
            }
            ConfigValidationError::MaxMessagesToProcess { configured, limit } => {
                format!(
                    "Maximum messages to process too high!\n\n\
                    Your configured value: {}\n\
                    Recommended maximum: {}\n\n\
                    Please update max_messages_to_process in config.toml.",
                    configured, limit
                )
            }
        }
    }
}

use std::sync::OnceLock;

/// Configuration loading result for better error handling
pub enum ConfigLoadResult {
    Success(Box<AppConfig>),
    LoadError(String),
    DeserializeError(String),
}

/// Safe configuration loading function
fn load_config() -> ConfigLoadResult {
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
            log::error!("Failed to load configuration: {}", e);
            return ConfigLoadResult::LoadError(format!(
                "Configuration loading failed: {}. Please check your config.toml file and environment variables.",
                e
            ));
        }
    };

    match config.try_deserialize::<AppConfig>() {
        Ok(app_config) => ConfigLoadResult::Success(Box::new(app_config)),
        Err(e) => {
            log::error!("Failed to deserialize configuration: {}", e);
            ConfigLoadResult::DeserializeError(format!(
                "Configuration format error: {}. Please check your config.toml syntax.",
                e
            ))
        }
    }
}

static CONFIG_CELL: OnceLock<ConfigLoadResult> = OnceLock::new();

/// Get the global configuration, loading it if necessary
pub fn get_config() -> &'static ConfigLoadResult {
    CONFIG_CELL.get_or_init(load_config)
}

/// Get the configuration, panicking if loading failed
/// Used by components that can't handle config errors gracefully
pub fn get_config_or_panic() -> &'static AppConfig {
    match get_config() {
        ConfigLoadResult::Success(config) => config.as_ref(),
        ConfigLoadResult::LoadError(error) => {
            panic!("Configuration loading failed: {}", error);
        }
        ConfigLoadResult::DeserializeError(error) => {
            panic!("Configuration parsing failed: {}", error);
        }
    }
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
    keys: KeyBindingsConfig,
    servicebus: ServicebusConfig,
    azure_ad: AzureAdConfig,
    logging: LoggingConfig,
    theme: Option<ThemeConfig>,
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
    /// Batch size for receiving messages in DLQ operations (default: 10)
    dlq_batch_size: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServicebusConfig {
    connection_string: Option<String>,
}

/// Configuration for key bindings
#[derive(Debug, Clone, Deserialize)]
pub struct KeyBindingsConfig {
    // Global keys
    key_quit: Option<char>,
    key_help: Option<char>,
    key_theme: Option<char>,

    // Navigation keys
    key_down: Option<char>,
    key_up: Option<char>,
    key_next_page: Option<char>,
    key_prev_page: Option<char>,
    key_alt_next_page: Option<char>,
    key_alt_prev_page: Option<char>,

    // Message actions
    key_send_to_dlq: Option<char>,
    key_resend_from_dlq: Option<char>,
    key_resend_and_delete_from_dlq: Option<char>,
    key_delete_message: Option<char>,
    key_alt_delete_message: Option<char>,

    // Message details actions
    key_copy_message: Option<char>,
    key_yank_message: Option<char>,
    key_send_edited_message: Option<char>,
    key_replace_edited_message: Option<char>,

    // Bulk selection keys
    key_toggle_selection: Option<char>,
    key_select_all_page: Option<char>,

    // Queue/Namespace selection
    key_queue_select: Option<char>,
    key_namespace_select: Option<char>,

    // Message composition keys
    key_toggle_dlq: Option<char>,
    key_compose_multiple: Option<char>,
    key_compose_single: Option<char>,

    // Confirmation keys
    key_confirm_yes: Option<char>,
    key_confirm_no: Option<char>,
}

impl AppConfig {
    /// Validate all configuration values against global limits
    pub fn validate(&self) -> Result<(), Vec<ConfigValidationError>> {
        let mut errors = Vec::new();

        // Validate batch configuration
        let batch_config = self.batch();

        // Debug logging to see what values we're actually getting
        log::debug!("Validating configuration:");
        log::debug!(
            "  max_batch_size: {} (limit: {})",
            batch_config.max_batch_size(),
            limits::AZURE_SERVICE_BUS_MAX_BATCH_SIZE
        );
        log::debug!(
            "  operation_timeout_secs: {} (limit: {})",
            batch_config.operation_timeout_secs(),
            limits::MAX_OPERATION_TIMEOUT_SECS
        );
        log::debug!(
            "  buffer_percentage: {} (limit: {})",
            batch_config.buffer_percentage(),
            limits::MAX_BUFFER_PERCENTAGE
        );
        log::debug!(
            "  min_buffer_size: {} (limit: {})",
            batch_config.min_buffer_size(),
            limits::MAX_MIN_BUFFER_SIZE
        );

        if batch_config.max_batch_size() > limits::AZURE_SERVICE_BUS_MAX_BATCH_SIZE {
            log::error!(
                "VALIDATION FAILED: max_batch_size {} > limit {}",
                batch_config.max_batch_size(),
                limits::AZURE_SERVICE_BUS_MAX_BATCH_SIZE
            );
            errors.push(ConfigValidationError::BatchSize {
                configured: batch_config.max_batch_size(),
                limit: limits::AZURE_SERVICE_BUS_MAX_BATCH_SIZE,
            });
        }

        if batch_config.operation_timeout_secs() > limits::MAX_OPERATION_TIMEOUT_SECS {
            errors.push(ConfigValidationError::OperationTimeout {
                configured: batch_config.operation_timeout_secs(),
                limit: limits::MAX_OPERATION_TIMEOUT_SECS,
            });
        }

        if batch_config.buffer_percentage() > limits::MAX_BUFFER_PERCENTAGE {
            errors.push(ConfigValidationError::BufferPercentage {
                configured: batch_config.buffer_percentage(),
                limit: limits::MAX_BUFFER_PERCENTAGE,
            });
        }

        if batch_config.min_buffer_size() > limits::MAX_MIN_BUFFER_SIZE {
            errors.push(ConfigValidationError::MinBufferSize {
                configured: batch_config.min_buffer_size(),
                limit: limits::MAX_MIN_BUFFER_SIZE,
            });
        }

        // Validate DLQ configuration
        log::debug!(
            "  dlq_batch_size: {} (limit: {})",
            self.dlq().batch_size(),
            limits::MAX_DLQ_BATCH_SIZE
        );
        if self.dlq().batch_size() > limits::MAX_DLQ_BATCH_SIZE {
            errors.push(ConfigValidationError::DlqBatchSize {
                configured: self.dlq().batch_size(),
                limit: limits::MAX_DLQ_BATCH_SIZE,
            });
        }

        // Validate batch configuration (includes bulk operations)
        log::debug!(
            "  bulk_operation_max_count: {} (limit: {})",
            batch_config.bulk_operation_max_count(),
            limits::BULK_OPERATION_MAX_COUNT
        );
        if batch_config.bulk_operation_max_count() > limits::BULK_OPERATION_MAX_COUNT {
            errors.push(ConfigValidationError::BulkOperationMaxCount {
                configured: batch_config.bulk_operation_max_count(),
                limit: limits::BULK_OPERATION_MAX_COUNT,
            });
        }

        log::debug!(
            "  auto_reload_threshold: {} (limit: {})",
            batch_config.auto_reload_threshold(),
            limits::MAX_AUTO_RELOAD_THRESHOLD
        );
        if batch_config.auto_reload_threshold() > limits::MAX_AUTO_RELOAD_THRESHOLD {
            errors.push(ConfigValidationError::AutoReloadThreshold {
                configured: batch_config.auto_reload_threshold(),
                limit: limits::MAX_AUTO_RELOAD_THRESHOLD,
            });
        }

        log::debug!(
            "  small_deletion_threshold: {} (limit: {})",
            batch_config.small_deletion_threshold(),
            limits::MAX_SMALL_DELETION_THRESHOLD
        );
        if batch_config.small_deletion_threshold() > limits::MAX_SMALL_DELETION_THRESHOLD {
            errors.push(ConfigValidationError::SmallDeletionThreshold {
                configured: batch_config.small_deletion_threshold(),
                limit: limits::MAX_SMALL_DELETION_THRESHOLD,
            });
        }

        log::debug!(
            "  bulk_chunk_size: {} (limit: {})",
            batch_config.bulk_chunk_size(),
            limits::MAX_BULK_CHUNK_SIZE
        );
        if batch_config.bulk_chunk_size() > limits::MAX_BULK_CHUNK_SIZE {
            errors.push(ConfigValidationError::BulkChunkSize {
                configured: batch_config.bulk_chunk_size(),
                limit: limits::MAX_BULK_CHUNK_SIZE,
            });
        }

        log::debug!(
            "  bulk_processing_time_secs: {} (limit: {})",
            batch_config.bulk_processing_time_secs(),
            limits::MAX_BULK_PROCESSING_TIME_SECS
        );
        if batch_config.bulk_processing_time_secs() > limits::MAX_BULK_PROCESSING_TIME_SECS {
            errors.push(ConfigValidationError::BulkProcessingTime {
                configured: batch_config.bulk_processing_time_secs(),
                limit: limits::MAX_BULK_PROCESSING_TIME_SECS,
            });
        }

        log::debug!(
            "  lock_timeout_secs: {} (limit: {})",
            batch_config.lock_timeout_secs(),
            limits::MAX_LOCK_TIMEOUT_SECS
        );
        if batch_config.lock_timeout_secs() > limits::MAX_LOCK_TIMEOUT_SECS {
            errors.push(ConfigValidationError::LockTimeout {
                configured: batch_config.lock_timeout_secs(),
                limit: limits::MAX_LOCK_TIMEOUT_SECS,
            });
        }

        log::debug!(
            "  max_messages_multiplier: {} (limit: {})",
            batch_config.max_messages_multiplier(),
            limits::MAX_MESSAGES_MULTIPLIER
        );
        if batch_config.max_messages_multiplier() > limits::MAX_MESSAGES_MULTIPLIER {
            errors.push(ConfigValidationError::MessagesMultiplier {
                configured: batch_config.max_messages_multiplier(),
                limit: limits::MAX_MESSAGES_MULTIPLIER,
            });
        }

        log::debug!(
            "  min_messages_to_process: {} (limit: {})",
            batch_config.min_messages_to_process(),
            limits::MIN_MESSAGES_TO_PROCESS_LIMIT
        );
        if batch_config.min_messages_to_process() < limits::MIN_MESSAGES_TO_PROCESS_LIMIT {
            errors.push(ConfigValidationError::MinMessagesToProcess {
                configured: batch_config.min_messages_to_process(),
                limit: limits::MIN_MESSAGES_TO_PROCESS_LIMIT,
            });
        }

        log::debug!(
            "  max_messages_to_process: {} (limit: {})",
            batch_config.max_messages_to_process(),
            limits::MAX_MESSAGES_TO_PROCESS_LIMIT
        );
        if batch_config.max_messages_to_process() > limits::MAX_MESSAGES_TO_PROCESS_LIMIT {
            errors.push(ConfigValidationError::MaxMessagesToProcess {
                configured: batch_config.max_messages_to_process(),
                limit: limits::MAX_MESSAGES_TO_PROCESS_LIMIT,
            });
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

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

impl ServicebusConfig {
    pub fn connection_string(&self) -> AppResult<&str> {
        self.connection_string.as_deref()
            .ok_or_else(|| AppError::Config(
                "SERVICEBUS_CONNECTION_STRING is required but not found in configuration or environment variables. Please set this value in .env file or environment.".to_string()
            ))
    }
}

impl DLQConfig {
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

impl KeyBindingsConfig {
    // Global keys
    pub fn quit(&self) -> char {
        self.key_quit.unwrap_or('q')
    }
    pub fn help(&self) -> char {
        self.key_help.unwrap_or('h')
    }
    pub fn theme(&self) -> char {
        self.key_theme.unwrap_or('t')
    }

    // Navigation keys
    pub fn down(&self) -> char {
        self.key_down.unwrap_or('j')
    }
    pub fn up(&self) -> char {
        self.key_up.unwrap_or('k')
    }
    pub fn next_page(&self) -> char {
        self.key_next_page.unwrap_or('n')
    }
    pub fn prev_page(&self) -> char {
        self.key_prev_page.unwrap_or('p')
    }
    pub fn alt_next_page(&self) -> char {
        self.key_alt_next_page.unwrap_or(']')
    }
    pub fn alt_prev_page(&self) -> char {
        self.key_alt_prev_page.unwrap_or('[')
    }

    // Message actions
    pub fn send_to_dlq(&self) -> char {
        self.key_send_to_dlq.unwrap_or('s')
    }
    pub fn resend_from_dlq(&self) -> char {
        self.key_resend_from_dlq.unwrap_or('s')
    }
    pub fn resend_and_delete_from_dlq(&self) -> char {
        self.key_resend_and_delete_from_dlq.unwrap_or('S')
    }
    pub fn delete_message(&self) -> char {
        self.key_delete_message.unwrap_or('X')
    }
    pub fn alt_delete_message(&self) -> char {
        self.key_alt_delete_message.unwrap_or('X')
    }

    // Message details actions
    pub fn copy_message(&self) -> char {
        self.key_copy_message.unwrap_or('c')
    }
    pub fn yank_message(&self) -> char {
        self.key_yank_message.unwrap_or('y')
    }
    pub fn send_edited_message(&self) -> char {
        self.key_send_edited_message.unwrap_or('s') // 's' key
    }
    pub fn replace_edited_message(&self) -> char {
        self.key_replace_edited_message.unwrap_or('s')
    }

    // Bulk selection keys
    pub fn toggle_selection(&self) -> char {
        self.key_toggle_selection.unwrap_or(' ')
    }
    pub fn select_all_page(&self) -> char {
        self.key_select_all_page.unwrap_or('a')
    }

    // Queue/Namespace selection
    pub fn queue_select(&self) -> char {
        self.key_queue_select.unwrap_or('o')
    }
    pub fn namespace_select(&self) -> char {
        self.key_namespace_select.unwrap_or('o')
    }

    // Confirmation keys
    // Message composition keys
    pub fn toggle_dlq(&self) -> char {
        self.key_toggle_dlq.unwrap_or('d')
    }
    pub fn compose_multiple(&self) -> char {
        self.key_compose_multiple.unwrap_or('m')
    }
    pub fn compose_single(&self) -> char {
        self.key_compose_single.unwrap_or('n') // Note: This will be used with Ctrl modifier
    }

    pub fn confirm_yes(&self) -> char {
        self.key_confirm_yes.unwrap_or('y')
    }
    pub fn confirm_no(&self) -> char {
        self.key_confirm_no.unwrap_or('n')
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
