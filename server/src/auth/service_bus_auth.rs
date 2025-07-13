//! Service Bus authentication provider creation and management.
//!
//! This module provides high-level functions for creating and managing authentication
//! providers for Azure Service Bus operations. It abstracts the complexity of choosing
//! between different authentication methods and configuring them appropriately.

use super::azure_ad::AzureAdProvider;
use super::connection_string::ConnectionStringProvider;
use super::provider::AuthProvider;
use super::types::{AuthType, AzureAdAuthConfig, ConnectionStringConfig};
use crate::service_bus_manager::{AzureAdConfig, ServiceBusError};
use std::sync::Arc;

/// Creates an authentication provider based on the specified configuration.
///
/// This function serves as the main entry point for creating authentication providers
/// for Service Bus operations. It analyzes the provided configuration and creates
/// the appropriate provider type (Connection String or Azure AD).
///
/// # Arguments
///
/// * `primary_method` - The primary authentication method ("azure_ad" or "connection_string")
/// * `connection_string` - Optional connection string for connection string authentication
/// * `azure_ad_config` - Azure AD configuration for Azure AD authentication
/// * `http_client` - HTTP client for making authentication requests
///
/// # Returns
///
/// An [`AuthProvider`] configured for the specified authentication method
///
/// # Errors
///
/// Returns [`ServiceBusError`] if:
/// - Required configuration is missing for the selected method
/// - Provider initialization fails
/// - Invalid configuration values are provided
///
/// # Examples
///
/// ```no_run
/// use server::auth::service_bus_auth::create_auth_provider;
/// use server::service_bus_manager::AzureAdConfig;
///
/// // Create Azure AD provider
/// let azure_config = AzureAdConfig {
///     auth_method: "device_code".to_string(),
///     tenant_id: Some("tenant-id".to_string()),
///     client_id: Some("client-id".to_string()),
///     ..Default::default()
/// };
///
/// let provider = create_auth_provider(
///     "azure_ad",
///     None,
///     &azure_config,
///     reqwest::Client::new()
/// )?;
///
/// // Create connection string provider
/// let provider = create_auth_provider(
///     "connection_string",
///     Some("Endpoint=sb://test.servicebus.windows.net/;..."),
///     &AzureAdConfig::default(),
///     reqwest::Client::new()
/// )?;
/// ```
pub fn create_auth_provider(
    primary_method: &str,
    connection_string: Option<&str>,
    azure_ad_config: &AzureAdConfig,
    http_client: reqwest::Client,
) -> Result<Arc<dyn AuthProvider>, ServiceBusError> {
    let auth_type = match primary_method {
        "azure_ad" => AuthType::AzureAd,
        _ => AuthType::ConnectionString,
    };

    create_provider_for_type(&auth_type, connection_string, azure_ad_config, http_client)
}

/// Creates a specific authentication provider based on the authentication type.
///
/// Internal function that handles the actual provider creation for different
/// authentication types. Converts configurations and initializes the appropriate
/// provider implementation.
///
/// # Arguments
///
/// * `auth_type` - The type of authentication to create
/// * `connection_string` - Optional connection string
/// * `azure_ad_config` - Azure AD configuration
/// * `http_client` - HTTP client for requests
///
/// # Returns
///
/// An [`AuthProvider`] for the specified authentication type
fn create_provider_for_type(
    auth_type: &AuthType,
    connection_string: Option<&str>,
    azure_ad_config: &AzureAdConfig,
    http_client: reqwest::Client,
) -> Result<Arc<dyn AuthProvider>, ServiceBusError> {
    match auth_type {
        AuthType::ConnectionString => {
            let conn_str = connection_string.ok_or_else(|| {
                ServiceBusError::ConfigurationError("Connection string is required".to_string())
            })?;

            let provider = ConnectionStringProvider::new(ConnectionStringConfig {
                value: conn_str.to_string(),
                encrypted_value: None,
                encryption_salt: None,
            })?;
            Ok(Arc::new(provider))
        }
        AuthType::AzureAd => {
            let azure_auth_config = AzureAdAuthConfig {
                auth_method: azure_ad_config.auth_method.clone(),
                tenant_id: azure_ad_config.tenant_id().ok().map(|s| s.to_string()),
                client_id: azure_ad_config.client_id().ok().map(|s| s.to_string()),
                client_secret: azure_ad_config.client_secret().ok().map(|s| s.to_string()),
                encrypted_client_secret: None,
                client_secret_encryption_salt: None,
                subscription_id: azure_ad_config
                    .subscription_id()
                    .ok()
                    .map(|s| s.to_string()),
                resource_group: azure_ad_config.resource_group().ok().map(|s| s.to_string()),
                namespace: azure_ad_config.namespace().ok().map(|s| s.to_string()),
                authority_host: None,
                scope: None,
            };

            let provider = AzureAdProvider::new(azure_auth_config, http_client)?;
            Ok(Arc::new(provider))
        }
    }
}

/// Gets an Azure AD token using the provided authentication provider.
///
/// This is a convenience function that performs authentication using any
/// authentication provider and extracts just the token string. Useful for
/// scenarios where only the token is needed.
///
/// # Arguments
///
/// * `auth_provider` - The authentication provider to use
///
/// # Returns
///
/// The authentication token as a string
///
/// # Errors
///
/// Returns [`ServiceBusError`] if authentication fails
///
/// # Examples
///
/// ```no_run
/// use server::auth::service_bus_auth::{create_auth_provider, get_azure_ad_token_with_auth};
///
/// let provider = create_auth_provider(/* config */).await?;
/// let token = get_azure_ad_token_with_auth(&provider).await?;
///
/// // Use token for Service Bus operations
/// println!("Token: {}", token);
/// ```
pub async fn get_azure_ad_token_with_auth(
    auth_provider: &Arc<dyn AuthProvider>,
) -> Result<String, ServiceBusError> {
    let auth_token = auth_provider.authenticate().await?;
    Ok(auth_token.token)
}
