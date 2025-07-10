use super::provider::{AuthProvider, AuthToken};
use super::types::{AuthType, AzureAdAuthConfig};
use crate::service_bus_manager::ServiceBusError;
use async_trait::async_trait;
use serde::Deserialize;

/// Information required to complete an Azure AD Device Code Flow authentication.
///
/// Contains the device code, user code, and verification URL that the user needs
/// to complete the authentication process on a separate device or browser.
#[derive(Clone, Debug)]
pub struct DeviceCodeFlowInfo {
    /// Device-specific code used internally by Azure AD
    pub device_code: String,
    /// Short user code that the user enters on the verification page
    pub user_code: String,
    /// URL where the user should go to enter the user code
    pub verification_uri: String,
    /// Time in seconds until the device code expires
    pub expires_in: u64,
    /// Recommended polling interval in seconds
    pub interval: u64,
    /// Human-readable message with authentication instructions
    pub message: String,
}

/// Authentication provider for Azure Active Directory authentication flows.
///
/// Supports both Device Code Flow (for interactive scenarios) and Client Credentials Flow
/// (for service-to-service authentication). This provider handles the complete OAuth 2.0
/// authentication process with Azure AD.
///
/// # Supported Flows
///
/// - **Device Code Flow** - Interactive authentication where users enter a code on a separate device
/// - **Client Credentials Flow** - Service principal authentication using client ID and secret
///
/// # Examples
///
/// ```no_run
/// use server::auth::{AzureAdProvider, AzureAdAuthConfig};
///
/// let config = AzureAdAuthConfig {
///     auth_method: "device_code".to_string(),
///     tenant_id: Some("your-tenant-id".to_string()),
///     client_id: Some("your-client-id".to_string()),
///     ..Default::default()
/// };
///
/// let client = reqwest::Client::new();
/// let provider = AzureAdProvider::new(config, client)?;
/// let token = provider.authenticate().await?;
/// ```
#[derive(Clone)]
pub struct AzureAdProvider {
    config: AzureAdAuthConfig,
    http_client: reqwest::Client,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
    message: String,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: String,
    error_description: Option<String>,
}

impl AzureAdProvider {
    /// Creates a new AzureAdProvider with the specified configuration and HTTP client.
    ///
    /// # Arguments
    ///
    /// * `config` - Azure AD authentication configuration
    /// * `http_client` - HTTP client for making authentication requests
    ///
    /// # Returns
    ///
    /// A configured AzureAdProvider ready for authentication
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use server::auth::{AzureAdProvider, AzureAdAuthConfig};
    ///
    /// let config = AzureAdAuthConfig::default();
    /// let client = reqwest::Client::new();
    /// let provider = AzureAdProvider::new(config, client)?;
    /// ```
    pub fn new(
        config: AzureAdAuthConfig,
        http_client: reqwest::Client,
    ) -> Result<Self, ServiceBusError> {
        Ok(Self {
            config,
            http_client,
        })
    }

    /// Gets the configured authentication flow type.
    ///
    /// # Returns
    ///
    /// The authentication method string ("device_code" or "client_credentials")
    pub fn flow_type(&self) -> &str {
        &self.config.auth_method
    }

    fn authority_host(&self) -> &str {
        self.config
            .authority_host
            .as_deref()
            .unwrap_or("https://login.microsoftonline.com")
    }

    fn scope(&self) -> &str {
        self.config
            .scope
            .as_deref()
            .unwrap_or("https://management.azure.com/.default")
    }

    fn tenant_id(&self) -> Result<&str, ServiceBusError> {
        self.config.tenant_id.as_deref().ok_or_else(|| {
            ServiceBusError::ConfigurationError("Azure AD tenant_id is required".to_string())
        })
    }

    fn client_id(&self) -> Result<&str, ServiceBusError> {
        self.config.client_id.as_deref().ok_or_else(|| {
            ServiceBusError::ConfigurationError("Azure AD client_id is required".to_string())
        })
    }

    async fn device_code_flow(&self) -> Result<AuthToken, ServiceBusError> {
        // For device code flow, we need to start it and poll separately
        // This method will start the flow and immediately poll
        let device_info = self.start_device_code_flow().await?;

        // Log the device code info (without sensitive data)
        log::info!("Device code authentication initiated - awaiting user action");

        // Poll for the token
        self.poll_device_code_token(&device_info).await
    }

    async fn client_credentials_flow(&self) -> Result<AuthToken, ServiceBusError> {
        let client_secret = self.config.client_secret.as_deref().ok_or_else(|| {
            ServiceBusError::ConfigurationError(
                "Client secret is required for client credentials flow".to_string(),
            )
        })?;

        let token_url = format!(
            "{}/{}/oauth2/v2.0/token",
            self.authority_host(),
            self.tenant_id()?
        );

        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", self.client_id()?),
            ("client_secret", client_secret),
            ("scope", self.scope()),
        ];

