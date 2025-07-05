use super::auth_state::{AuthStateManager, AuthenticationState};
use super::provider::{AuthProvider as AuthProviderTrait, AuthToken};
use super::types::AuthType;
use crate::service_bus_manager::ServiceBusError;
use async_trait::async_trait;
use std::sync::Arc;

/// Auth provider that integrates with UI auth state
pub struct AuthProvider {
    auth_state: Arc<AuthStateManager>,
    fallback_provider: Option<Arc<dyn AuthProviderTrait>>,
}

impl AuthProvider {
    pub fn new(
        auth_state: Arc<AuthStateManager>,
        fallback_provider: Option<Arc<dyn AuthProviderTrait>>,
    ) -> Self {
        Self {
            auth_state,
            fallback_provider,
        }
    }
}

#[async_trait]
impl AuthProviderTrait for AuthProvider {
    async fn authenticate(&self) -> Result<AuthToken, ServiceBusError> {
        // First check if we have a valid token from UI authentication
        if let Some(token) = self.auth_state.get_azure_ad_token().await {
            return Ok(AuthToken {
                token,
                token_type: "Bearer".to_string(),
                expires_in_secs: Some(3600), // Default 1 hour
            });
        }

        // Check the authentication state
        match self.auth_state.get_state().await {
            AuthenticationState::Authenticated { token, .. } => {
                Ok(AuthToken {
                    token,
                    token_type: "Bearer".to_string(),
                    expires_in_secs: Some(3600), // Default 1 hour
                })
            }
            AuthenticationState::AwaitingDeviceCode { .. } => {
                Err(ServiceBusError::AuthenticationError(
                    "Authentication in progress. Please complete device code authentication in the UI.".to_string()
                ))
            }
            AuthenticationState::Failed(error) => {
                Err(ServiceBusError::AuthenticationError(
                    format!("Authentication failed: {error}")
                ))
            }
            AuthenticationState::NotAuthenticated => {
                // If we have a fallback provider, try it
                if let Some(fallback) = &self.fallback_provider {
                    fallback.authenticate().await
                } else {
                    Err(ServiceBusError::AuthenticationError(
                        "Not authenticated. Please authenticate through the UI first.".to_string()
                    ))
                }
            }
        }
    }

    async fn refresh(&self) -> Result<AuthToken, ServiceBusError> {
        // For now, just try to authenticate again
        self.authenticate().await
    }

    fn auth_type(&self) -> AuthType {
        AuthType::AzureAd
    }

    fn requires_refresh(&self) -> bool {
        // Let the auth state manager handle refresh logic
        false
    }
}
