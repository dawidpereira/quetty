//! Error types for Azure Service Bus operations.
//!
//! This module provides comprehensive error handling for all Service Bus operations,
//! including Azure API errors, connection issues, message operations, and bulk operations.
//! The error types are designed to provide detailed context for debugging and user feedback.

use crate::common::{CacheError, HttpError};
use std::fmt;

/// Comprehensive error type for all Service Bus operations.
///
/// Provides detailed error information with appropriate context for debugging
/// and user feedback. Includes specialized variants for different operation types
/// and Azure API integration.
///
/// # Error Categories
///
/// - **Azure API Errors** - Detailed Azure service errors with request tracking
/// - **Connection Errors** - Authentication and connection issues
/// - **Consumer/Producer Errors** - Client creation and management errors
/// - **Message Operation Errors** - Message handling failures
/// - **Bulk Operation Errors** - Batch operation failures with partial success tracking
/// - **Queue Errors** - Queue management and navigation errors
/// - **Configuration Errors** - Invalid configuration or setup issues
///
/// # Examples
///
/// ```no_run
/// use server::service_bus_manager::{ServiceBusError, ServiceBusResult};
///
/// fn handle_error(error: ServiceBusError) {
///     match error {
///         ServiceBusError::QueueNotFound(queue) => {
///             eprintln!("Queue '{}' does not exist", queue);
///         }
///         ServiceBusError::AzureApiError { code, message, .. } => {
///             eprintln!("Azure API error {}: {}", code, message);
///         }
///         _ => eprintln!("Service Bus error: {}", error),
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub enum ServiceBusError {
    /// Azure API specific errors with full context for debugging and support.
    ///
    /// Contains detailed information about Azure service errors including
    /// error codes, HTTP status, and request tracking information.
    AzureApiError {
        /// Azure error code (e.g., "SubscriptionNotFound", "Unauthorized")
        code: String,
        /// HTTP status code from the API response
        status_code: u16,
        /// Human-readable error message from Azure
        message: String,
        /// Azure request ID for tracking and support
        request_id: Option<String>,
        /// Operation that failed (e.g., "list_subscriptions", "send_message")
        operation: String,
    },

    /// Connection establishment failed
    ConnectionFailed(String),
    /// Existing connection was lost during operation
    ConnectionLost(String),
    /// Authentication process failed
    AuthenticationFailed(String),
    /// Authentication configuration or credential error
    AuthenticationError(String),

    /// Message consumer creation failed
    ConsumerCreationFailed(String),
    /// No consumer found for the current context
    ConsumerNotFound,
    /// Consumer already exists for the specified queue
    ConsumerAlreadyExists(String),

    /// Message producer creation failed
    ProducerCreationFailed(String),
    /// No producer found for the specified queue
    ProducerNotFound(String),

    /// Message receive operation failed
    MessageReceiveFailed(String),
    /// Message send operation failed
    MessageSendFailed(String),
    /// Message completion failed
    MessageCompleteFailed(String),
    /// Message abandon operation failed
    MessageAbandonFailed(String),
    /// Message dead letter operation failed
    MessageDeadLetterFailed(String),

    /// Bulk operation failed completely
    BulkOperationFailed(String),
    /// Bulk operation partially failed with detailed results
    BulkOperationPartialFailure {
        /// Number of successful operations
        successful: usize,
        /// Number of failed operations
        failed: usize,
        /// Detailed error messages for failed operations
        errors: Vec<String>,
    },

    /// Specified queue does not exist
    QueueNotFound(String),
    /// Failed to switch to the specified queue
    QueueSwitchFailed(String),
    /// Queue name format is invalid
    InvalidQueueName(String),

    /// Configuration value is missing or invalid
    ConfigurationError(String),
    /// Configuration format or structure is invalid
    InvalidConfiguration(String),

    /// Operation exceeded timeout limit
    OperationTimeout(String),

    /// Internal service error
    InternalError(String),
    /// Unknown or unexpected error
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
    /// Creates an Azure API error with full context.
    ///
    /// # Arguments
    ///
    /// * `operation` - The operation that failed (e.g., "list_queues")
    /// * `code` - Azure error code (e.g., "Unauthorized")
    /// * `status_code` - HTTP status code from the response
    /// * `message` - Human-readable error message
    ///
    /// # Returns
    ///
    /// A new [`ServiceBusError::AzureApiError`] instance
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

    /// Creates an Azure API error with request ID for tracing.
    ///
    /// # Arguments
    ///
    /// * `operation` - The operation that failed
    /// * `code` - Azure error code
    /// * `status_code` - HTTP status code
    /// * `message` - Error message
    /// * `request_id` - Azure request ID for support tracking
    ///
    /// # Returns
    ///
    /// A new [`ServiceBusError::AzureApiError`] with request ID
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

    /// Extracts Azure error details from a reqwest Response.
    ///
    /// Parses Azure API error responses and extracts structured error information
    /// including request IDs for tracking. Handles both JSON and plain text responses.
    ///
    /// # Arguments
    ///
    /// * `response` - The HTTP response from Azure API
    /// * `operation` - The operation that resulted in this response
    ///
    /// # Returns
    ///
    /// A [`ServiceBusError::AzureApiError`] with extracted details
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

    /// Checks if this is an Azure API error.
    ///
    /// # Returns
    ///
    /// `true` if this is an [`AzureApiError`], `false` otherwise
    pub fn is_azure_api_error(&self) -> bool {
        matches!(self, ServiceBusError::AzureApiError { .. })
    }

    /// Gets the Azure error code if this is an Azure API error.
    ///
    /// # Returns
    ///
    /// The Azure error code as a string slice, or `None` if not an Azure API error
    pub fn azure_error_code(&self) -> Option<&str> {
        match self {
            ServiceBusError::AzureApiError { code, .. } => Some(code),
            _ => None,
        }
    }

    /// Gets the Azure request ID if available.
    ///
    /// Request IDs are useful for tracking issues with Azure support.
    ///
    /// # Returns
    ///
    /// The Azure request ID as a string slice, or `None` if not available
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

/// Type alias for [`Result`] with [`ServiceBusError`] as the error type.
///
/// Provides convenient result handling for all Service Bus operations.
///
/// # Examples
///
/// ```no_run
/// use server::service_bus_manager::{ServiceBusResult, ServiceBusError};
///
/// fn get_queue_info() -> ServiceBusResult<String> {
///     // ... operation that might fail
///     Ok("queue-info".to_string())
/// }
/// ```
pub type ServiceBusResult<T> = Result<T, ServiceBusError>;
