use super::azure_management_client::{AzureManagementClient, ManagementApiError, StatisticsConfig};
use super::types::QueueType;

/// Service for getting real queue statistics from Azure Management API
pub struct QueueStatisticsService {
    management_client: Option<AzureManagementClient>,
    config: StatisticsConfig,
}

impl QueueStatisticsService {
    /// Create a new queue statistics service
    pub fn new(config: StatisticsConfig, azure_ad_config: super::AzureAdConfig) -> Self {
        let management_client = if config.use_management_api {
            match AzureManagementClient::from_config(azure_ad_config) {
                Ok(client) => {
                    log::info!("Azure Management API client initialized successfully");
                    Some(client)
                }
                Err(e) => {
                    log::warn!(
                        "Failed to initialize Azure Management API client: {}. Queue statistics will not be available.",
                        e
                    );
                    None
                }
            }
        } else {
            log::info!("Azure Management API disabled in configuration");
            None
        };

        Self {
            management_client,
            config,
        }
    }

    /// Get real queue statistics from Azure Management API
    pub async fn get_queue_statistics(
        &self,
        queue_name: &str,
        queue_type: &QueueType,
    ) -> Option<u64> {
        if !self.config.display_enabled {
            log::debug!("Queue statistics display is disabled");
            return None;
        }

        let client = match &self.management_client {
            Some(client) => client,
            None => {
                log::debug!("Management API client not available");
                return None;
            }
        };

        // Fetch counts from management API for the main queue name
        log::info!(
            "Getting statistics for queue: {} (type: {:?})",
            queue_name,
            queue_type
        );

        match client.get_queue_counts(queue_name).await {
            Ok((active, dlq)) => {
                let count = match queue_type {
                    QueueType::Main => active,
                    QueueType::DeadLetter => dlq,
                };
                log::debug!(
                    "Retrieved counts - active: {}, dlq: {}. Returning {} for {:?} queue",
                    active,
                    dlq,
                    count,
                    queue_type
                );
                Some(count)
            }
            Err(ManagementApiError::QueueNotFound(_)) => {
                log::warn!("Queue not found: {}", queue_name);
                None
            }
            Err(ManagementApiError::AuthenticationFailed(msg)) => {
                log::warn!("Authentication failed for management API: {}", msg);
                None
            }
            Err(e) => {
                log::warn!("Failed to get queue statistics: {}", e);
                None
            }
        }
    }

    /// Check if the service is properly configured and ready
    pub fn is_available(&self) -> bool {
        self.config.display_enabled && self.management_client.is_some()
    }

    /// Get both active and dead letter counts from Azure Management API
    pub async fn get_both_queue_counts(&self, queue_name: &str) -> (Option<u64>, Option<u64>) {
        if !self.config.display_enabled {
            log::debug!("Queue statistics display is disabled");
            return (None, None);
        }

        let client = match &self.management_client {
            Some(client) => client,
            None => {
                log::debug!("Management API client not available");
                return (None, None);
            }
        };

        // Fetch counts from management API for the main queue name
        log::info!("Getting both counts for queue: {}", queue_name);

        match client.get_queue_counts(queue_name).await {
            Ok((active, dlq)) => {
                log::debug!("Retrieved counts - active: {}, dlq: {}", active, dlq);
                (Some(active), Some(dlq))
            }
            Err(ManagementApiError::QueueNotFound(_)) => {
                log::warn!("Queue not found: {}", queue_name);
                (None, None)
            }
            Err(ManagementApiError::AuthenticationFailed(msg)) => {
                log::warn!("Authentication failed for management API: {}", msg);
                (None, None)
            }
            Err(e) => {
                log::warn!("Failed to get queue statistics: {}", e);
                (None, None)
            }
        }
    }

    /// Get the current configuration
    pub fn config(&self) -> &StatisticsConfig {
        &self.config
    }
}

