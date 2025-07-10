//! # Service Bus Manager Module
//!
//! This module provides comprehensive Azure Service Bus management capabilities,
//! including queue operations, message handling, authentication, and Azure resource discovery.
//!
//! ## Core Components
//!
//! - [`ServiceBusManager`] - Main interface for Service Bus operations
//! - [`AzureManagementClient`] - Azure Resource Manager integration
//! - [`ServiceBusCommand`] / [`ServiceBusResponse`] - Command/response pattern for operations
//! - [`ServiceBusError`] - Comprehensive error handling
//!
//! ## Features
//!
//! - **Queue Management** - Create, list, and manage Service Bus queues
//! - **Message Operations** - Send, receive, and bulk process messages
//! - **Authentication** - Multiple auth methods (Device Code, Client Credentials, Connection String)
//! - **Resource Discovery** - Discover Azure subscriptions, resource groups, and namespaces
//! - **Statistics** - Queue statistics and monitoring
//! - **Bulk Operations** - Efficient bulk message processing
//!
//! ## Usage
//!
//! ```no_run
//! use server::service_bus_manager::{ServiceBusManager, AzureAdConfig};
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = AzureAdConfig::default();
//!     let manager = ServiceBusManager::new(config).await?;
//!
//!     // List available queues
//!     let queues = manager.list_queues().await?;
//!
//!     // Connect to a specific queue
//!     manager.connect_to_queue("my-queue").await?;
//!
//!     Ok(())
//! }
//! ```

pub use self::azure_management_client::{
    AccessKeys, AzureManagementClient, NamespaceProperties, ResourceGroup, ServiceBusNamespace,
    Subscription,
};
pub use self::commands::ServiceBusCommand;
pub use self::errors::{ServiceBusError, ServiceBusResult};
pub use self::manager::ServiceBusManager;
pub use self::responses::ServiceBusResponse;
pub use self::types::*;

/// Azure Management Client for resource discovery and management
pub mod azure_management_client;
/// Command handlers for processing Service Bus operations
pub mod command_handlers;
/// Command definitions for Service Bus operations
pub mod commands;
/// Consumer management for message reception
pub mod consumer_manager;
/// Error types and handling for Service Bus operations
pub mod errors;
/// Main Service Bus Manager implementation
pub mod manager;
/// Producer management for message sending
pub mod producer_manager;
/// Queue statistics and monitoring services
pub mod queue_statistics_service;
/// Response types for Service Bus operations
pub mod responses;
/// Core types and data structures
pub mod types;

use crate::utils::env::EnvUtils;

/// Configuration for Azure Active Directory authentication and resource access.
///
/// This struct contains all the necessary information for authenticating with Azure AD
/// and accessing Azure Service Bus resources. It supports multiple authentication methods
/// and can load configuration from both direct values and environment variables.
///
/// # Authentication Methods
///
/// - `device_code` - Interactive device code flow (default for CLI usage)
/// - `client_credentials` - Service principal authentication
/// - `connection_string` - Direct connection string authentication
///
/// # Examples
///
/// ```no_run
/// use server::service_bus_manager::AzureAdConfig;
///
/// let config = AzureAdConfig {
///     auth_method: "device_code".to_string(),
///     tenant_id: Some("tenant-id".to_string()),
///     client_id: Some("client-id".to_string()),
///     subscription_id: Some("subscription-id".to_string()),
///     resource_group: Some("resource-group".to_string()),
///     namespace: Some("servicebus-namespace".to_string()),
///     ..Default::default()
/// };
/// ```
#[derive(Clone, Debug, serde::Deserialize, Default)]
pub struct AzureAdConfig {
    /// Authentication method: "device_code", "client_credentials", or "connection_string"
    #[serde(default = "default_auth_method")]
    pub auth_method: String,
    /// Azure AD tenant ID
    pub tenant_id: Option<String>,
    /// Azure AD application (client) ID
    pub client_id: Option<String>,
    /// Azure AD application client secret (required for client_credentials)
    pub client_secret: Option<String>,
    /// Azure subscription ID for resource discovery
    pub subscription_id: Option<String>,
    /// Resource group name containing the Service Bus namespace
    pub resource_group: Option<String>,
    /// Service Bus namespace name
    pub namespace: Option<String>,
}

fn default_auth_method() -> String {
    "connection_string".to_string()
}

impl AzureAdConfig {
    /// Gets the Azure AD tenant ID, returning an error if not configured.
    ///
    /// # Returns
    ///
    /// The tenant ID as a string reference
    ///
    /// # Errors
    ///
    /// Returns [`ServiceBusError::ConfigurationError`] if the tenant ID is not set
    pub fn tenant_id(&self) -> Result<&str, ServiceBusError> {
        self.tenant_id.as_deref()
            .ok_or_else(|| ServiceBusError::ConfigurationError(
                "AZURE_AD__TENANT_ID is required but not found in configuration or environment variables. Please set this value in .env file or environment.".to_string()
            ))
    }

