use super::app::AppConfig;

/// Configuration validation errors
#[derive(Debug)]
pub enum ConfigValidationError {
    BatchSize {
        configured: u32,
        limit: u32,
    },
    OperationTimeout {
        configured: u64,
        limit: u64,
    },
    DlqBatchSize {
        configured: u32,
        limit: u32,
    },
    BufferPercentage {
        configured: f64,
        limit: f64,
    },
    MinBufferSize {
        configured: usize,
        limit: usize,
    },
    BulkOperationMaxCount {
        configured: usize,
        limit: usize,
    },
    AutoReloadThreshold {
        configured: usize,
        limit: usize,
    },
    SmallDeletionThreshold {
        configured: usize,
        limit: usize,
    },
    BulkChunkSize {
        configured: usize,
        limit: usize,
    },
    BulkProcessingTime {
        configured: u64,
        limit: u64,
    },
    LockTimeout {
        configured: u64,
        limit: u64,
    },
    MessagesMultiplier {
        configured: usize,
        limit: usize,
    },
    MinMessagesToProcess {
        configured: usize,
        limit: usize,
    },
    MaxMessagesToProcess {
        configured: usize,
        limit: usize,
    },
    QueueStatsCacheTtl {
        configured: u64,
        min_limit: u64,
        max_limit: u64,
    },
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
            ConfigValidationError::QueueStatsCacheTtl {
                configured,
                min_limit,
                max_limit,
            } => {
                format!(
                    "Queue statistics cache TTL out of range!\n\n\
                    Your configured value: {} seconds\n\
                    Valid range: {} - {} seconds\n\n\
                    Please update queue_stats_cache_ttl_seconds in config.toml to a value between {} and {} seconds.",
                    configured, min_limit, max_limit, min_limit, max_limit
                )
            }
        }
    }
}

/// Configuration loading result
pub enum ConfigLoadResult {
    Success(Box<AppConfig>),
    LoadError(String),
    DeserializeError(String),
}
