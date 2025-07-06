use crate::service_bus_manager::ServiceBusError;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

//TODO: I wonder if this should be separated file or part of azure_management_client?
const AZURE_MANAGEMENT_URL: &str = "https://management.azure.com";
const API_VERSION_SUBSCRIPTIONS: &str = "2022-12-01";
const API_VERSION_RESOURCE_GROUPS: &str = "2021-04-01";
const API_VERSION_SERVICE_BUS: &str = "2021-11-01";

#[derive(Debug, Clone)]
pub struct AzureManagementClient {
    client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Subscription {
    pub id: String,
    #[serde(rename = "subscriptionId")]
    pub subscription_id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceGroup {
    pub id: String,
    pub name: String,
    pub location: String,
    #[serde(default)]
    pub tags: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServiceBusNamespace {
    pub id: String,
    pub name: String,
    pub location: String,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub properties: NamespaceProperties,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NamespaceProperties {
    #[serde(rename = "serviceBusEndpoint")]
    pub service_bus_endpoint: String,
    pub status: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccessKeys {
    #[serde(rename = "primaryConnectionString")]
    pub primary_connection_string: String,
    #[serde(rename = "secondaryConnectionString")]
    pub secondary_connection_string: String,
    #[serde(rename = "primaryKey")]
    pub primary_key: String,
    #[serde(rename = "secondaryKey")]
    pub secondary_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListResponse<T> {
    value: Vec<T>,
    #[serde(rename = "nextLink")]
    next_link: Option<String>,
}

impl AzureManagementClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// List all subscriptions accessible to the authenticated user
    pub async fn list_subscriptions(
        &self,
        token: &str,
    ) -> Result<Vec<Subscription>, ServiceBusError> {
        let url = format!(
            "{}/subscriptions?api-version={}",
            AZURE_MANAGEMENT_URL, API_VERSION_SUBSCRIPTIONS
        );

        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| ServiceBusError::InternalError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ServiceBusError::InternalError(format!(
                "Failed to list subscriptions: {} - {}",
                status, error_text
            )));
        }

        let list_response: ListResponse<Subscription> = response
            .json()
            .await
            .map_err(|e| ServiceBusError::ConfigurationError(e.to_string()))?;

        Ok(list_response.value)
    }

    /// List all resource groups in a subscription
    pub async fn list_resource_groups(
        &self,
        token: &str,
        subscription_id: &str,
    ) -> Result<Vec<ResourceGroup>, ServiceBusError> {
        let url = format!(
            "{}/subscriptions/{}/resourcegroups?api-version={}",
            AZURE_MANAGEMENT_URL, subscription_id, API_VERSION_RESOURCE_GROUPS
        );

        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| ServiceBusError::InternalError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ServiceBusError::InternalError(format!(
                "Failed to list resource groups: {} - {}",
                status, error_text
            )));
        }

        let list_response: ListResponse<ResourceGroup> = response
            .json()
            .await
            .map_err(|e| ServiceBusError::ConfigurationError(e.to_string()))?;

        Ok(list_response.value)
    }

    /// List all Service Bus namespaces in a subscription
    pub async fn list_service_bus_namespaces(
        &self,
        token: &str,
        subscription_id: &str,
    ) -> Result<Vec<ServiceBusNamespace>, ServiceBusError> {
        let url = format!(
            "{}/subscriptions/{}/providers/Microsoft.ServiceBus/namespaces?api-version={}",
            AZURE_MANAGEMENT_URL, subscription_id, API_VERSION_SERVICE_BUS
        );

        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| ServiceBusError::InternalError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ServiceBusError::InternalError(format!(
                "Failed to list Service Bus namespaces: {} - {}",
                status, error_text
            )));
        }

        let list_response: ListResponse<ServiceBusNamespace> = response
            .json()
            .await
            .map_err(|e| ServiceBusError::ConfigurationError(e.to_string()))?;

