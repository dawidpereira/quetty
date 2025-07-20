use super::provider::AuthProvider;
use super::token_cache::TokenCache;
use super::token_refresh_service::TokenRefreshService;
use super::types::DeviceCodeInfo;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

/// Authentication state tracking for the application.
///
/// Represents the current authentication status and provides context
/// for ongoing authentication processes like device code flows.
#[derive(Clone, Debug, Default)]
pub enum AuthenticationState {
    /// No authentication is currently active
    #[default]
    NotAuthenticated,
    /// Device code authentication is in progress
    AwaitingDeviceCode {
        /// Device code information for user interaction
        info: DeviceCodeInfo,
        /// When the device code flow was initiated
        started_at: Instant,
    },
    /// Authentication completed successfully
    Authenticated {
        /// The authentication token
        token: String,
        /// When the token expires
        expires_at: Instant,
        /// Optional connection string for Service Bus operations
        connection_string: Option<String>,
    },
    /// Authentication failed with error message
    Failed(String),
}

// Consolidated state structure to prevent deadlocks
#[derive(Default)]
struct AuthState {
    authentication_state: AuthenticationState,
    azure_ad_token: Option<(String, Instant)>,
    sas_token: Option<(String, Instant)>,
    service_bus_provider: Option<Arc<dyn AuthProvider>>,
    management_provider: Option<Arc<dyn AuthProvider>>,
    refresh_service: Option<Arc<TokenRefreshService>>,
    refresh_handle: Option<JoinHandle<()>>,
}

/// Centralized authentication state management for the application.
///
/// Manages authentication state, token caching, and refresh services across
/// the entire application. Provides thread-safe access to authentication
/// providers and tokens with automatic expiration handling.
///
/// # Features
///
/// - Thread-safe state management with RwLock
/// - Token caching with automatic expiration
/// - Authentication provider management
/// - Token refresh service integration
/// - Device code flow support
///
/// # Examples
///
/// ```no_run
/// use quetty_server::auth::AuthStateManager;
/// use std::sync::Arc;
///
/// let auth_manager = Arc::new(AuthStateManager::new());
///
/// // Check authentication status
/// if !auth_manager.is_authenticated().await {
///     // Start authentication process
/// }
///
/// // Get cached tokens
/// if let Some(token) = auth_manager.get_azure_ad_token().await {
///     // Use token for API calls
/// }
/// ```
pub struct AuthStateManager {
    inner: Arc<RwLock<AuthState>>,
    token_cache: TokenCache,
}

