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
    #[error("Invalid authentication method: {method}")]
    InvalidAuthMethod { method: String },
    #[error("Missing required Azure AD configuration: {field}")]
    MissingAzureAdField { field: String },
    #[error("Invalid Azure AD flow: {flow}")]
    InvalidAzureAdFlow { flow: String },
    #[error("Conflicting authentication configuration: {message}")]
    ConflictingAuthConfig { message: String },
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
            ConfigValidationError::InvalidAuthMethod { method } => {
                format!(
                    "Invalid authentication method!\n\n\
                    Your configured value: {method}\n\
                    Valid methods: connection_string, device_code, client_secret\n\n\
                    Please update azure_ad.auth_method in config.toml."
                )
            }
            ConfigValidationError::MissingAzureAdField { field } => {
                format!(
                    "Missing required Azure AD configuration!\n\n\
                    Missing field: {field}\n\n\
                    When using azure_ad authentication with device_code flow, you must provide:\n\
                    - azure_ad.tenant_id (or AZURE_AD__TENANT_ID env var)\n\
                    - azure_ad.client_id (or AZURE_AD__CLIENT_ID env var)\n\n\
                    Other fields (subscription_id, resource_group, namespace) are optional and can be selected interactively after authentication."
                )
            }
            ConfigValidationError::InvalidAzureAdFlow { flow } => {
                format!(
                    "Invalid Azure AD authentication flow!\n\n\
                    Your configured value: {flow}\n\
                    Valid flow: device_code\n\n\
                    Please update azure_ad.auth_method in config.toml."
                )
            }
            ConfigValidationError::ConflictingAuthConfig { message } => {
                format!("Conflicting authentication configuration!\n\n{message}")
            }
        }
    }
}

/// Configuration loading result
#[derive(Clone)]
pub enum ConfigLoadResult {
    Success(Box<AppConfig>),
    LoadError(String),
    DeserializeError(String),
}
