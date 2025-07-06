use super::azure_ad::AzureAdProvider;
use super::connection_string::ConnectionStringProvider;
use super::provider::AuthProvider;
use super::types::{AuthType, AzureAdAuthConfig, ConnectionStringConfig};
use crate::service_bus_manager::{AzureAdConfig, ServiceBusError};
use std::sync::Arc;

/// Creates an auth provider based on configuration
pub fn create_auth_provider(
    primary_method: &str,
    connection_string: Option<&str>,
    azure_ad_config: &AzureAdConfig,
) -> Result<Arc<dyn AuthProvider>, ServiceBusError> {
    let auth_type = match primary_method {
        "azure_ad" => AuthType::AzureAd,
        _ => AuthType::ConnectionString,
    };

    create_provider_for_type(&auth_type, connection_string, azure_ad_config)
}

fn create_provider_for_type(
    auth_type: &AuthType,
    connection_string: Option<&str>,
    azure_ad_config: &AzureAdConfig,
) -> Result<Arc<dyn AuthProvider>, ServiceBusError> {
    match auth_type {
        AuthType::ConnectionString => {
            let conn_str = connection_string.ok_or_else(|| {
                ServiceBusError::ConfigurationError("Connection string is required".to_string())
            })?;

            let provider = ConnectionStringProvider::new(ConnectionStringConfig {
                value: conn_str.to_string(),
            })?;
            Ok(Arc::new(provider))
        }
        AuthType::AzureAd => {
            let azure_auth_config = AzureAdAuthConfig {
                auth_method: azure_ad_config.auth_method.clone(),
                tenant_id: azure_ad_config.tenant_id().ok().map(|s| s.to_string()),
                client_id: azure_ad_config.client_id().ok().map(|s| s.to_string()),
                client_secret: azure_ad_config.client_secret().ok().map(|s| s.to_string()),
                subscription_id: azure_ad_config
                    .subscription_id()
                    .ok()
                    .map(|s| s.to_string()),
                resource_group: azure_ad_config.resource_group().ok().map(|s| s.to_string()),
                namespace: azure_ad_config.namespace().ok().map(|s| s.to_string()),
                authority_host: None,
                scope: None,
            };

            let provider = AzureAdProvider::new(azure_auth_config)?;
            Ok(Arc::new(provider))
        }
    }
}

/// Helper to get Azure AD token using the new auth system
pub async fn get_azure_ad_token_with_auth(
    auth_provider: &Arc<dyn AuthProvider>,
) -> Result<String, ServiceBusError> {
    let auth_token = auth_provider.authenticate().await?;
    Ok(auth_token.token)
}