        log::info!("Client credentials authentication initiated");

        let response = self
            .http_client
            .post(&token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                ServiceBusError::AuthenticationError(format!(
                    "Failed to authenticate with client credentials: {e}"
                ))
            })?;

        if !response.status().is_success() {
            let error_info = response
                .json::<ErrorResponse>()
                .await
                .unwrap_or(ErrorResponse {
                    error: "unknown_error".to_string(),
                    error_description: Some("Failed to parse error response".to_string()),
                });

            let user_friendly_message = match error_info.error.as_str() {
                "invalid_client" => {
                    "Invalid client credentials. Please check your client ID and client secret."
                }
                "invalid_request" => {
                    "Invalid authentication request. Please verify your configuration."
                }
                "unauthorized_client" => {
                    "This application is not authorized for client credentials flow. Please check Azure AD configuration."
                }
                "access_denied" => {
                    "Access denied. Please ensure the application has sufficient permissions."
                }
                "invalid_scope" => {
                    "Invalid scope specified. Please check the requested permissions."
                }
                _ => error_info
                    .error_description
                    .as_deref()
                    .unwrap_or(&error_info.error),
            };

            return Err(ServiceBusError::AuthenticationError(format!(
                "Client credentials authentication failed: {user_friendly_message}"
            )));
        }

        let token_response: TokenResponse = response.json().await.map_err(|e| {
            ServiceBusError::AuthenticationError(format!("Failed to parse token response: {e}"))
        })?;

        log::info!("Client credentials authentication successful");

        Ok(AuthToken {
            token: token_response.access_token,
            token_type: token_response.token_type,
            expires_in_secs: Some(token_response.expires_in),
        })
    }

    /// Initiates a Device Code Flow authentication process.
    ///
    /// This method starts the device code flow by requesting a device code from Azure AD.
    /// The returned information should be displayed to the user so they can complete
    /// authentication on a separate device or browser.
    ///
    /// # Returns
    ///
    /// [`DeviceCodeFlowInfo`] containing the user code, verification URL, and other details
    ///
    /// # Errors
    ///
    /// Returns [`ServiceBusError::AuthenticationError`] if:
    /// - The device code request fails
    /// - Invalid client configuration
    /// - Network connectivity issues
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use server::auth::AzureAdProvider;
    ///
    /// let provider = AzureAdProvider::new(config, client)?;
    /// let device_info = provider.start_device_code_flow().await?;
    ///
    /// println!("Go to: {}", device_info.verification_uri);
    /// println!("Enter code: {}", device_info.user_code);
    /// ```
    pub async fn start_device_code_flow(&self) -> Result<DeviceCodeFlowInfo, ServiceBusError> {
        let device_code_url = format!(
            "{}/{}/oauth2/v2.0/devicecode",
            self.authority_host(),
            self.tenant_id()?
        );

        let params = [("client_id", self.client_id()?), ("scope", self.scope())];

        let device_response = self
            .http_client
            .post(&device_code_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                ServiceBusError::AuthenticationError(format!(
                    "Failed to initiate device code flow: {e}"
                ))
            })?;

        // Check if the response is successful
        if !device_response.status().is_success() {
            // Try to parse error response
            let error_info =
                device_response
                    .json::<ErrorResponse>()
                    .await
                    .unwrap_or(ErrorResponse {
                        error: "unknown_error".to_string(),
                        error_description: Some("Failed to parse error response".to_string()),
                    });

            let user_friendly_message = match error_info.error.as_str() {
                "invalid_client" => {
                    "Invalid client configuration. Please check your Azure AD app registration and ensure 'Allow public client flows' is enabled."
                }
                "invalid_request" => {
                    "Invalid authentication request. Please check your client ID and tenant ID."
                }
                "unauthorized_client" => {
                    "This application is not authorized for device code flow. Please check Azure AD configuration."
                }
                "access_denied" => {
                    "Access denied. Please ensure you have the necessary permissions."
                }
                "expired_token" => "Authentication expired. Please try again.",
                _ => error_info
                    .error_description
                    .as_deref()
                    .unwrap_or(&error_info.error),
            };

            return Err(ServiceBusError::AuthenticationError(format!(
                "Authentication failed: {user_friendly_message}"
            )));
        }

        let device_code: DeviceCodeResponse = device_response.json().await.map_err(|e| {
            ServiceBusError::AuthenticationError(format!(
                "Failed to parse device code response: {e}"
            ))
        })?;

        Ok(DeviceCodeFlowInfo {
            device_code: device_code.device_code,
            user_code: device_code.user_code,
            verification_uri: device_code.verification_uri,
            expires_in: device_code.expires_in,
            interval: device_code.interval,
            message: device_code.message,
        })
    }

    /// Polls Azure AD for completion of device code authentication.
    ///
    /// This method continuously polls Azure AD to check if the user has completed
    /// the device code authentication process. It handles all the standard OAuth 2.0
    /// device flow polling logic including backoff and error handling.
    ///
    /// # Arguments
    ///
    /// * `device_info` - Device code information from [`start_device_code_flow`]
    ///
    /// # Returns
    ///
    /// An [`AuthToken`] when authentication is successfully completed
    ///
    /// # Errors
    ///
    /// Returns [`ServiceBusError::AuthenticationError`] if:
    /// - Authentication times out or expires
    /// - User denies access
    /// - Network errors during polling
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use server::auth::AzureAdProvider;
    ///
    /// let provider = AzureAdProvider::new(config, client)?;
    /// let device_info = provider.start_device_code_flow().await?;
    ///
    /// // Display info to user...
    ///
    /// let token = provider.poll_device_code_token(&device_info).await?;
    /// ```
    pub async fn poll_device_code_token(
        &self,
        device_info: &DeviceCodeFlowInfo,
    ) -> Result<AuthToken, ServiceBusError> {
        let token_url = format!(
            "{}/{}/oauth2/v2.0/token",
            self.authority_host(),
            self.tenant_id()?
        );

        let mut interval = std::time::Duration::from_secs(device_info.interval);
        let timeout = std::time::Duration::from_secs(device_info.expires_in);
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                return Err(ServiceBusError::AuthenticationError(
                    "Authentication timed out. The device code has expired. Please restart the authentication process.".to_string()
                ));
            }

            tokio::time::sleep(interval).await;

            let mut params = vec![
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", self.client_id()?),
                ("device_code", device_info.device_code.as_str()),
            ];

            // Include client_secret if configured (for confidential clients)
            if let Some(client_secret) = self.config.client_secret.as_deref() {
                params.push(("client_secret", client_secret));
            }

            let response = self
                .http_client
                .post(&token_url)
                .form(&params)
                .send()
                .await
                .map_err(|e| {
                    ServiceBusError::AuthenticationError(format!("Failed to poll for token: {e}"))
                })?;

            if response.status().is_success() {
                let token_response: TokenResponse = response.json().await.map_err(|e| {
                    ServiceBusError::AuthenticationError(format!(
                        "Failed to parse token response: {e}"
                    ))
                })?;

                return Ok(AuthToken {
                    token: token_response.access_token,
                    token_type: token_response.token_type,
                    expires_in_secs: Some(token_response.expires_in),
                });
            }

            let error_response: serde_json::Value = response.json().await.unwrap_or_default();

            if let Some(error) = error_response["error"].as_str() {
                match error {
                    "authorization_pending" => {
                        log::debug!("Waiting for user to complete authentication");
                        continue;
                    }
                    "slow_down" => {
                        log::debug!("Polling too frequently, increasing interval");
                        interval += std::time::Duration::from_secs(5);
                        continue;
                    }
                    "expired_token" => {
                        return Err(ServiceBusError::AuthenticationError(
                            "The device code has expired. Please restart the authentication process.".to_string()
                        ));
                    }
                    "access_denied" => {
                        return Err(ServiceBusError::AuthenticationError(
                            "Access was denied. Please ensure you have the necessary permissions."
                                .to_string(),
                        ));
                    }
                    _ => {
                        let error_desc = error_response["error_description"]
                            .as_str()
                            .unwrap_or("Unknown error occurred");
                        return Err(ServiceBusError::AuthenticationError(format!(
                            "Authentication failed: {error} - {error_desc}"
                        )));
                    }
                }
            }
        }
    }
}

