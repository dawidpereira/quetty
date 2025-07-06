use super::auth_state::AuthStateManager;
use super::provider::AuthProvider;
use super::types::CachedToken;
use crate::common::TokenRefreshError;
use crate::service_bus_manager::ServiceBusError;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::{interval, sleep};

/// Callback for handling refresh failures
pub type RefreshFailureCallback = Arc<dyn Fn(TokenRefreshError) + Send + Sync>;

/// Service that periodically checks and refreshes tokens before they expire
pub struct TokenRefreshService {
    auth_state: Arc<AuthStateManager>,
    check_interval: Duration,
    shutdown_signal: Arc<RwLock<bool>>,
    failure_callback: Option<RefreshFailureCallback>,
}

impl TokenRefreshService {
    /// Create a new token refresh service
    pub fn new(auth_state: Arc<AuthStateManager>) -> Self {
        Self {
            auth_state,
            check_interval: Duration::from_secs(120), // Check every 2 minutes
            shutdown_signal: Arc::new(RwLock::new(false)),
            failure_callback: None,
        }
    }

    /// Set a callback to be invoked when token refresh fails
    pub fn with_failure_callback(mut self, callback: RefreshFailureCallback) -> Self {
        self.failure_callback = Some(callback);
        self
    }

    /// Start the background refresh service
    pub fn start(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            self.run().await;
        })
    }

    /// Signal the service to shutdown
    pub async fn shutdown(&self) {
        let mut shutdown = self.shutdown_signal.write().await;
        *shutdown = true;
    }

    /// Run the refresh service loop
    async fn run(&self) {
        let mut check_interval = interval(self.check_interval);
        check_interval.tick().await; // Skip the first immediate tick

        loop {
            // Check if we should shutdown
            if *self.shutdown_signal.read().await {
                log::info!("Token refresh service shutting down");
                break;
            }

            check_interval.tick().await;

            // Check and refresh tokens
            if let Err(e) = self.check_and_refresh_tokens().await {
                log::error!("Error during token refresh check: {e}");
            }
        }
    }

    /// Check all cached tokens and refresh those that need it
    async fn check_and_refresh_tokens(&self) -> Result<(), ServiceBusError> {
        log::debug!("Checking tokens for refresh...");

        // Check Service Bus token
        if let Some(provider) = self.auth_state.get_service_bus_provider().await {
            self.refresh_if_needed("service_bus", provider).await?;
        }

        // Check Management API token
        if let Some(provider) = self.auth_state.get_management_provider().await {
            self.refresh_if_needed("management_api", provider).await?;
        }

        Ok(())
    }

    /// Refresh a specific token if it needs refreshing
    async fn refresh_if_needed(
        &self,
        cache_key: &str,
        provider: Arc<dyn AuthProvider>,
    ) -> Result<(), ServiceBusError> {
        let token_cache = self.auth_state.get_token_cache();

        if token_cache.needs_refresh(cache_key).await {
            log::info!("Token for '{cache_key}' needs refresh, attempting refresh...");

            match self.refresh_with_retry(provider, 3).await {
                Ok(auth_token) => {
                    // Store the refreshed token
                    let cached_token = CachedToken::new(
                        auth_token.token,
                        Duration::from_secs(auth_token.expires_in_secs.unwrap_or(3600)),
                        auth_token.token_type,
                    );

                    token_cache.set(cache_key.to_string(), cached_token).await;
                    log::info!("Successfully refreshed token for '{cache_key}'");
                }
                Err(e) => {
                    log::error!("Failed to refresh token for '{cache_key}': {e}");

                    // Invalidate the token so next access will trigger re-authentication
                    token_cache.invalidate(cache_key).await;

                    // Invoke failure callback if set
                    if let Some(callback) = &self.failure_callback {
                        callback(e.clone());
                    }

                    // Convert to ServiceBusError
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    /// Attempt to refresh a token with retry logic
    async fn refresh_with_retry(
        &self,
        provider: Arc<dyn AuthProvider>,
        max_attempts: u32,
    ) -> Result<super::provider::AuthToken, TokenRefreshError> {
        let mut last_error = None;

        for attempt in 1..=max_attempts {
            match provider.refresh().await {
                Ok(token) => return Ok(token),
                Err(e) => {
                    // Convert ServiceBusError to TokenRefreshError
                    let refresh_error = match &e {
                        ServiceBusError::AuthenticationFailed(_) => {
                            TokenRefreshError::InvalidRefreshToken
                        }
                        ServiceBusError::AuthenticationError(msg) if msg.contains("expired") => {
                            TokenRefreshError::RefreshTokenExpired
                        }
                        ServiceBusError::ConnectionFailed(reason) => {
                            TokenRefreshError::NetworkError {
                                reason: reason.clone(),
                            }
                        }
                        ServiceBusError::OperationTimeout(msg) => {
                            if msg.contains("rate") {
                                TokenRefreshError::RateLimited {
                                    retry_after_seconds: None,
                                }
                            } else {
                                TokenRefreshError::ServiceUnavailable {
                                    reason: msg.clone(),
                                }
                            }
                        }
                        _ => TokenRefreshError::Internal(e.to_string()),
                    };

                    last_error = Some(refresh_error);

                    if attempt < max_attempts {
                        let delay = Duration::from_secs(2u64.pow(attempt - 1)); // Exponential backoff: 1s, 2s, 4s
                        log::warn!(
                            "Token refresh attempt {attempt} failed, retrying in {delay:?}..."
                        );
                        sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or(TokenRefreshError::MaxRetriesExceeded {
            attempts: max_attempts,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::auth_state::AuthStateManager;
    use crate::auth::provider::{AuthProvider, AuthToken};
    use crate::auth::types::AuthType;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};

    // Mock provider for testing
    struct MockAuthProvider {
        refresh_count: Arc<AtomicU32>,
        should_fail: bool,
    }

    #[async_trait]
    impl AuthProvider for MockAuthProvider {
        async fn authenticate(&self) -> Result<AuthToken, ServiceBusError> {
            Ok(AuthToken {
                token: "test_token".to_string(),
                token_type: "Bearer".to_string(),
                expires_in_secs: Some(3600),
            })
        }

        async fn refresh(&self) -> Result<AuthToken, ServiceBusError> {
            self.refresh_count.fetch_add(1, Ordering::SeqCst);

            if self.should_fail {
                Err(ServiceBusError::AuthenticationError(
                    "Mock refresh failure".to_string(),
                ))
            } else {
                self.authenticate().await
            }
        }

        fn auth_type(&self) -> AuthType {
            AuthType::ConnectionString
        }
    }

    #[tokio::test]
    async fn test_refresh_with_retry_success() {
        let auth_state = Arc::new(AuthStateManager::new());
        let service = TokenRefreshService::new(auth_state);

        let provider = Arc::new(MockAuthProvider {
            refresh_count: Arc::new(AtomicU32::new(0)),
            should_fail: false,
        });

        let result = service.refresh_with_retry(provider.clone(), 3).await;
        assert!(result.is_ok());
        assert_eq!(provider.refresh_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_refresh_with_retry_failure() {
        let auth_state = Arc::new(AuthStateManager::new());
        let service = TokenRefreshService::new(auth_state);

        let provider = Arc::new(MockAuthProvider {
            refresh_count: Arc::new(AtomicU32::new(0)),
            should_fail: true,
        });

        let result = service.refresh_with_retry(provider.clone(), 3).await;
        assert!(result.is_err());
        assert_eq!(provider.refresh_count.load(Ordering::SeqCst), 3); // All 3 attempts
    }
}
