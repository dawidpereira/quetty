use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Authentication method types supported by the application.
///
/// This enum defines the different ways the application can authenticate
/// with Azure Service Bus resources.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    /// Direct connection using Service Bus connection string
    ConnectionString,
    /// Azure Active Directory authentication (Device Code or Client Credentials)
    AzureAd,
}

/// Complete authentication configuration for the application.
///
/// This struct contains all authentication-related settings including
/// the primary authentication method and fallback options.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    /// The primary authentication method to use
    pub primary_method: AuthType,
    /// Whether to enable fallback to alternative authentication methods
    pub fallback_enabled: bool,
    /// Connection string configuration (if using ConnectionString auth)
    pub connection_string: Option<ConnectionStringConfig>,
    /// Azure AD authentication configuration (if using AzureAd auth)
    pub azure_ad: Option<AzureAdAuthConfig>,
}

/// Configuration for connection string authentication.
///
/// Contains the Service Bus connection string used for direct authentication
/// using Shared Access Signatures (SAS). This is the simplest authentication
/// method but requires managing connection strings securely.
///
/// # Required Fields
///
/// - `value` - Complete Azure Service Bus connection string with access credentials
///
/// # Connection String Format
///
/// The connection string must include:
/// - `Endpoint` - Service Bus namespace endpoint
/// - `SharedAccessKeyName` - Name of the shared access key
/// - `SharedAccessKey` - The shared access key value
///
/// # Examples
///
/// ```no_run
/// use server::auth::types::ConnectionStringConfig;
///
/// let config = ConnectionStringConfig {
///     value: "Endpoint=sb://my-namespace.servicebus.windows.net/;SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=abcd1234...".to_string(),
/// };
/// ```
///
/// # Security Considerations
///
/// - Store connection strings securely (environment variables, key vault, etc.)
/// - Use principle of least privilege - avoid "RootManageSharedAccessKey" in production
/// - Rotate access keys regularly
/// - Consider using Azure AD authentication for enhanced security
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectionStringConfig {
    /// The Azure Service Bus connection string (REQUIRED)
    /// Must include Endpoint, SharedAccessKeyName, and SharedAccessKey
    pub value: String,
}

/// Configuration for Azure Active Directory authentication.
///
/// Contains all necessary parameters for Azure AD authentication flows
/// including Device Code Flow and Client Credentials Flow.
///
/// # Required Fields
///
/// - `auth_method` - Must be "device_code" or "client_credentials"
///
/// # Required for Device Code Flow
///
/// - `tenant_id` - Azure AD tenant ID
/// - `client_id` - Azure AD application (client) ID
///
/// # Required for Client Credentials Flow
///
/// - `tenant_id` - Azure AD tenant ID
/// - `client_id` - Azure AD application (client) ID
/// - `client_secret` - Azure AD application client secret
///
/// # Optional Fields
///
/// - `subscription_id` - For resource discovery (defaults to env AZURE_SUBSCRIPTION_ID)
/// - `resource_group` - For resource discovery (defaults to auto-discovery)
/// - `namespace` - Service Bus namespace (defaults to auto-discovery)
/// - `authority_host` - Azure AD authority host (defaults to https://login.microsoftonline.com)
/// - `scope` - OAuth scope (defaults to https://servicebus.azure.net/.default)
///
/// # Examples
///
/// ## Device Code Flow Configuration
/// ```no_run
/// use server::auth::types::AzureAdAuthConfig;
///
/// let config = AzureAdAuthConfig {
///     auth_method: "device_code".to_string(),
///     tenant_id: Some("your-tenant-id".to_string()),
///     client_id: Some("your-client-id".to_string()),
///     client_secret: None, // Not needed for device code flow
///     subscription_id: Some("your-subscription-id".to_string()),
///     resource_group: Some("your-resource-group".to_string()),
///     namespace: Some("your-servicebus-namespace".to_string()),
///     authority_host: None, // Uses default
///     scope: None, // Uses default
/// };
/// ```
///
/// ## Client Credentials Flow Configuration
/// ```no_run
/// use server::auth::types::AzureAdAuthConfig;
///
/// let config = AzureAdAuthConfig {
///     auth_method: "client_credentials".to_string(),
///     tenant_id: Some("your-tenant-id".to_string()),
///     client_id: Some("your-client-id".to_string()),
///     client_secret: Some("your-client-secret".to_string()), // Required
///     subscription_id: None, // Optional
///     resource_group: None, // Optional
///     namespace: None, // Optional
///     authority_host: None, // Uses default
///     scope: None, // Uses default
/// };
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AzureAdAuthConfig {
    /// Authentication method: "device_code" or "client_credentials" (REQUIRED)
    pub auth_method: String,
    /// Azure AD tenant ID (REQUIRED for all flows)
    pub tenant_id: Option<String>,
    /// Azure AD application (client) ID (REQUIRED for all flows)
    pub client_id: Option<String>,
    /// Azure AD application client secret (REQUIRED for client_credentials flow)
    pub client_secret: Option<String>,
    /// Azure subscription ID (OPTIONAL - defaults to env AZURE_SUBSCRIPTION_ID)
    pub subscription_id: Option<String>,
    /// Resource group name (OPTIONAL - defaults to auto-discovery)
    pub resource_group: Option<String>,
    /// Service Bus namespace name (OPTIONAL - defaults to auto-discovery)
    pub namespace: Option<String>,
    /// Azure AD authority host URL (OPTIONAL - defaults to https://login.microsoftonline.com)
    pub authority_host: Option<String>,
    /// OAuth scope for token requests (OPTIONAL - defaults to https://servicebus.azure.net/.default)
    pub scope: Option<String>,
}

/// A cached authentication token with expiration tracking.
///
/// This struct holds an authentication token along with its expiration time
/// to enable efficient token caching and refresh logic.
#[derive(Clone, Debug)]
pub struct CachedToken {
    /// The authentication token string
    pub token: String,
    /// When the token expires
    pub expires_at: Instant,
    /// The type of token (e.g., "Bearer")
    pub token_type: String,
}

impl CachedToken {
    /// Creates a new cached token with the given parameters.
    ///
    /// # Arguments
    ///
    /// * `token` - The authentication token string
    /// * `expires_in` - Duration until the token expires
    /// * `token_type` - The type of token (e.g., "Bearer")
    pub fn new(token: String, expires_in: Duration, token_type: String) -> Self {
        Self {
            token,
            expires_at: Instant::now() + expires_in,
            token_type,
        }
    }

    /// Checks if the token has expired.
    ///
    /// # Returns
    ///
    /// `true` if the token has passed its expiration time, `false` otherwise.
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    /// Checks if the token needs to be refreshed soon.
    ///
    /// Uses a 5-minute buffer before expiration to ensure tokens are
    /// refreshed before they actually expire.
    ///
    /// # Returns
    ///
    /// `true` if the token should be refreshed, `false` otherwise.
    pub fn needs_refresh(&self) -> bool {
        let buffer = Duration::from_secs(300); // 5 minute buffer
        Instant::now() + buffer >= self.expires_at
    }
}

/// Information required for Azure AD Device Code Flow authentication.
///
/// This struct contains the user code and verification URL that the user
/// needs to complete the device code authentication flow.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceCodeInfo {
    /// The user code to be entered on the verification page
    pub user_code: String,
    /// The URL where the user should enter the code
    pub verification_uri: String,
    /// Human-readable message with authentication instructions
    pub message: String,
}
