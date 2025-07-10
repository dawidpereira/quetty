use super::types::AuthType;
use crate::service_bus_manager::ServiceBusError;
use async_trait::async_trait;

/// Authentication token containing access credentials for Azure Service Bus.
///
/// This struct represents an authentication token obtained from Azure AD
/// that can be used to authenticate with Azure Service Bus resources.
#[derive(Clone, Debug)]
pub struct AuthToken {
    /// The actual authentication token string
    pub token: String,
    /// The type of token (e.g., "Bearer")
    pub token_type: String,
    /// Optional expiration time in seconds from when the token was issued
    pub expires_in_secs: Option<u64>,
}

/// Trait for authentication providers that can obtain Azure AD tokens.
///
/// This trait defines the interface for different authentication methods
/// (Device Code Flow, Client Credentials, Connection String) to obtain
/// access tokens for Azure Service Bus operations.
///
/// # Examples
///
/// ```no_run
/// use server::auth::provider::{AuthProvider, AuthToken};
/// use server::auth::types::AuthType;
/// use server::service_bus_manager::ServiceBusError;
/// use async_trait::async_trait;
///
/// struct MyAuthProvider;
///
/// #[async_trait]
/// impl AuthProvider for MyAuthProvider {
///     async fn authenticate(&self) -> Result<AuthToken, ServiceBusError> {
///         // Implementation specific authentication logic
///         Ok(AuthToken {
///             token: "example_token".to_string(),
///             token_type: "Bearer".to_string(),
///             expires_in_secs: Some(3600),
///         })
///     }
///
///     fn auth_type(&self) -> AuthType {
///         AuthType::AzureAd
///     }
/// }
/// ```
#[async_trait]
pub trait AuthProvider: Send + Sync {
    /// Performs authentication and returns an access token.
    ///
    /// This method should implement the specific authentication flow
    /// for the provider (e.g., device code flow, client credentials).
    ///
    /// # Errors
    ///
    /// Returns [`ServiceBusError`] if authentication fails for any reason,
    /// including network issues, invalid credentials, or service unavailability.
    async fn authenticate(&self) -> Result<AuthToken, ServiceBusError>;

    /// Refreshes the authentication token.
    ///
    /// Default implementation calls [`authenticate`] again. Providers that
    /// support refresh tokens can override this method for more efficient
    /// token renewal.
    ///
    /// # Errors
    ///
    /// Returns [`ServiceBusError`] if token refresh fails.
    async fn refresh(&self) -> Result<AuthToken, ServiceBusError> {
        self.authenticate().await
    }

    /// Returns the authentication type used by this provider.
    ///
    /// This is used for identifying the authentication method
    /// and may affect how the token is used.
    fn auth_type(&self) -> AuthType;

    /// Indicates whether this provider's tokens require periodic refresh.
    ///
    /// Returns `true` by default. Providers with long-lived tokens
    /// (like connection strings) can override this to return `false`.
    fn requires_refresh(&self) -> bool {
        true
    }
}