        Ok(list_response.value)
    }

    /// Get the connection string for a Service Bus namespace
    pub async fn get_namespace_connection_string(
        &self,
        token: &str,
        subscription_id: &str,
        resource_group: &str,
        namespace: &str,
    ) -> Result<String, ServiceBusError> {
        // Try to get RootManageSharedAccessKey first
        let url = format!(
            "{}/subscriptions/{}/resourceGroups/{}/providers/Microsoft.ServiceBus/namespaces/{}/authorizationRules/RootManageSharedAccessKey/listKeys?api-version={}",
            AZURE_MANAGEMENT_URL,
            subscription_id,
            resource_group,
            namespace,
            API_VERSION_SERVICE_BUS
        );

        let response = self
            .client
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .header(CONTENT_TYPE, "application/json")
            .body("{}") // Empty JSON body required for Azure Management API POST requests
            .send()
            .await
            .map_err(|e| ServiceBusError::InternalError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ServiceBusError::InternalError(format!(
                "Failed to get namespace keys: {} - {}",
                status, error_text
            )));
        }

        let keys: AccessKeys = response
            .json()
            .await
            .map_err(|e| ServiceBusError::ConfigurationError(e.to_string()))?;

        Ok(keys.primary_connection_string)
    }

    /// Get the connection string for a Service Bus namespace using resource ID
    pub async fn get_namespace_connection_string_by_id(
        &self,
        token: &str,
        resource_id: &str,
    ) -> Result<String, ServiceBusError> {
        // Resource ID format: /subscriptions/{subscriptionId}/resourceGroups/{resourceGroupName}/providers/Microsoft.ServiceBus/namespaces/{namespaceName}
        let parts: Vec<&str> = resource_id.split('/').collect();

        if parts.len() < 9 {
            return Err(ServiceBusError::ConfigurationError(
                "Invalid resource ID format".to_string(),
            ));
        }

        let subscription_id = parts[2];
        let resource_group = parts[4];
        let namespace = parts[8];

        self.get_namespace_connection_string(token, subscription_id, resource_group, namespace)
            .await
    }

    /// List all queues in a Service Bus namespace
    pub async fn list_queues(
        &self,
        token: &str,
        subscription_id: &str,
        resource_group: &str,
        namespace: &str,
    ) -> Result<Vec<String>, ServiceBusError> {
        let url = format!(
            "{}/subscriptions/{}/resourceGroups/{}/providers/Microsoft.ServiceBus/namespaces/{}/queues?api-version={}",
            AZURE_MANAGEMENT_URL,
            subscription_id,
            resource_group,
            namespace,
            API_VERSION_SERVICE_BUS
        );

        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| ServiceBusError::InternalError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ServiceBusError::InternalError(format!(
                "Failed to list queues: {} - {}",
                status, error_text
            )));
        }

        let list_response: ListResponse<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| ServiceBusError::ConfigurationError(e.to_string()))?;

        let queue_names: Vec<String> = list_response
            .value
            .iter()
            .filter_map(|queue| queue["name"].as_str().map(|s| s.to_string()))
            .collect();

        Ok(queue_names)
    }
}

/// Cache for Azure resources to avoid repeated API calls
#[derive(Debug, Clone, Default)]
pub struct AzureResourceCache {
    pub subscriptions: Vec<Subscription>,
    pub resource_groups: std::collections::HashMap<String, Vec<ResourceGroup>>,
    pub namespaces: std::collections::HashMap<String, Vec<ServiceBusNamespace>>,
    pub connection_strings: std::collections::HashMap<String, String>,
}

impl AzureResourceCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cache_subscriptions(&mut self, subscriptions: Vec<Subscription>) {
        self.subscriptions = subscriptions;
    }

    pub fn cache_resource_groups(&mut self, subscription_id: String, groups: Vec<ResourceGroup>) {
        self.resource_groups.insert(subscription_id, groups);
    }

    pub fn cache_namespaces(
        &mut self,
        subscription_id: String,
        namespaces: Vec<ServiceBusNamespace>,
    ) {
        self.namespaces.insert(subscription_id, namespaces);
    }

    pub fn cache_connection_string(&mut self, namespace_id: String, connection_string: String) {
        self.connection_strings
            .insert(namespace_id, connection_string);
    }

    pub fn get_cached_connection_string(&self, namespace_id: &str) -> Option<&String> {
        self.connection_strings.get(namespace_id)
    }
}
