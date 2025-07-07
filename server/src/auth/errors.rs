use thiserror::Error;

/// Errors that can occur during token refresh operations
#[derive(Debug, Clone, Error)]
pub enum TokenRefreshError {
    #[error("Token refresh failed after {attempts} attempts")]
    MaxRetriesExceeded { attempts: u32 },

    #[error("Network error during token refresh: {reason}")]
    NetworkError { reason: String },

    #[error("Invalid refresh token")]
    InvalidRefreshToken,

    #[error("Token refresh not supported by provider")]
    RefreshNotSupported,

    #[error("Authentication required - refresh token expired")]
    RefreshTokenExpired,

    #[error("Rate limited by authentication provider")]
    RateLimited { retry_after_seconds: Option<u64> },

    #[error("Service unavailable: {reason}")]
    ServiceUnavailable { reason: String },

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<TokenRefreshError> for crate::service_bus_manager::ServiceBusError {
    fn from(err: TokenRefreshError) -> Self {
        match err {
            TokenRefreshError::RefreshTokenExpired | TokenRefreshError::InvalidRefreshToken => {
                crate::service_bus_manager::ServiceBusError::AuthenticationFailed(err.to_string())
            }
            TokenRefreshError::NetworkError { .. }
            | TokenRefreshError::ServiceUnavailable { .. } => {
                crate::service_bus_manager::ServiceBusError::ConnectionFailed(err.to_string())
            }
            TokenRefreshError::RateLimited { .. } => {
                crate::service_bus_manager::ServiceBusError::OperationTimeout(err.to_string())
            }
            _ => crate::service_bus_manager::ServiceBusError::AuthenticationError(err.to_string()),
        }
    }
}