#[async_trait]
impl AuthProvider for AzureAdProvider {
    /// Authenticates using the configured Azure AD authentication flow.
    ///
    /// Automatically selects the appropriate authentication method based on the
    /// configuration (device_code or client_credentials) and handles the complete
    /// OAuth 2.0 flow including error handling and token retrieval.
    ///
    /// # Returns
    ///
    /// An [`AuthToken`] containing the Azure AD access token and metadata
    ///
    /// # Errors
    ///
    /// Returns [`ServiceBusError`] if:
    /// - Authentication method is not supported
    /// - Authentication flow fails
    /// - Network connectivity issues
    /// - Invalid credentials or configuration
    async fn authenticate(&self) -> Result<AuthToken, ServiceBusError> {
        match self.config.auth_method.as_str() {
            "device_code" => self.device_code_flow().await,
            "client_secret" => self.client_credentials_flow().await,
            _ => Err(ServiceBusError::ConfigurationError(format!(
                "Unsupported auth method: {}",
                self.config.auth_method
            ))),
        }
    }

    /// Returns the authentication type for this provider.
    ///
    /// # Returns
    ///
    /// [`AuthType::AzureAd`] indicating Azure Active Directory authentication
    fn auth_type(&self) -> AuthType {
        AuthType::AzureAd
    }
}
