pub use self::commands::ServiceBusCommand;
pub use self::errors::{ServiceBusError, ServiceBusResult};
pub use self::manager::ServiceBusManager;
pub use self::responses::ServiceBusResponse;
pub use self::types::*;

// Module declarations
pub mod commands;
pub mod consumer_manager;
pub mod errors;
pub mod manager;
pub mod producer_manager;
pub mod responses;
pub mod types;

#[derive(Clone, Debug, serde::Deserialize)]
pub struct AzureAdConfig {
    tenant_id: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
    subscription_id: Option<String>,
    resource_group: Option<String>,
    pub namespace: Option<String>,
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

    /// Azure AD operations
    pub async fn get_azure_ad_token(&self) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            self.tenant_id()?
        );
        let client = reqwest::Client::new();
        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", self.client_id()?),
            ("client_secret", self.client_secret()?),
            ("scope", "https://management.azure.com/.default"),
        ];
        let resp = client.post(url).form(&params).send().await?;
        let json: serde_json::Value = resp.json().await?;
        let token = json["access_token"]
            .as_str()
            .ok_or("No access_token in response")?
            .to_string();
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
