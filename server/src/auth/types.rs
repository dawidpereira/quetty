use crate::encryption::{ClientSecretEncryption, ConnectionStringEncryption, EncryptionError};
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

impl AuthConfig {
    /// Returns true if any encrypted data is present in this config
    pub fn has_encrypted_data(&self) -> bool {
        let connection_string_encrypted = self
            .connection_string
            .as_ref()
            .map(|cs| cs.is_encrypted())
            .unwrap_or(false);

        let azure_ad_encrypted = self
            .azure_ad
            .as_ref()
            .map(|ad| ad.has_encrypted_data())
            .unwrap_or(false);

        connection_string_encrypted || azure_ad_encrypted
    }

    /// Returns a list of authentication methods that require password decryption
    pub fn get_encrypted_auth_methods(&self) -> Vec<String> {
        let mut methods = Vec::new();

        if let Some(cs) = &self.connection_string {
            if cs.is_encrypted() {
                methods.push("Connection String".to_string());
            }
        }

        if let Some(ad) = &self.azure_ad {
            if ad.has_encrypted_client_secret() {
                methods.push("Azure AD Client Secret".to_string());
            }
        }

        methods
    }
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
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ConnectionStringConfig {
    /// The Azure Service Bus connection string (REQUIRED)
    /// Must include Endpoint, SharedAccessKeyName, and SharedAccessKey
    pub value: String,
    /// Encrypted connection string (alternative to value)
    pub encrypted_value: Option<String>,
    /// Salt for connection string encryption (required when encrypted_value is used)
    pub encryption_salt: Option<String>,
}

impl ConnectionStringConfig {
    /// Returns the actual connection string, decrypting if necessary
    pub fn get_connection_string(&self, password: Option<&str>) -> Result<String, EncryptionError> {
        // If we have an encrypted value, decrypt it
        if let (Some(encrypted), Some(salt)) = (&self.encrypted_value, &self.encryption_salt) {
            let password = password.ok_or_else(|| {
                EncryptionError::InvalidData(
                    "Password required for encrypted connection string".to_string(),
                )
            })?;

            let encryption = ConnectionStringEncryption::from_salt_base64(salt)?;
            encryption.decrypt_connection_string(encrypted, password)
        } else {
            // Return plain text value
            Ok(self.value.clone())
        }
    }

    /// Returns true if this config contains encrypted data
    pub fn is_encrypted(&self) -> bool {
        self.encrypted_value.is_some() && self.encryption_salt.is_some()
    }

    /// Encrypts the connection string with the given password
    pub fn encrypt_with_password(&mut self, password: &str) -> Result<(), EncryptionError> {
        if self.value.trim().is_empty() {
            return Err(EncryptionError::InvalidData(
                "Connection string cannot be empty".to_string(),
            ));
        }

        let encryption = ConnectionStringEncryption::new();
        let encrypted = encryption.encrypt_connection_string(&self.value, password)?;

        self.encrypted_value = Some(encrypted);
        self.encryption_salt = Some(encryption.salt_base64());

        // Clear the plain text value for security
        self.value.clear();

        Ok(())
    }
}

/// Configuration for Azure Active Directory authentication.
///
/// Contains all necessary parameters for Azure AD authentication flows
/// including Device Code Flow and Client Credentials Flow.
///
/// # Required Fields
///
/// - `auth_method` - Must be "device_code" or "client_secret"
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
///     auth_method: "client_secret".to_string(),
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
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AzureAdAuthConfig {
    /// Authentication method: "device_code" or "client_secret" (REQUIRED)
    #[serde(default = "default_auth_method")]
    pub auth_method: String,
    /// Azure AD tenant ID (REQUIRED for all flows)
    pub tenant_id: Option<String>,
    /// Azure AD application (client) ID (REQUIRED for all flows)
    pub client_id: Option<String>,
    /// Azure AD application client secret (REQUIRED for client_secret flow)
    pub client_secret: Option<String>,
    /// Encrypted client secret (alternative to client_secret)
    pub encrypted_client_secret: Option<String>,
    /// Salt for client secret encryption (required when encrypted_client_secret is used)
    pub client_secret_encryption_salt: Option<String>,
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

fn default_auth_method() -> String {
    "device_code".to_string()
}

impl AzureAdAuthConfig {
    /// Returns the actual client secret, decrypting if necessary
    pub fn get_client_secret(
        &self,
        password: Option<&str>,
    ) -> Result<Option<String>, EncryptionError> {
        // If we have an encrypted client secret, decrypt it
        if let (Some(encrypted), Some(salt)) = (
            &self.encrypted_client_secret,
            &self.client_secret_encryption_salt,
        ) {
            let password = password.ok_or_else(|| {
                EncryptionError::InvalidData(
                    "Password required for encrypted client secret".to_string(),
                )
            })?;

            let encryption = ClientSecretEncryption::from_salt_base64(salt)?;
            let decrypted = encryption.decrypt_client_secret(encrypted, password)?;
            Ok(Some(decrypted))
        } else {
            // Return plain text client secret
            Ok(self.client_secret.clone())
        }
    }

    /// Returns true if this config contains encrypted client secret
    pub fn has_encrypted_client_secret(&self) -> bool {
        self.encrypted_client_secret.is_some() && self.client_secret_encryption_salt.is_some()
    }

    /// Returns true if any encrypted data is present in this config
    pub fn has_encrypted_data(&self) -> bool {
        self.has_encrypted_client_secret()
    }

    /// Encrypts the client secret with the given password
    pub fn encrypt_client_secret_with_password(
        &mut self,
        password: &str,
    ) -> Result<(), EncryptionError> {
        let client_secret = match &self.client_secret {
            Some(secret) if !secret.trim().is_empty() => secret,
            _ => {
                return Err(EncryptionError::InvalidData(
                    "Client secret cannot be empty".to_string(),
                ));
            }
        };

        let encryption = ClientSecretEncryption::new();
        let encrypted = encryption.encrypt_client_secret(client_secret, password)?;

        self.encrypted_client_secret = Some(encrypted);
        self.client_secret_encryption_salt = Some(encryption.salt_base64());

        // Clear the plain text value for security
        self.client_secret = None;

        Ok(())
    }
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
