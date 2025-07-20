use super::auth_state::{AuthStateManager, AuthenticationState};
use super::provider::{AuthProvider as AuthProviderTrait, AuthToken};
use super::types::AuthType;
use crate::service_bus_manager::ServiceBusError;
use async_trait::async_trait;
use std::sync::Arc;

/// Authentication provider that integrates with UI authentication state.
///
/// Provides a bridge between the server-side authentication system and the UI
/// authentication state, enabling seamless authentication flow coordination between
/// the terminal interface and Azure Service Bus operations.
///
/// This provider prioritizes UI-managed authentication tokens and provides fallback
/// mechanisms for automated scenarios where UI authentication is not available.
///
/// See also: [`AuthStateManager`] for state management, [`AuthProvider`](crate::auth::provider::AuthProvider) for the base trait.
///
/// # Architecture
///
/// The [`AuthProvider`] implements a hierarchical authentication strategy:
///
/// 1. **UI State Priority** - First checks for valid tokens from UI authentication
/// 2. **State-based Authentication** - Uses centralized authentication state management
/// 3. **Fallback Provider** - Falls back to alternative authentication methods if available
/// 4. **Error Propagation** - Provides detailed error feedback for authentication failures
///
/// # Authentication Flow
///
/// ```no_run
/// use quetty_server::auth::{AuthProvider, AuthStateManager};
/// use std::sync::Arc;
///
/// // Create with UI authentication state integration
/// let auth_state = Arc::new(AuthStateManager::new());
/// let provider = AuthProvider::new(auth_state, None);
///
/// // Authenticate using UI state or fallback methods
/// match provider.authenticate().await {
///     Ok(token) => {
///         println!("Authentication successful: {}", token.token);
///         // Use token for Service Bus operations
///     }
///     Err(e) => eprintln!("Authentication failed: {}", e),
/// }
/// ```
///
/// # Integration with UI Authentication
///
/// This provider seamlessly integrates with UI authentication flows:
///
/// - **Device Code Flow** - Coordinates with UI device code authentication
/// - **Token Management** - Uses UI-managed token cache and refresh logic
/// - **State Synchronization** - Maintains consistency between UI and server authentication
/// - **Error Handling** - Provides user-friendly error messages for UI display
///
/// # Fallback Authentication
///
/// When UI authentication is not available, the provider can use fallback methods:
///
/// For more details on fallback providers, see [`ConnectionStringProvider`].
///
/// ```no_run
/// use quetty_server::auth::{AuthProvider, AuthStateManager, ConnectionStringProvider};
/// use std::sync::Arc;
///
/// // Create fallback provider for automated scenarios
/// let connection_provider = Arc::new(ConnectionStringProvider::new(config)?);
/// let auth_state = Arc::new(AuthStateManager::new());
///
/// let provider = AuthProvider::new(
///     auth_state,
///     Some(connection_provider as Arc<dyn AuthProviderTrait>)
/// );
///
/// // Will use UI state if available, otherwise fall back to connection string
/// let token = provider.authenticate().await?;
/// ```
///
/// # Thread Safety
///
/// The provider is designed for concurrent access and can be safely shared across
/// multiple threads and async tasks. All internal state is protected by appropriate
/// synchronization mechanisms.
///
/// # Error Handling
///
/// Provides comprehensive error handling for various authentication scenarios:
///
/// - **Not Authenticated** - Clear guidance for users to authenticate through UI
/// - **Authentication in Progress** - Informative messages during device code flow
/// - **Authentication Failed** - Detailed error information from underlying providers
///
/// All errors are returned as [`ServiceBusError`](crate::service_bus_manager::ServiceBusError) variants.
/// - **Token Refresh Failures** - Graceful handling of token expiration scenarios
pub struct AuthProvider {
    /// Centralized authentication state manager shared with UI components
    auth_state: Arc<AuthStateManager>,
    /// Optional fallback authentication provider for automated scenarios
    fallback_provider: Option<Arc<dyn AuthProviderTrait>>,
}

impl AuthProvider {
    /// Creates a new authentication provider with UI state integration.
    ///
    /// # Arguments
    ///
    /// * `auth_state` - Shared authentication state manager for UI coordination
    /// * `fallback_provider` - Optional fallback provider for automated scenarios
    ///
    /// # Returns
    ///
    /// A new `AuthProvider` instance ready for authentication operations
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use quetty_server::auth::{AuthProvider, AuthStateManager};
    /// use std::sync::Arc;
    ///
    /// // Basic provider with UI state only
    /// let auth_state = Arc::new(AuthStateManager::new());
    /// let provider = AuthProvider::new(auth_state, None);
    ///
    /// // Provider with fallback for automated scenarios
    /// let auth_state = Arc::new(AuthStateManager::new());
    /// let fallback = Arc::new(connection_provider);
    /// let provider = AuthProvider::new(auth_state, Some(fallback));
    /// ```
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
