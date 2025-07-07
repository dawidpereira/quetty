use crate::common::{CacheError, HttpError};
use std::fmt;

#[derive(Debug, Clone)]
pub enum ServiceBusError {
    /// Azure API specific errors with full context
    AzureApiError {
        code: String,               // Azure error code (e.g., "SubscriptionNotFound")
        status_code: u16,           // HTTP status code
        message: String,            // Human-readable error message
        request_id: Option<String>, // Azure request ID for tracking
        operation: String,          // Operation that failed (e.g., "list_subscriptions")
    },

    /// Connection related errors
    ConnectionFailed(String),
    ConnectionLost(String),
    AuthenticationFailed(String),
    AuthenticationError(String),

    /// Consumer related errors
    ConsumerCreationFailed(String),
    ConsumerNotFound,
    ConsumerAlreadyExists(String),

    /// Producer related errors
    ProducerCreationFailed(String),
    ProducerNotFound(String),

    /// Message operation errors
    MessageReceiveFailed(String),
    MessageSendFailed(String),
    MessageCompleteFailed(String),
    MessageAbandonFailed(String),
    MessageDeadLetterFailed(String),

    /// Bulk operation errors
    BulkOperationFailed(String),
    BulkOperationPartialFailure {
        successful: usize,
        failed: usize,
        errors: Vec<String>,
    },

    /// Queue operation errors
    QueueNotFound(String),
    QueueSwitchFailed(String),
    InvalidQueueName(String),

    /// Configuration errors
    ConfigurationError(String),
    InvalidConfiguration(String),

    /// Timeout errors
    OperationTimeout(String),

    /// Generic errors
    InternalError(String),
    Unknown(String),
}

impl fmt::Display for ServiceBusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceBusError::AzureApiError {
                code,
                status_code,
                message,
                request_id,
                operation,
            } => {
                write!(
                    f,
                    "Azure API error during {operation}: {code} (HTTP {status_code}) - {message}"
                )?;
                if let Some(req_id) = request_id {
                    write!(f, " [Request ID: {req_id}]")?;
                }
                Ok(())
            }
            ServiceBusError::ConnectionFailed(msg) => write!(f, "Connection failed: {msg}"),
            ServiceBusError::ConnectionLost(msg) => write!(f, "Connection lost: {msg}"),
            ServiceBusError::AuthenticationFailed(msg) => {
                write!(f, "Authentication failed: {msg}")
            }
            ServiceBusError::AuthenticationError(msg) => {
                write!(f, "Authentication error: {msg}")
            }

            ServiceBusError::ConsumerCreationFailed(msg) => {
                write!(f, "Consumer creation failed: {msg}")
            }
            ServiceBusError::ConsumerNotFound => write!(f, "Consumer not found"),
            ServiceBusError::ConsumerAlreadyExists(queue) => {
                write!(f, "Consumer already exists for queue: {queue}")
            }

            ServiceBusError::ProducerCreationFailed(msg) => {
                write!(f, "Producer creation failed: {msg}")
            }
            ServiceBusError::ProducerNotFound(queue) => {
                write!(f, "Producer not found for queue: {queue}")
            }

            ServiceBusError::MessageReceiveFailed(msg) => {
                write!(f, "Message receive failed: {msg}")
            }
            ServiceBusError::MessageSendFailed(msg) => write!(f, "Message send failed: {msg}"),
            ServiceBusError::MessageCompleteFailed(msg) => {
                write!(f, "Message complete failed: {msg}")
            }
            ServiceBusError::MessageAbandonFailed(msg) => {
                write!(f, "Message abandon failed: {msg}")
            }
            ServiceBusError::MessageDeadLetterFailed(msg) => {
                write!(f, "Message dead letter failed: {msg}")
            }

            ServiceBusError::BulkOperationFailed(msg) => {
                write!(f, "Bulk operation failed: {msg}")
            }
            ServiceBusError::BulkOperationPartialFailure {
                successful,
                failed,
                errors,
            } => {
                write!(
                    f,
                    "Bulk operation partially failed: {} successful, {} failed. Errors: {}",
                    successful,
                    failed,
                    errors.join("; ")
                )
            }

            ServiceBusError::QueueNotFound(queue) => write!(f, "Queue not found: {queue}"),
            ServiceBusError::QueueSwitchFailed(msg) => write!(f, "Queue switch failed: {msg}"),
            ServiceBusError::InvalidQueueName(queue) => write!(f, "Invalid queue name: {queue}"),

            ServiceBusError::ConfigurationError(msg) => write!(f, "Configuration error: {msg}"),
            ServiceBusError::InvalidConfiguration(msg) => {
                write!(f, "Invalid configuration: {msg}")
            }

            ServiceBusError::OperationTimeout(msg) => write!(f, "Operation timeout: {msg}"),

            ServiceBusError::InternalError(msg) => write!(f, "Internal error: {msg}"),
            ServiceBusError::Unknown(msg) => write!(f, "Unknown error: {msg}"),
        }
    }
}

impl std::error::Error for ServiceBusError {}

