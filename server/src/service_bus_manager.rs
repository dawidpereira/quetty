pub use self::commands::ServiceBusCommand;
pub use self::errors::{ServiceBusError, ServiceBusResult};
pub use self::manager::ServiceBusManager;
pub use self::responses::ServiceBusResponse;
pub use self::types::*;

// Module declarations
pub mod azure_management_client;
pub mod command_handlers;
pub mod commands;
pub mod consumer_manager;
pub mod errors;
pub mod manager;
pub mod producer_manager;
pub mod queue_statistics_service;
pub mod responses;
pub mod types;

#[derive(Clone, Debug, serde::Deserialize, Default)]
pub struct AzureAdConfig {
    #[serde(default = "default_flow")]
    pub flow: String,
    tenant_id: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
    subscription_id: Option<String>,
    resource_group: Option<String>,
    pub namespace: Option<String>,
}

fn default_flow() -> String {
    "device_code".to_string()
}

impl AzureAdConfig {
    pub fn tenant_id(&self) -> Result<&str, ServiceBusError> {
        self.tenant_id.as_deref()
            .ok_or_else(|| ServiceBusError::ConfigurationError(
                "AZURE_AD__TENANT_ID is required but not found in configuration or environment variables. Please set this value in .env file or environment.".to_string()
            ))
    }

    pub fn client_id(&self) -> Result<&str, ServiceBusError> {
        self.client_id.as_deref()
            .ok_or_else(|| ServiceBusError::ConfigurationError(
                "AZURE_AD__CLIENT_ID is required but not found in configuration or environment variables. Please set this value in .env file or environment.".to_string()
            ))
    }

    pub fn client_secret(&self) -> Result<&str, ServiceBusError> {
        self.client_secret.as_deref()
            .ok_or_else(|| ServiceBusError::ConfigurationError(
                "AZURE_AD__CLIENT_SECRET is required but not found in configuration or environment variables. Please set this value in .env file or environment.".to_string()
            ))
    }

    pub fn subscription_id(&self) -> Result<&str, ServiceBusError> {
        self.subscription_id.as_deref()
            .ok_or_else(|| ServiceBusError::ConfigurationError(
                "AZURE_AD__SUBSCRIPTION_ID is required but not found in configuration or environment variables. Please set this value in .env file or environment.".to_string()
            ))
    }

    pub fn resource_group(&self) -> Result<&str, ServiceBusError> {
        self.resource_group.as_deref()
            .ok_or_else(|| ServiceBusError::ConfigurationError(
                "AZURE_AD__RESOURCE_GROUP is required but not found in configuration or environment variables. Please set this value in .env file or environment.".to_string()
            ))
    }

    pub fn namespace(&self) -> Result<&str, ServiceBusError> {
        self.namespace.as_deref()
            .ok_or_else(|| ServiceBusError::ConfigurationError(
                "AZURE_AD__NAMESPACE is required but not found in configuration or environment variables. Please set this value in .env file or environment.".to_string()
            ))
    }

    // Helper methods for validation - check if fields are present in config (not env vars)
    pub fn has_tenant_id(&self) -> bool {
        self.tenant_id.is_some()
    }

    pub fn has_client_id(&self) -> bool {
        self.client_id.is_some()
    }

    pub fn has_client_secret(&self) -> bool {
        self.client_secret.is_some()
    }

    pub fn has_subscription_id(&self) -> bool {
        self.subscription_id.is_some()
    }

    pub fn has_resource_group(&self) -> bool {
        self.resource_group.is_some()
    }

    pub fn has_namespace(&self) -> bool {
        self.namespace.is_some()
    }

    /// Azure AD operations using the new auth system
    pub async fn get_azure_ad_token(&self) -> Result<String, Box<dyn std::error::Error>> {
        use crate::auth::{
            create_auth_provider, create_service_bus_auth_provider, get_azure_ad_token_with_auth,
        };

        // If device code flow is configured, try to use UI-integrated auth first
        if self.flow == "device_code" {
            if let Ok(ui_provider) = create_auth_provider(None) {
                if let Ok(token) = get_azure_ad_token_with_auth(&ui_provider).await {
                    return Ok(token);
                }
            }
        }

        // Fallback to regular auth provider
        let auth_provider = create_service_bus_auth_provider("azure_ad", None, self)?;

        let token = get_azure_ad_token_with_auth(&auth_provider).await?;
        Ok(token)
    }

    pub async fn list_queues_azure_ad(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let token = self.get_azure_ad_token().await?;
        let url = format!(
            "https://management.azure.com/subscriptions/{}/resourceGroups/{}/providers/Microsoft.ServiceBus/namespaces/{}/queues?api-version=2017-04-01",
            self.subscription_id()?,
            self.resource_group()?,
            self.namespace()?
        );
        let client = reqwest::Client::new();
        let resp = client.get(url).bearer_auth(token).send().await?;
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

    pub async fn list_namespaces_azure_ad(
        &self,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let token = self.get_azure_ad_token().await?;
        let url = format!(
            "https://management.azure.com/subscriptions/{}/resourceGroups/{}/providers/Microsoft.ServiceBus/namespaces?api-version=2017-04-01",
            self.subscription_id()?,
            self.resource_group()?
        );
        let client = reqwest::Client::new();
        let resp = client.get(url).bearer_auth(token).send().await?;
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