    /// Gets the Azure AD client ID, returning an error if not configured.
    ///
    /// # Returns
    ///
    /// The client ID as a string reference
    ///
    /// # Errors
    ///
    /// Returns [`ServiceBusError::ConfigurationError`] if the client ID is not set
    pub fn client_id(&self) -> Result<&str, ServiceBusError> {
        self.client_id.as_deref()
            .ok_or_else(|| ServiceBusError::ConfigurationError(
                "AZURE_AD__CLIENT_ID is required but not found in configuration or environment variables. Please set this value in .env file or environment.".to_string()
            ))
    }

    /// Gets the Azure AD client secret, returning an error if not configured.
    ///
    /// Required for client credentials authentication flow.
    ///
    /// # Returns
    ///
    /// The client secret as a string reference
    ///
    /// # Errors
    ///
    /// Returns [`ServiceBusError::ConfigurationError`] if the client secret is not set
    pub fn client_secret(&self) -> Result<&str, ServiceBusError> {
        self.client_secret.as_deref()
            .ok_or_else(|| ServiceBusError::ConfigurationError(
                "AZURE_AD__CLIENT_SECRET is required but not found in configuration or environment variables. Please set this value in .env file or environment.".to_string()
            ))
    }

    /// Gets the Azure subscription ID from config or environment variables.
    ///
    /// Checks the config first, then falls back to the `AZURE_AD__SUBSCRIPTION_ID` environment variable.
    ///
    /// # Returns
    ///
    /// The subscription ID as a string
    ///
    /// # Errors
    ///
    /// Returns [`ServiceBusError::ConfigurationError`] if the subscription ID is not found
    pub fn subscription_id(&self) -> Result<String, ServiceBusError> {
        if let Some(ref id) = self.subscription_id {
            Ok(id.clone())
        } else {
            EnvUtils::get_validated_var("AZURE_AD__SUBSCRIPTION_ID")
                .map_err(|_| ServiceBusError::ConfigurationError(
                    "AZURE_AD__SUBSCRIPTION_ID is required but not found in configuration or environment variables. Please set this value in .env file or environment.".to_string()
                ))
        }
    }

    /// Gets the Azure resource group name from config or environment variables.
    ///
    /// Checks the config first, then falls back to the `AZURE_AD__RESOURCE_GROUP` environment variable.
    ///
    /// # Returns
    ///
    /// The resource group name as a string
    ///
    /// # Errors
    ///
    /// Returns [`ServiceBusError::ConfigurationError`] if the resource group is not found
    pub fn resource_group(&self) -> Result<String, ServiceBusError> {
        if let Some(ref group) = self.resource_group {
            Ok(group.clone())
        } else {
            EnvUtils::get_validated_var("AZURE_AD__RESOURCE_GROUP")
                .map_err(|_| ServiceBusError::ConfigurationError(
                    "AZURE_AD__RESOURCE_GROUP is required but not found in configuration or environment variables. Please set this value in .env file or environment.".to_string()
                ))
        }
    }

    /// Gets the Service Bus namespace name from config or environment variables.
    ///
    /// Checks the config first, then falls back to the `AZURE_AD__NAMESPACE` environment variable.
    ///
    /// # Returns
    ///
    /// The namespace name as a string
    ///
    /// # Errors
    ///
    /// Returns [`ServiceBusError::ConfigurationError`] if the namespace is not found
    pub fn namespace(&self) -> Result<String, ServiceBusError> {
        if let Some(ref ns) = self.namespace {
            Ok(ns.clone())
        } else {
            EnvUtils::get_validated_var("AZURE_AD__NAMESPACE")
                .map_err(|_| ServiceBusError::ConfigurationError(
                    "AZURE_AD__NAMESPACE is required but not found in configuration or environment variables. Please set this value in .env file or environment.".to_string()
                ))
        }
    }

    /// Checks if tenant ID is configured (in config only, not environment).
    ///
    /// # Returns
    ///
    /// `true` if tenant ID is set in the configuration
    pub fn has_tenant_id(&self) -> bool {
        self.tenant_id.is_some()
    }

    /// Checks if client ID is configured (in config only, not environment).
    ///
    /// # Returns
    ///
    /// `true` if client ID is set in the configuration
    pub fn has_client_id(&self) -> bool {
        self.client_id.is_some()
    }

    /// Checks if client secret is configured (in config only, not environment).
    ///
    /// # Returns
    ///
    /// `true` if client secret is set in the configuration
    pub fn has_client_secret(&self) -> bool {
        self.client_secret.is_some()
    }

