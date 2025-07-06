use super::ServiceBusError;
use super::azure_management_client::{AzureManagementClient, StatisticsConfig};
use super::types::QueueType;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Service for getting real queue statistics from Azure Management API
pub struct QueueStatisticsService {
    management_client: Arc<Mutex<Option<AzureManagementClient>>>,
    config: StatisticsConfig,
    azure_ad_config: super::AzureAdConfig,
    initialized: Arc<Mutex<bool>>,
    http_client: reqwest::Client,
}

impl QueueStatisticsService {
    /// Create a new queue statistics service
    pub fn new(
        http_client: reqwest::Client,
        config: StatisticsConfig,
        azure_ad_config: super::AzureAdConfig,
    ) -> Self {
        Self {
            management_client: Arc::new(Mutex::new(None)),
            config,
            azure_ad_config,
            initialized: Arc::new(Mutex::new(false)),
            http_client,
        }
    }

    /// Initialize the management client lazily on first use
    async fn ensure_initialized(&self) {
        let mut initialized = self.initialized.lock().await;
        if *initialized {
            return;
        }

        if self.config.use_management_api {
            match AzureManagementClient::from_config(
                self.http_client.clone(),
                self.azure_ad_config.clone(),
            ) {
                Ok(client) => {
                    log::info!("Azure Management API client initialized successfully");
                    let mut client_lock = self.management_client.lock().await;
                    *client_lock = Some(client);
                }
                Err(e) => {
                    log::warn!(
                        "Failed to initialize Azure Management API client: {e}. Queue statistics will not be available.",
                    );
                }
            }
        } else {
            log::info!("Azure Management API disabled in configuration");
        }

        *initialized = true;
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

        // Ensure the client is initialized
        self.ensure_initialized().await;

        let client_lock = self.management_client.lock().await;
        let client = match &*client_lock {
            Some(client) => client,
            None => {
                log::debug!("Management API client not available");
                return None;
            }
        };

        // Fetch counts from management API for the main queue name
        log::info!("Getting statistics for queue: {queue_name} (type: {queue_type:?})");

        match client.get_queue_counts(queue_name).await {
            Ok((active, dlq)) => {
                let count = match queue_type {
                    QueueType::Main => active,
                    QueueType::DeadLetter => dlq,
                };
                log::debug!(
                    "Retrieved counts - active: {active}, dlq: {dlq}. Returning {count} for {queue_type:?} queue"
                );
                Some(count)
            }
            Err(ServiceBusError::InternalError(msg)) if msg.contains("Queue not found") => {
                log::warn!("Queue not found: {queue_name}");
                None
            }
            Err(ServiceBusError::AuthenticationError(msg)) => {
                log::warn!("Authentication failed for management API: {msg}");
                None
            }
            Err(e) => {
                log::warn!("Failed to get queue statistics: {e}");
                None
            }
        }
    }

    /// Check if the service is properly configured and ready
    pub async fn is_available(&self) -> bool {
        if !self.config.display_enabled {
            return false;
        }

        // Check if we have a client after initialization
        self.ensure_initialized().await;
        let client_lock = self.management_client.lock().await;
        client_lock.is_some()
    }

    /// Get both active and dead letter counts from Azure Management API
    pub async fn get_both_queue_counts(&self, queue_name: &str) -> (Option<u64>, Option<u64>) {
        if !self.config.display_enabled {
            log::debug!("Queue statistics display is disabled");
            return (None, None);
        }

        // Ensure the client is initialized
        self.ensure_initialized().await;

        let client_lock = self.management_client.lock().await;
        let client = match &*client_lock {
            Some(client) => client,
            None => {
                log::debug!("Management API client not available");
                return (None, None);
            }
        };

        // Fetch counts from management API for the main queue name
        log::info!("Getting both counts for queue: {queue_name}");

        match client.get_queue_counts(queue_name).await {
            Ok((active, dlq)) => {
                log::debug!("Retrieved counts - active: {active}, dlq: {dlq}");
                (Some(active), Some(dlq))
            }
            Err(ServiceBusError::InternalError(msg)) if msg.contains("Queue not found") => {
                log::warn!("Queue not found: {queue_name}");
                (None, None)
            }
            Err(ServiceBusError::AuthenticationError(msg)) => {
                log::warn!("Authentication failed for management API: {msg}");
                (None, None)
            }
            Err(e) => {
                log::warn!("Failed to get queue statistics: {e}");
                (None, None)
            }
        }
    }

    /// Get the current configuration
    pub fn config(&self) -> &StatisticsConfig {
        &self.config
    }
}
