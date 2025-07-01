use super::app::AppConfig;

/// Configuration validation errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigValidationError {
    #[error("Invalid batch_size: {configured} (limit: {limit})")]
    BatchSize { configured: u32, limit: u32 },
    #[error("Invalid operation_timeout_secs: {configured} (limit: {limit})")]
    OperationTimeout { configured: u64, limit: u64 },
    #[error("Invalid bulk_chunk_size: {configured} (limit: {limit})")]
    BulkChunkSize { configured: usize, limit: usize },
    #[error("Invalid bulk_processing_time_secs: {configured} (limit: {limit})")]
    BulkProcessingTime { configured: u64, limit: u64 },
    #[error("Invalid lock_timeout_secs: {configured} (limit: {limit})")]
    LockTimeout { configured: u64, limit: u64 },
    #[error("Invalid max_messages_to_process: {configured} (limit: {limit})")]
    MaxMessagesToProcess { configured: usize, limit: usize },
    #[error(
        "Invalid queue_stats_cache_ttl_seconds: {configured} (min: {min_limit}, max: {max_limit})"
    )]
    QueueStatsCacheTtl {
        configured: u64,
        min_limit: u64,
        max_limit: u64,
    },
}

impl ConfigValidationError {
    pub fn user_message(&self) -> String {
        match self {
            ConfigValidationError::MaxMessagesToProcess { configured, limit } => {
                format!(
                    "Maximum messages to process too high!\n\n\
                    Your configured value: {}\n\
                    Recommended maximum: {}\n\n\
                    Please update max_messages_to_process in config.toml.",
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
