use super::app::AppConfig;

/// Configuration validation errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigValidationError {
    #[error("Invalid page_size: {configured} (min: {min_limit}, max: {max_limit})")]
    PageSize {
        configured: u32,
        min_limit: u32,
        max_limit: u32,
    },
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
            ConfigValidationError::PageSize {
                configured,
                min_limit,
                max_limit,
            } => {
                format!(
                    "Page size out of range!\n\n\
                    Your configured value: {configured}\n\
                    Valid range: {min_limit} - {max_limit}\n\n\
                    Please update page_size in config.toml to a value between {min_limit} and {max_limit}."
                )
            }
            ConfigValidationError::MaxMessagesToProcess { configured, limit } => {
                format!(
                    "Maximum messages to process too high!\n\n\n                    Your configured value: {configured}\n\n\n                    Recommended maximum: {limit}\n\n\n                    Please update max_messages_to_process in config.toml."
                )
            }
            ConfigValidationError::BulkChunkSize { configured, limit } => {
                format!(
                    "Bulk chunk size too high!\n\n\n                    Your configured value: {configured}\n\n\n                    Recommended maximum: {limit}\n\n\n                    Please update bulk_chunk_size in config.toml."
                )
            }
            ConfigValidationError::BatchSize { configured, limit } => {
                format!(
                    "Bulk batch size configuration error!\n\n\n                    Your configured value: {configured}\n\n                    Azure Service Bus limit: {limit}\n\n\n                    Please update max_batch_size in config.toml to {limit} or less."
                )
            }
            ConfigValidationError::OperationTimeout { configured, limit } => {
                format!(
                    "Operation timeout too high!\n\n\n                    Your configured value: {configured} seconds\n\n\n                    Recommended maximum: {limit} seconds\n\n\n                    Please update operation_timeout_secs in config.toml."
                )
            }
            ConfigValidationError::BulkProcessingTime { configured, limit } => {
                format!(
                    "Bulk processing time too high!\n\n\n                    Your configured value: {configured} seconds\n\n                    Recommended maximum: {limit} seconds\n\n\n                    Please update bulk_processing_time_secs in config.toml."
                )
            }
            ConfigValidationError::LockTimeout { configured, limit } => {
                format!(
                    "Lock timeout too high!\n\n\n                    Your configured value: {configured} seconds\n\n                    Recommended maximum: {limit} seconds\n\n\n                    Please update lock_timeout_secs in config.toml."
                )
            }
            ConfigValidationError::QueueStatsCacheTtl {
                configured,
                min_limit,
                max_limit,
            } => {
                format!(
                    "Queue statistics cache TTL out of range!\n\nYour configured value: {configured} seconds\nValid range: {min_limit} - {max_limit} seconds\n\nPlease update queue_stats_cache_ttl_seconds in config.toml to a value between {min_limit} and {max_limit} seconds."
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