impl ServiceBusError {
    /// Create an Azure API error with full context
    pub fn azure_api_error(
        operation: impl Into<String>,
        code: impl Into<String>,
        status_code: u16,
        message: impl Into<String>,
    ) -> Self {
        Self::AzureApiError {
            code: code.into(),
            status_code,
            message: message.into(),
            request_id: None,
            operation: operation.into(),
        }
    }

    /// Create an Azure API error with request ID for tracing
    pub fn azure_api_error_with_request_id(
        operation: impl Into<String>,
        code: impl Into<String>,
        status_code: u16,
        message: impl Into<String>,
        request_id: impl Into<String>,
    ) -> Self {
        Self::AzureApiError {
            code: code.into(),
            status_code,
            message: message.into(),
            request_id: Some(request_id.into()),
            operation: operation.into(),
        }
    }

    /// Extract Azure error details from a reqwest Response
    pub async fn from_azure_response(
        response: reqwest::Response,
        operation: impl Into<String>,
    ) -> Self {
        let operation = operation.into();
        let status_code = response.status().as_u16();
        let request_id = response
            .headers()
            .get("x-ms-request-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Try to extract Azure error details from response body
        match response.text().await {
            Ok(body) => {
                // Try to parse Azure error response format
                if let Ok(azure_error) = serde_json::from_str::<AzureErrorResponse>(&body) {
                    Self::AzureApiError {
                        code: azure_error.error.code,
                        status_code,
                        message: azure_error.error.message,
                        request_id,
                        operation,
                    }
                } else {
                    // Fallback for non-JSON responses
                    Self::AzureApiError {
                        code: format!("HTTP_{status_code}"),
                        status_code,
                        message: if body.is_empty() {
                            format!("HTTP {status_code} error")
                        } else {
                            body
                        },
                        request_id,
                        operation,
                    }
                }
            }
            Err(_) => Self::AzureApiError {
                code: format!("HTTP_{status_code}"),
                status_code,
                message: format!("HTTP {status_code} error - unable to read response body"),
                request_id,
                operation,
            },
        }
    }

    /// Check if this is an Azure API error
    pub fn is_azure_api_error(&self) -> bool {
        matches!(self, ServiceBusError::AzureApiError { .. })
    }

    /// Get the Azure error code if this is an Azure API error
    pub fn azure_error_code(&self) -> Option<&str> {
        match self {
            ServiceBusError::AzureApiError { code, .. } => Some(code),
            _ => None,
        }
    }

    /// Get the Azure request ID if available
    pub fn azure_request_id(&self) -> Option<&str> {
        match self {
            ServiceBusError::AzureApiError { request_id, .. } => request_id.as_deref(),
            _ => None,
        }
    }
}

/// Azure API error response format
#[derive(Debug, serde::Deserialize)]
struct AzureErrorResponse {
    error: AzureErrorDetails,
}

#[derive(Debug, serde::Deserialize)]
struct AzureErrorDetails {
    code: String,
    message: String,
}

impl From<azure_core::Error> for ServiceBusError {
    fn from(err: azure_core::Error) -> Self {
        ServiceBusError::InternalError(err.to_string())
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for ServiceBusError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        ServiceBusError::InternalError(err.to_string())
    }
}

impl From<tokio::time::error::Elapsed> for ServiceBusError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        ServiceBusError::OperationTimeout(err.to_string())
    }
}

impl From<HttpError> for ServiceBusError {
    fn from(err: HttpError) -> Self {
        match err {
            HttpError::ClientCreation { reason } => ServiceBusError::ConfigurationError(format!(
                "HTTP client creation failed: {reason}"
            )),
            HttpError::RequestFailed { url, reason } => {
                ServiceBusError::InternalError(format!("Request to {url} failed: {reason}"))
            }
            HttpError::Timeout { url, seconds } => ServiceBusError::OperationTimeout(format!(
                "Request to {url} timed out after {seconds}s"
            )),
            HttpError::RateLimited {
                retry_after_seconds,
            } => ServiceBusError::InternalError(format!(
                "Rate limited, retry after {retry_after_seconds}s"
            )),
            HttpError::InvalidResponse { expected, actual } => ServiceBusError::ConfigurationError(
                format!("Invalid response: expected {expected}, got {actual}"),
            ),
        }
    }
}

impl From<CacheError> for ServiceBusError {
    fn from(err: CacheError) -> Self {
        match err {
            CacheError::Expired { key } => {
                ServiceBusError::InternalError(format!("Cache entry expired: {key}"))
            }
            CacheError::Miss { key } => {
                ServiceBusError::InternalError(format!("Cache miss: {key}"))
            }
            CacheError::Full { key } => {
                ServiceBusError::InternalError(format!("Cache full, cannot add: {key}"))
            }
            CacheError::OperationFailed { reason } => {
                ServiceBusError::InternalError(format!("Cache operation failed: {reason}"))
            }
        }
    }
}

// Result type alias for convenience
pub type ServiceBusResult<T> = Result<T, ServiceBusError>;
