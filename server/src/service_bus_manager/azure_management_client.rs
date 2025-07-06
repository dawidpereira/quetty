use super::{AzureAdConfig, ServiceBusError};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Azure Management API client for discovering Azure resources and managing Service Bus operations.
/// This client is used when authentication is done via Azure AD (device code flow).
const AZURE_MANAGEMENT_URL: &str = "https://management.azure.com";
const API_VERSION_SUBSCRIPTIONS: &str = "2022-12-01";
const API_VERSION_RESOURCE_GROUPS: &str = "2021-04-01";
const API_VERSION_SERVICE_BUS: &str = "2021-11-01";

#[derive(Debug, Clone)]
pub struct AzureManagementClient {
    client: reqwest::Client,
    /// Optional Azure AD configuration for operations that need persistent config
    azure_ad_config: Option<AzureAdConfig>,
}

// Resource discovery types
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

// Queue statistics types
#[derive(Debug, Deserialize)]
struct QueuePropertiesResponse {
    properties: QueueProperties,
}

#[derive(Debug, Deserialize)]
struct QueueProperties {
    #[serde(rename = "countDetails")]
    count_details: CountDetails,
}

#[derive(Debug, Deserialize)]
struct CountDetails {
    #[serde(rename = "activeMessageCount")]
    active_message_count: i64,
    #[serde(rename = "deadLetterMessageCount")]
    dead_letter_message_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListResponse<T> {
    value: Vec<T>,
    #[serde(rename = "nextLink")]
    next_link: Option<String>,
}

impl AzureManagementClient {
    /// Create a new client for general operations (without persistent config)
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            azure_ad_config: None,
        }
    }

    /// Create a new client with Azure AD configuration for authenticated operations
    pub fn with_config(azure_ad_config: AzureAdConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            azure_ad_config: Some(azure_ad_config),
        }
    }

    /// Create a client from configuration (for backward compatibility)
    pub fn from_config(azure_ad_config: AzureAdConfig) -> Result<Self, ServiceBusError> {
        // Validate required fields when using from_config
        azure_ad_config.subscription_id()?;
        azure_ad_config.resource_group()?;
        azure_ad_config.namespace()?;
        
        Ok(Self::with_config(azure_ad_config))
    }

    /// Get access token from Azure AD config if available
    async fn get_management_api_token(&self) -> Result<String, ServiceBusError> {
        match &self.azure_ad_config {
            Some(config) => config.get_azure_ad_token().await
                .map_err(|e| ServiceBusError::AuthenticationError(e.to_string())),
            None => Err(ServiceBusError::ConfigurationError(
                "Azure AD configuration not available for this operation".to_string(),
            )),
        }
    }

    // ===== Resource Discovery Operations =====

    /// List all subscriptions accessible to the authenticated user
    pub async fn list_subscriptions(
        &self,
        token: &str,
    ) -> Result<Vec<Subscription>, ServiceBusError> {
        let url = format!(
            "{AZURE_MANAGEMENT_URL}/subscriptions?api-version={API_VERSION_SUBSCRIPTIONS}"
        );

        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await
            .map_err(|e| ServiceBusError::InternalError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ServiceBusError::InternalError(format!(
                "Failed to list subscriptions: {status} - {error_text}"
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
            "{AZURE_MANAGEMENT_URL}/subscriptions/{subscription_id}/resourcegroups?api-version={API_VERSION_RESOURCE_GROUPS}"
        );

        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await
            .map_err(|e| ServiceBusError::InternalError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ServiceBusError::InternalError(format!(
                "Failed to list resource groups: {status} - {error_text}"
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
            "{AZURE_MANAGEMENT_URL}/subscriptions/{subscription_id}/providers/Microsoft.ServiceBus/namespaces?api-version={API_VERSION_SERVICE_BUS}"
        );

        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await
            .map_err(|e| ServiceBusError::InternalError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ServiceBusError::InternalError(format!(
                "Failed to list Service Bus namespaces: {status} - {error_text}"
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
            "{AZURE_MANAGEMENT_URL}/subscriptions/{subscription_id}/resourceGroups/{resource_group}/providers/Microsoft.ServiceBus/namespaces/{namespace}/authorizationRules/RootManageSharedAccessKey/listKeys?api-version={API_VERSION_SERVICE_BUS}"
        );

        let response = self
            .client
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .header(CONTENT_TYPE, "application/json")
            .body("{}") // Empty JSON body required for Azure Management API POST requests
            .send()
            .await
            .map_err(|e| ServiceBusError::InternalError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ServiceBusError::InternalError(format!(
                "Failed to get namespace keys: {status} - {error_text}"
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
            "{AZURE_MANAGEMENT_URL}/subscriptions/{subscription_id}/resourceGroups/{resource_group}/providers/Microsoft.ServiceBus/namespaces/{namespace}/queues?api-version={API_VERSION_SERVICE_BUS}"
        );

        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .send()
            .await
            .map_err(|e| ServiceBusError::InternalError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ServiceBusError::InternalError(format!(
                "Failed to list queues: {status} - {error_text}"
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

    // ===== Queue Statistics Operations =====

    /// Get the actual message count for a queue from Azure Management API
    pub async fn get_queue_message_count(
        &self,
        queue_name: &str,
    ) -> Result<u64, ServiceBusError> {
        let (active_count, _) = self.get_queue_counts(queue_name).await?;
        Ok(active_count)
    }

    /// Get both active and dead-letter counts from Azure Management API
    pub async fn get_queue_counts(
        &self,
        queue_name: &str,
    ) -> Result<(u64, u64), ServiceBusError> {
        self.get_queue_counts_with_retry(queue_name, 3).await
    }

    /// Get queue counts with retry logic for transient failures
    async fn get_queue_counts_with_retry(
        &self,
        queue_name: &str,
        max_retries: u32,
    ) -> Result<(u64, u64), ServiceBusError> {
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match self.get_queue_counts_internal(queue_name).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);

                    // Don't retry on authentication or configuration errors
                    if let Some(ref err) = last_error {
                        match err {
                            ServiceBusError::ConfigurationError(_)
                            | ServiceBusError::AuthenticationError(_) => {
                                log::debug!("Non-retryable error, failing immediately: {err}");
                                return Err(last_error.unwrap());
                            }
                            ServiceBusError::InternalError(msg) if msg.contains("404") => {
                                return Err(ServiceBusError::InternalError(format!(
                                    "Queue not found: {queue_name}"
                                )));
                            }
                            _ => {}
                        }
                    }

                    if attempt < max_retries {
                        let delay = Duration::from_millis(100 * (2_u64.pow(attempt))); // Exponential backoff
                        log::debug!(
                            "Attempt {} failed, retrying in {:?}: {}",
                            attempt + 1,
                            delay,
                            last_error.as_ref().unwrap()
                        );
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }

    /// Internal implementation for getting queue counts (single attempt)
    async fn get_queue_counts_internal(
        &self,
        queue_name: &str,
    ) -> Result<(u64, u64), ServiceBusError> {
        log::debug!("Getting queue counts for: {queue_name}");

        // Get configuration from Azure AD config
        let config = self.azure_ad_config.as_ref().ok_or_else(|| {
            ServiceBusError::ConfigurationError(
                "Azure AD configuration required for queue statistics".to_string(),
            )
        })?;

        let subscription_id = config.subscription_id()?;
        let resource_group = config.resource_group()?;
        let namespace = config.namespace()?;

        // Get access token
        let access_token = self.get_management_api_token().await?;

        // Build the management API URL with encoded queue name
        let encoded_queue_name = urlencoding::encode(queue_name);
        let url = format!(
            "{AZURE_MANAGEMENT_URL}/subscriptions/{subscription_id}/resourceGroups/{resource_group}/providers/Microsoft.ServiceBus/namespaces/{namespace}/queues/{encoded_queue_name}?api-version={API_VERSION_SERVICE_BUS}"
        );

        log::debug!("Requesting queue properties from Azure Management API: {url}");

        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {access_token}"))
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await
            .map_err(|e| ServiceBusError::InternalError(format!("HTTP request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            if status == 404 {
                return Err(ServiceBusError::InternalError(format!(
                    "Queue not found: {queue_name}"
                )));
            }

            return Err(ServiceBusError::InternalError(format!(
                "API request failed with status {status}: {error_text}"
            )));
        }

        let response_text = response.text().await.map_err(|e| {
            ServiceBusError::InternalError(format!("Failed to read response: {e}"))
        })?;

        let queue_response: QueuePropertiesResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                ServiceBusError::ConfigurationError(format!("Failed to parse JSON: {e}"))
            })?;

        let active_raw = queue_response.properties.count_details.active_message_count;
        let dlq_raw = queue_response
            .properties
            .count_details
            .dead_letter_message_count;

        let active = if active_raw < 0 { 0 } else { active_raw as u64 };
        let dlq = if dlq_raw < 0 { 0 } else { dlq_raw as u64 };

        Ok((active, dlq))
    }
}

impl Default for AzureManagementClient {
    fn default() -> Self {
        Self::new()
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

/// Configuration for queue statistics
#[derive(Debug, Clone)]
pub struct StatisticsConfig {
    pub display_enabled: bool,
    pub cache_ttl_seconds: u64,
    pub use_management_api: bool,
}

impl StatisticsConfig {
    pub fn new(display_enabled: bool, cache_ttl_seconds: u64, use_management_api: bool) -> Self {
        Self {
            display_enabled,
            cache_ttl_seconds,
            use_management_api,
        }
    }
}