    /// Checks if subscription ID is available (config or environment).
    ///
    /// # Returns
    ///
    /// `true` if subscription ID is available from config or environment variables
    pub fn has_subscription_id(&self) -> bool {
        self.subscription_id.is_some() || EnvUtils::has_non_empty_var("AZURE_AD__SUBSCRIPTION_ID")
    }

    /// Checks if resource group is available (config or environment).
    ///
    /// # Returns
    ///
    /// `true` if resource group is available from config or environment variables
    pub fn has_resource_group(&self) -> bool {
        self.resource_group.is_some() || EnvUtils::has_non_empty_var("AZURE_AD__RESOURCE_GROUP")
    }

    /// Checks if namespace is available (config or environment).
    ///
    /// # Returns
    ///
    /// `true` if namespace is available from config or environment variables
    pub fn has_namespace(&self) -> bool {
        self.namespace.is_some() || EnvUtils::has_non_empty_var("AZURE_AD__NAMESPACE")
    }

    /// Obtains an Azure AD access token using the configured authentication method.
    ///
    /// This method tries different authentication approaches based on the configured auth method:
    /// 1. For device code flow, attempts UI-integrated auth first
    /// 2. Falls back to regular auth provider for other methods
    /// 3. Returns an error for connection string auth (no Azure AD token available)
    ///
    /// # Arguments
    ///
    /// * `http_client` - HTTP client for making authentication requests
    ///
    /// # Returns
    ///
    /// An Azure AD access token string
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication fails
    /// - Connection string auth is used (Azure AD tokens not available)
    /// - Required configuration is missing
    pub async fn get_azure_ad_token(
        &self,
        http_client: &reqwest::Client,
    ) -> Result<String, Box<dyn std::error::Error>> {
        use crate::auth::{
            create_auth_provider, create_service_bus_auth_provider, get_azure_ad_token_with_auth,
        };

        // If device code flow is configured, try to use UI-integrated auth first
        if self.auth_method == "device_code" {
            if let Ok(ui_provider) = create_auth_provider(None) {
                if let Ok(token) = get_azure_ad_token_with_auth(&ui_provider).await {
                    return Ok(token);
                }
            }
        }

        // For connection string authentication, we cannot get Azure AD tokens
        if self.auth_method == "connection_string" {
            return Err("Azure AD token not available for connection string authentication".into());
        }

        // Fallback to regular auth provider
        let auth_provider =
            create_service_bus_auth_provider("azure_ad", None, self, http_client.clone())?;

        let token = get_azure_ad_token_with_auth(&auth_provider).await?;
        Ok(token)
    }

    /// Lists all Service Bus queues in the configured namespace using Azure AD authentication.
    ///
    /// # Arguments
    ///
    /// * `http_client` - HTTP client for making API requests
    ///
    /// # Returns
    ///
    /// A vector of queue names
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication fails
    /// - Azure Management API request fails
    /// - Required configuration (subscription, resource group, namespace) is missing
    pub async fn list_queues_azure_ad(
        &self,
        http_client: &reqwest::Client,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let token = self.get_azure_ad_token(http_client).await?;
        let url = format!(
            "https://management.azure.com/subscriptions/{}/resourceGroups/{}/providers/Microsoft.ServiceBus/namespaces/{}/queues?api-version=2017-04-01",
            self.subscription_id()?,
            self.resource_group()?,
            self.namespace()?
        );

        let resp = http_client.get(url).bearer_auth(token).send().await?;
        let json: serde_json::Value = resp.json().await?;
        let mut queues = Vec::new();
        if let Some(arr) = json["value"].as_array() {
            for queue in arr {
                if let Some(name) = queue["name"].as_str() {
                    queues.push(name.to_string());
                }
            }
        }
        Ok(queues)
    }

    /// Lists all Service Bus namespaces in the configured resource group using Azure AD authentication.
    ///
    /// # Arguments
    ///
    /// * `http_client` - HTTP client for making API requests
    ///
    /// # Returns
    ///
    /// A vector of namespace names
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication fails
    /// - Azure Management API request fails
    /// - Required configuration (subscription, resource group) is missing
    pub async fn list_namespaces_azure_ad(
        &self,
        http_client: &reqwest::Client,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let token = self.get_azure_ad_token(http_client).await?;
        let url = format!(
            "https://management.azure.com/subscriptions/{}/resourceGroups/{}/providers/Microsoft.ServiceBus/namespaces?api-version=2017-04-01",
            self.subscription_id()?,
            self.resource_group()?
        );

        let resp = http_client.get(url).bearer_auth(token).send().await?;
        let json: serde_json::Value = resp.json().await?;
        let mut namespaces = Vec::new();
        if let Some(arr) = json["value"].as_array() {
            for ns in arr {
                if let Some(name) = ns["name"].as_str() {
                    namespaces.push(name.to_string());
                }
            }
        }
        Ok(namespaces)
    }
}
