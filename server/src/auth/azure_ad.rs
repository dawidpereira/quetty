use super::provider::{AuthProvider, AuthToken};
use super::types::{AuthType, AzureAdAuthConfig, AzureAdFlowType};
use crate::service_bus_manager::ServiceBusError;
use async_trait::async_trait;
use serde::Deserialize;

pub use super::types::AzureAdFlowType as AzureAdFlow;

#[derive(Clone, Debug)]
pub struct DeviceCodeFlowInfo {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    pub message: String,
}

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
    pub fn new(config: AzureAdAuthConfig) -> Result<Self, ServiceBusError> {
        Ok(Self {
            config,
            http_client: reqwest::Client::new(),
        })
    }

    pub fn flow_type(&self) -> &AzureAdFlowType {
        &self.config.flow
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
    async fn authenticate(&self) -> Result<AuthToken, ServiceBusError> {
        match self.config.flow {
            AzureAdFlowType::DeviceCode => self.device_code_flow().await,
        }
    }

    fn auth_type(&self) -> AuthType {
        AuthType::AzureAd
    }
}
