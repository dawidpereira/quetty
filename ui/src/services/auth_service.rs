use crate::components::common::{AuthActivityMsg, Msg};
use crate::error::{AppError, AppResult};
use server::auth::auth_state::AuthStateManager;
use server::auth::{AuthProvider, AzureAdProvider};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::Duration;

pub struct AuthService {
    auth_state: Arc<AuthStateManager>,
    azure_ad_provider: Option<Arc<AzureAdProvider>>,
    tx: Sender<Msg>,
}

impl AuthService {
    pub fn new(
        config: &server::service_bus_manager::AzureAdConfig,
        tx: Sender<Msg>,
        http_client: reqwest::Client,
    ) -> AppResult<Self> {
        // Use shared auth state
        let auth_state = super::init_shared_auth_state();

        // Convert AzureAdConfig to AzureAdAuthConfig
        let auth_config = server::auth::types::AzureAdAuthConfig {
            auth_method: config.auth_method.clone(),
            tenant_id: config.tenant_id().ok().map(|s| s.to_string()),
            client_id: config.client_id().ok().map(|s| s.to_string()),
            client_secret: config.client_secret().ok().map(|s| s.to_string()),
            subscription_id: config.subscription_id().ok().map(|s| s.to_string()),
            resource_group: config.resource_group().ok().map(|s| s.to_string()),
            namespace: config.namespace.clone(),
            authority_host: None,
            scope: None,
        };

        let azure_ad_provider = Some(Arc::new(
            AzureAdProvider::new(auth_config, http_client)
                .map_err(|e| AppError::Auth(e.to_string()))?,
        ));

        Ok(Self {
            auth_state,
            azure_ad_provider,
            tx,
        })
    }

    /// Initiate authentication flow
    pub async fn initiate_authentication(&self) -> AppResult<()> {
        let provider = self
            .azure_ad_provider
            .as_ref()
            .ok_or_else(|| AppError::Auth("Azure AD not configured".to_string()))?;

        // Check if device code flow is configured
        if provider.flow_type() == "device_code" {
            return self.handle_device_code_flow(provider.clone()).await;
        }

        // For other flows, authenticate directly
        match provider.authenticate().await {
            Ok(token) => {
                self.auth_state
                    .set_authenticated(
                        token.token,
                        Duration::from_secs(token.expires_in_secs.unwrap_or(3600)),
                        None,
                    )
                    .await;

                self.tx
                    .send(Msg::AuthActivity(AuthActivityMsg::AuthenticationSuccess))
                    .map_err(|e| AppError::Channel(e.to_string()))?;
            }
            Err(e) => {
                self.auth_state.set_failed(e.to_string()).await;

                self.tx
                    .send(Msg::AuthActivity(AuthActivityMsg::AuthenticationFailed(
                        e.to_string(),
                    )))
                    .map_err(|e| AppError::Channel(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// Handle device code flow authentication
    async fn handle_device_code_flow(&self, provider: Arc<AzureAdProvider>) -> AppResult<()> {
        // Start device code flow
        match provider.start_device_code_flow().await {
            Ok(device_code_info) => {
                // Send device code info to UI
                self.tx
                    .send(Msg::AuthActivity(AuthActivityMsg::ShowDeviceCode {
                        user_code: device_code_info.user_code.clone(),
                        verification_url: device_code_info.verification_uri.clone(),
                        message: device_code_info.message.clone(),
                        expires_in: device_code_info.expires_in,
                    }))
                    .map_err(|e| AppError::Channel(e.to_string()))?;

                // Update auth state with device code info
                let device_code = server::auth::DeviceCodeInfo {
                    user_code: device_code_info.user_code.clone(),
                    verification_uri: device_code_info.verification_uri.clone(),
                    message: device_code_info.message.clone(),
                };
                self.auth_state.set_device_code_pending(device_code).await;

                // Start polling for authentication in background
                let auth_state = self.auth_state.clone();
                let tx = self.tx.clone();
                let provider = provider.clone();

                tokio::spawn(async move {
                    match provider.poll_device_code_token(&device_code_info).await {
                        Ok(token) => {
                            auth_state
                                .set_authenticated(
                                    token.token,
                                    Duration::from_secs(token.expires_in_secs.unwrap_or(3600)),
                                    None,
                                )
                                .await;

                            let _ =
                                tx.send(Msg::AuthActivity(AuthActivityMsg::AuthenticationSuccess));
                        }
                        Err(e) => {
                            auth_state.set_failed(e.to_string()).await;
                            let _ = tx.send(Msg::AuthActivity(
                                AuthActivityMsg::AuthenticationFailed(e.to_string()),
                            ));
                        }
                    }
                });

                Ok(())
            }
            Err(e) => {
                self.auth_state.set_failed(e.to_string()).await;

                self.tx
                    .send(Msg::AuthActivity(AuthActivityMsg::AuthenticationFailed(
                        e.to_string(),
                    )))
                    .map_err(|e| AppError::Channel(e.to_string()))?;

                Err(AppError::Auth(e.to_string()))
            }
        }
    }

    /// Get the current auth state manager for sharing with other services
    pub fn auth_state_manager(&self) -> Arc<AuthStateManager> {
        self.auth_state.clone()
    }

    /// Get device code info if in device code flow
    pub async fn get_device_code_info(&self) -> Option<server::auth::DeviceCodeInfo> {
        self.auth_state.get_device_code_info().await
    }

    /// Get a token for Azure Management API operations
    pub async fn get_management_token(&self) -> Result<String, AppError> {
        // For now, we'll use the same token as Service Bus
        // In the future, we might need to request a different scope
        match self.auth_state.get_azure_ad_token().await {
            Some(token) => Ok(token),
            None => {
                // Try to authenticate if not already authenticated
                self.initiate_authentication().await?;

                // Wait a bit for authentication to complete
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                self.auth_state
                    .get_azure_ad_token()
                    .await
                    .ok_or_else(|| AppError::Auth("Failed to obtain management token".to_string()))
            }
        }
    }
}
