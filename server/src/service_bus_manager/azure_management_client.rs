use super::AzureAdConfig;
use serde::Deserialize;
use std::time::Duration;

/// Error types for Azure Management API operations
#[derive(Debug, thiserror::Error)]
pub enum ManagementApiError {
    #[error("Management API not configured")]
    NotConfigured,
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),
    #[error("Queue not found: {0}")]
    QueueNotFound(String),
    #[error("JSON parsing failed: {0}")]
    JsonParsingFailed(String),
    #[error("Missing required configuration: {0}")]
    MissingConfiguration(String),
}

/// Response structure for Azure Management API queue properties
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

/// Azure Management API client for getting real queue statistics
pub struct AzureManagementClient {
    subscription_id: String,
    resource_group_name: String,
    namespace_name: String,
    http_client: reqwest::Client,
    azure_ad_config: AzureAdConfig,
}

impl AzureManagementClient {
    /// Create a new Azure Management API client
    pub fn new(
        subscription_id: String,
        resource_group_name: String,
        namespace_name: String,
        azure_ad_config: AzureAdConfig,
    ) -> Self {
        let http_client = reqwest::Client::new();

        Self {
            subscription_id,
            resource_group_name,
            namespace_name,
            http_client,
            azure_ad_config,
        }
    }

    /// Get the actual message count for a queue from Azure Management API
    pub async fn get_queue_message_count(
        &self,
        queue_name: &str,
    ) -> Result<u64, ManagementApiError> {
        // Use the retry logic from get_queue_counts and extract just the active count
        let (active_count, _) = self.get_queue_counts_with_retry(queue_name, 3).await?;
        Ok(active_count)
    }

    /// Get access token for Azure Management API
    async fn get_management_api_token(&self) -> Result<String, ManagementApiError> {
        // Use the existing Azure AD configuration to get a token with management scope
        match self.azure_ad_config.get_azure_ad_token().await {
            Ok(token) => Ok(token),
            Err(e) => Err(ManagementApiError::AuthenticationFailed(format!(
                "Failed to get Azure AD token: {}",
                e
            ))),
        }
    }

    /// Create a new client from environment or configuration
    pub fn from_config(azure_ad_config: AzureAdConfig) -> Result<Self, ManagementApiError> {
        // Use the existing Azure AD configuration
        let subscription_id = azure_ad_config
            .subscription_id()
            .map_err(|e| ManagementApiError::MissingConfiguration(e.to_string()))?
            .to_string();

        let resource_group_name = azure_ad_config
            .resource_group()
            .map_err(|e| ManagementApiError::MissingConfiguration(e.to_string()))?
            .to_string();

        let namespace_name = azure_ad_config
            .namespace()
            .map_err(|e| ManagementApiError::MissingConfiguration(e.to_string()))?
            .to_string();

        Ok(Self::new(
            subscription_id,
            resource_group_name,
            namespace_name,
            azure_ad_config,
        ))
    }

    /// Get both active and dead-letter counts from Azure Management API
    pub async fn get_queue_counts(
        &self,
        queue_name: &str,
    ) -> Result<(u64, u64), ManagementApiError> {
        self.get_queue_counts_with_retry(queue_name, 3).await
    }

    /// Get queue counts with retry logic for transient failures
    async fn get_queue_counts_with_retry(
        &self,
        queue_name: &str,
        max_retries: u32,
    ) -> Result<(u64, u64), ManagementApiError> {
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match self.get_queue_counts_internal(queue_name).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);

                    // Don't retry on authentication or queue not found errors
                    if let Some(ref err) = last_error {
                        match err {
                            ManagementApiError::AuthenticationFailed(_)
                            | ManagementApiError::QueueNotFound(_)
                            | ManagementApiError::NotConfigured
                            | ManagementApiError::MissingConfiguration(_) => {
                                log::debug!("Non-retryable error, failing immediately: {}", err);
                                return Err(last_error.unwrap());
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
    ) -> Result<(u64, u64), ManagementApiError> {
        // Reuse existing logic to fetch queue properties
        log::debug!("Getting queue counts for: {}", queue_name);

        // Get access token for management API
        let access_token = self.get_management_api_token().await?;

        // Build the management API URL with encoded queue name
        let encoded_queue_name = urlencoding::encode(queue_name);
        let url = format!(
            "https://management.azure.com/subscriptions/{}/resourceGroups/{}/providers/Microsoft.ServiceBus/namespaces/{}/queues/{}?api-version=2021-11-01",
            self.subscription_id, self.resource_group_name, self.namespace_name, encoded_queue_name
        );

        log::debug!(
            "Requesting queue properties from Azure Management API: {}",
            url
        );

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| {
                ManagementApiError::RequestFailed(format!("HTTP request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            if status == 404 {
                return Err(ManagementApiError::QueueNotFound(queue_name.to_string()));
            }

            return Err(ManagementApiError::RequestFailed(format!(
                "API request failed with status {}: {}",
                status, error_text
            )));
        }

        let response_text = response.text().await.map_err(|e| {
            ManagementApiError::RequestFailed(format!("Failed to read response: {}", e))
        })?;

        let queue_response: QueuePropertiesResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                ManagementApiError::JsonParsingFailed(format!("Failed to parse JSON: {}", e))
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