impl AuthStateManager {
    /// Creates a new authentication state manager.
    ///
    /// # Returns
    ///
    /// A new AuthStateManager with clean state and empty token cache
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(AuthState::default())),
            token_cache: TokenCache::new(),
        }
    }

    /// Gets the current authentication state.
    ///
    /// # Returns
    ///
    /// The current [`AuthenticationState`] indicating the authentication status
    pub async fn get_state(&self) -> AuthenticationState {
        self.inner.read().await.authentication_state.clone()
    }

    /// Sets the authentication state to indicate device code flow is in progress.
    ///
    /// This method is called when a device code authentication flow has been initiated
    /// and is waiting for user interaction to complete the authentication process.
    ///
    /// # Arguments
    ///
    /// * `info` - Device code information including user code and verification URL
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use quetty_server::auth::{AuthStateManager, DeviceCodeInfo};
    /// use std::sync::Arc;
    ///
    /// let auth_manager = Arc::new(AuthStateManager::new());
    /// let device_info = DeviceCodeInfo {
    ///     device_code: "device123".to_string(),
    ///     user_code: "ABC123".to_string(),
    ///     verification_uri: "https://microsoft.com/devicelogin".to_string(),
    ///     expires_in: 900,
    ///     interval: 5,
    ///     message: "Enter code ABC123 at https://microsoft.com/devicelogin".to_string(),
    /// };
    ///
    /// auth_manager.set_device_code_pending(device_info).await;
    /// ```
    pub async fn set_device_code_pending(&self, info: DeviceCodeInfo) {
        let mut state = self.inner.write().await;
        state.authentication_state = AuthenticationState::AwaitingDeviceCode {
            info,
            started_at: Instant::now(),
        };
    }

    pub async fn set_authenticated(
        &self,
        token: String,
        expires_in: Duration,
        connection_string: Option<String>,
    ) {
        let mut state = self.inner.write().await;
        let expires_at = Instant::now() + expires_in;

        state.authentication_state = AuthenticationState::Authenticated {
            token: token.clone(),
            expires_at,
            connection_string,
        };

        // Store Azure AD token
        state.azure_ad_token = Some((token, expires_at));
    }

    /// Sets the authentication state to failed with an error message.
    ///
    /// This method is called when authentication attempts fail, providing
    /// detailed error information that can be displayed to the user.
    ///
    /// # Arguments
    ///
    /// * `error` - Human-readable error message describing the authentication failure
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use quetty_server::auth::AuthStateManager;
    /// use std::sync::Arc;
    ///
    /// let auth_manager = Arc::new(AuthStateManager::new());
    /// auth_manager.set_failed("Invalid credentials provided".to_string()).await;
    /// ```
    pub async fn set_failed(&self, error: String) {
        let mut state = self.inner.write().await;
        state.authentication_state = AuthenticationState::Failed(error);
    }

    /// Logs out the user and clears all authentication state.
    ///
    /// This method resets the authentication state to `NotAuthenticated` and
    /// clears all cached tokens and authentication providers. It also stops
    /// any running token refresh services.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use quetty_server::auth::AuthStateManager;
    /// use std::sync::Arc;
    ///
    /// let auth_manager = Arc::new(AuthStateManager::new());
    ///
    /// // After authentication...
    /// auth_manager.logout().await;
    ///
    /// // State is now reset
    /// assert!(!auth_manager.is_authenticated().await);
    /// ```
    pub async fn logout(&self) {
        let mut state = self.inner.write().await;
        state.authentication_state = AuthenticationState::NotAuthenticated;
        state.azure_ad_token = None;
        state.sas_token = None;
    }

    /// Checks if the user is currently authenticated.
    ///
    /// # Returns
    ///
    /// `true` if authentication is successful and active, `false` otherwise
    pub async fn is_authenticated(&self) -> bool {
        let state = self.inner.read().await;
        matches!(
            state.authentication_state,
            AuthenticationState::Authenticated { .. }
        )
    }

    /// Checks if reauthentication is needed.
    ///
    /// Returns `true` if the user is not authenticated or if the current
    /// authentication token expires within 5 minutes.
    ///
    /// # Returns
    ///
    /// `true` if reauthentication is required, `false` if current auth is still valid
    pub async fn needs_reauthentication(&self) -> bool {
        let state = self.inner.read().await;
        match &state.authentication_state {
            AuthenticationState::Authenticated { expires_at, .. } => {
                // Check if token expires in less than 5 minutes
                Instant::now() + Duration::from_secs(300) >= *expires_at
            }
            _ => true,
        }
    }

    /// Retrieves a valid Azure AD access token if available.
    ///
    /// Returns the cached Azure AD token if it exists and hasn't expired.
    /// This token can be used for authenticating with Azure Service Bus
    /// and other Azure resources.
    ///
    /// # Returns
    ///
    /// * `Some(token)` - Valid Azure AD access token
    /// * `None` - No token available or token has expired
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use quetty_server::auth::AuthStateManager;
    /// use std::sync::Arc;
    ///
    /// let auth_manager = Arc::new(AuthStateManager::new());
    ///
    /// if let Some(token) = auth_manager.get_azure_ad_token().await {
    ///     println!("Using Azure AD token: {}", token);
    ///     // Use token for Service Bus operations
    /// } else {
    ///     println!("No valid Azure AD token available");
    /// }
    /// ```
    pub async fn get_azure_ad_token(&self) -> Option<String> {
        let state = self.inner.read().await;
        if let Some((token_str, expires_at)) = &state.azure_ad_token {
            if Instant::now() < *expires_at {
                return Some(token_str.clone());
            }
        }
        None
    }

    /// Retrieves a valid SAS token if available.
    ///
    /// Returns the cached SAS (Shared Access Signature) token if it exists
    /// and hasn't expired. SAS tokens are used for connection string-based
    /// authentication with Azure Service Bus.
    ///
    /// # Returns
    ///
    /// * `Some(token)` - Valid SAS token
    /// * `None` - No token available or token has expired
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use quetty_server::auth::AuthStateManager;
    /// use std::sync::Arc;
    ///
    /// let auth_manager = Arc::new(AuthStateManager::new());
    ///
    /// if let Some(sas_token) = auth_manager.get_sas_token().await {
    ///     println!("Using SAS token: {}", sas_token);
    ///     // Use token for Service Bus operations
    /// } else {
    ///     println!("No valid SAS token available");
    /// }
    /// ```
    pub async fn get_sas_token(&self) -> Option<String> {
        let state = self.inner.read().await;
        if let Some((token_str, expires_at)) = &state.sas_token {
            if Instant::now() < *expires_at {
                return Some(token_str.clone());
            }
        }
        None
    }

    /// Stores a SAS token with its expiration time.
    ///
    /// Caches a SAS token for future use with automatic expiration handling.
    /// The token will be considered invalid after the specified duration.
    ///
    /// # Arguments
    ///
    /// * `token` - The SAS token string to cache
    /// * `expires_in` - Duration until the token expires
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use quetty_server::auth::AuthStateManager;
    /// use std::sync::Arc;
    /// use std::time::Duration;
    ///
    /// let auth_manager = Arc::new(AuthStateManager::new());
    /// let token = "SharedAccessSignature sr=...".to_string();
    /// let expires_in = Duration::from_secs(24 * 3600); // 24 hours
    ///
    /// auth_manager.set_sas_token(token, expires_in).await;
    /// ```
    pub async fn set_sas_token(&self, token: String, expires_in: Duration) {
        let mut state = self.inner.write().await;
        state.sas_token = Some((token, Instant::now() + expires_in));
    }

    /// Retrieves the connection string from the current authentication state.
    ///
    /// Returns the connection string if the user is authenticated and a
    /// connection string is available in the authentication state.
    ///
    /// # Returns
    ///
    /// * `Some(connection_string)` - Valid connection string for Service Bus
    /// * `None` - No connection string available or not authenticated
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use quetty_server::auth::AuthStateManager;
    /// use std::sync::Arc;
    ///
    /// let auth_manager = Arc::new(AuthStateManager::new());
    ///
    /// if let Some(conn_str) = auth_manager.get_connection_string().await {
    ///     println!("Using connection string: {}", conn_str);
    ///     // Use connection string for Service Bus operations
    /// }
    /// ```
    pub async fn get_connection_string(&self) -> Option<String> {
        let state = self.inner.read().await;
        match &state.authentication_state {
            AuthenticationState::Authenticated {
                connection_string, ..
            } => connection_string.clone(),
            _ => None,
        }
    }

    /// Retrieves device code information if device code flow is in progress.
    ///
    /// Returns the device code information (user code, verification URL, etc.)
    /// if a device code authentication flow is currently active.
    ///
    /// # Returns
    ///
    /// * `Some(DeviceCodeInfo)` - Device code flow information
    /// * `None` - No device code flow is currently active
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use quetty_server::auth::AuthStateManager;
    /// use std::sync::Arc;
    ///
    /// let auth_manager = Arc::new(AuthStateManager::new());
    ///
    /// if let Some(device_info) = auth_manager.get_device_code_info().await {
    ///     println!("Go to: {}", device_info.verification_uri);
    ///     println!("Enter code: {}", device_info.user_code);
    /// }
    /// ```
    pub async fn get_device_code_info(&self) -> Option<DeviceCodeInfo> {
        let state = self.inner.read().await;
        match &state.authentication_state {
            AuthenticationState::AwaitingDeviceCode { info, .. } => Some(info.clone()),
            _ => None,
        }
    }

    // Provider management methods

    /// Sets the authentication provider for Service Bus operations.
    ///
    /// Configures the authentication provider that will be used for
    /// Service Bus data plane operations (sending/receiving messages).
    ///
    /// # Arguments
    ///
    /// * `provider` - Authentication provider for Service Bus operations
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use quetty_server::auth::{AuthStateManager, AzureAdProvider};
    /// use std::sync::Arc;
    ///
    /// let auth_manager = Arc::new(AuthStateManager::new());
    /// let provider = Arc::new(AzureAdProvider::new(config, client)?);
    ///
    /// auth_manager.set_service_bus_provider(provider).await;
    /// ```
    pub async fn set_service_bus_provider(&self, provider: Arc<dyn AuthProvider>) {
        let mut state = self.inner.write().await;
        state.service_bus_provider = Some(provider);
    }

    /// Retrieves the current Service Bus authentication provider.
    ///
    /// Returns the authentication provider configured for Service Bus
    /// data plane operations if one has been set.
    ///
    /// # Returns
    ///
    /// * `Some(provider)` - Configured Service Bus authentication provider
    /// * `None` - No provider has been configured
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use quetty_server::auth::AuthStateManager;
    /// use std::sync::Arc;
    ///
    /// let auth_manager = Arc::new(AuthStateManager::new());
    ///
    /// if let Some(provider) = auth_manager.get_service_bus_provider().await {
    ///     let token = provider.authenticate().await?;
    ///     // Use token for Service Bus operations
    /// }
    /// ```
    pub async fn get_service_bus_provider(&self) -> Option<Arc<dyn AuthProvider>> {
        self.inner.read().await.service_bus_provider.clone()
    }

    /// Sets the authentication provider for Service Bus management operations.
    ///
    /// Configures the authentication provider that will be used for
    /// Service Bus management plane operations (creating queues, topics, etc.).
    ///
    /// # Arguments
    ///
    /// * `provider` - Authentication provider for management operations
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use quetty_server::auth::{AuthStateManager, AzureAdProvider};
    /// use std::sync::Arc;
    ///
    /// let auth_manager = Arc::new(AuthStateManager::new());
    /// let provider = Arc::new(AzureAdProvider::new(config, client)?);
    ///
    /// auth_manager.set_management_provider(provider).await;
    /// ```
    pub async fn set_management_provider(&self, provider: Arc<dyn AuthProvider>) {
        let mut state = self.inner.write().await;
        state.management_provider = Some(provider);
    }

    /// Retrieves the current Service Bus management authentication provider.
    ///
    /// Returns the authentication provider configured for Service Bus
    /// management plane operations if one has been set.
    ///
    /// # Returns
    ///
    /// * `Some(provider)` - Configured management authentication provider
    /// * `None` - No provider has been configured
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use quetty_server::auth::AuthStateManager;
    /// use std::sync::Arc;
    ///
    /// let auth_manager = Arc::new(AuthStateManager::new());
    ///
    /// if let Some(provider) = auth_manager.get_management_provider().await {
    ///     let token = provider.authenticate().await?;
    ///     // Use token for management operations
    /// }
    /// ```
    pub async fn get_management_provider(&self) -> Option<Arc<dyn AuthProvider>> {
        self.inner.read().await.management_provider.clone()
    }

    /// Gets a reference to the token cache.
    ///
    /// # Returns
    ///
    /// A reference to the [`TokenCache`] for manual token management
    pub fn get_token_cache(&self) -> &TokenCache {
        &self.token_cache
    }

    // Token refresh service management

    /// Starts the automatic token refresh service.
    ///
    /// Initiates a background service that automatically refreshes tokens
    /// before they expire, ensuring continuous authentication without
    /// user intervention.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use quetty_server::auth::AuthStateManager;
    /// use std::sync::Arc;
    ///
    /// let auth_manager = Arc::new(AuthStateManager::new());
    ///
    /// // Start automatic token refresh
    /// auth_manager.clone().start_refresh_service().await;
    ///
    /// // Tokens will now be refreshed automatically in the background
    /// ```
    pub async fn start_refresh_service(self: Arc<Self>) {
        self.start_refresh_service_with_callback(None).await;
    }

    pub async fn start_refresh_service_with_callback(
        self: Arc<Self>,
        failure_callback: Option<super::token_refresh_service::RefreshFailureCallback>,
    ) {
        // Stop any existing service
        self.stop_refresh_service().await;

        // Create and start new service
        let mut refresh_service = TokenRefreshService::new(self.clone());
        if let Some(callback) = failure_callback {
            refresh_service = refresh_service.with_failure_callback(callback);
        }

        let refresh_service = Arc::new(refresh_service);
        let handle = refresh_service.clone().start();

        // Store service and handle in consolidated state
        let mut state = self.inner.write().await;
        state.refresh_service = Some(refresh_service);
        state.refresh_handle = Some(handle);

        log::info!("Token refresh service started");
    }

    /// Stops the automatic token refresh service.
    ///
    /// Gracefully shuts down the background token refresh service,
    /// stopping automatic token renewal. Tokens will no longer be
    /// refreshed automatically after calling this method.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use quetty_server::auth::AuthStateManager;
    /// use std::sync::Arc;
    ///
    /// let auth_manager = Arc::new(AuthStateManager::new());
    ///
    /// // Start refresh service
    /// auth_manager.clone().start_refresh_service().await;
    ///
    /// // Later, stop the service
    /// auth_manager.stop_refresh_service().await;
    /// ```
    pub async fn stop_refresh_service(&self) {
        // Get service reference and signal shutdown
        let service_ref = {
            let state = self.inner.read().await;
            state.refresh_service.clone()
        };

        if let Some(service) = service_ref {
            service.shutdown().await;
        }

        // Wait for service to stop and clear references
        let mut state = self.inner.write().await;
        if let Some(handle) = state.refresh_handle.take() {
            // Drop the write lock before waiting
            drop(state);
            let _ = handle.await;

            // Re-acquire write lock to clear service reference
            let mut state = self.inner.write().await;
            state.refresh_service = None;
        } else {
            state.refresh_service = None;
        }

        log::info!("Token refresh service stopped");
    }
}

impl Default for AuthStateManager {
    fn default() -> Self {
        Self::new()
    }
}
