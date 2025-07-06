use thiserror::Error;

/// Common HTTP-related errors with structured information
#[derive(Debug, Error)]
pub enum HttpError {
    #[error("HTTP client creation failed: {reason}")]
    ClientCreation { reason: String },

    #[error("Request failed: {url} - {reason}")]
    RequestFailed { url: String, reason: String },

    #[error("Request timeout after {seconds}s: {url}")]
    Timeout { url: String, seconds: u64 },

    #[error("Rate limit exceeded: retry after {retry_after_seconds}s")]
    RateLimited { retry_after_seconds: u64 },

    #[error("Invalid response: expected {expected}, got {actual}")]
    InvalidResponse { expected: String, actual: String },
}

/// Cache-related errors
#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Cache entry expired for key: {key}")]
    Expired { key: String },

    #[error("Cache miss for key: {key}")]
    Miss { key: String },

    #[error("Cache full, unable to add entry for key: {key}")]
    Full { key: String },

    #[error("Cache operation failed: {reason}")]
    OperationFailed { reason: String },
}

/// Helper trait for adding context to errors
pub trait ErrorContext<T> {
    /// Add context to an error result
    fn context(self, msg: &str) -> Result<T, String>;

    /// Add lazy context to an error result
    fn with_context<F>(self, f: F) -> Result<T, String>
    where
        F: FnOnce() -> String;
}

impl<T, E> ErrorContext<T> for Result<T, E>
where
    E: std::fmt::Display,
{
    fn context(self, msg: &str) -> Result<T, String> {
        self.map_err(|e| format!("{msg}: {e}"))
    }

    fn with_context<F>(self, f: F) -> Result<T, String>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| format!("{}: {e}", f()))
    }
}
