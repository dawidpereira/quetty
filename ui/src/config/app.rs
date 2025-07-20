use super::{
    LoggingConfig, azure::ServicebusConfig, keys::KeyBindingsConfig, limits::*, ui::UIConfig,
    validation::ConfigValidationError,
};
use crate::constants::env_vars::*;
use crate::theme::types::ThemeConfig;
use crate::utils::auth::{
    AUTH_METHOD_CLIENT_SECRET, AUTH_METHOD_CONNECTION_STRING, AUTH_METHOD_DEVICE_CODE, AuthUtils,
};
use quetty_server::bulk_operations::BatchConfig;
use quetty_server::service_bus_manager::AzureAdConfig;
use serde::Deserialize;
use std::time::Duration;

/// Main application configuration
#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    page_size: Option<u32>,
    crossterm_input_listener_interval_ms: Option<u64>,
    crossterm_input_listener_retries: Option<usize>,
    poll_timeout_ms: Option<u64>,
    tick_interval_millis: Option<u64>,
    // Queue statistics configuration
    queue_stats_display_enabled: Option<bool>,
    queue_stats_cache_ttl_seconds: Option<u64>,
    queue_stats_use_management_api: Option<bool>,
    // Azure resource cache configuration
    azure_resource_cache_ttl_seconds: Option<u64>,
    azure_resource_cache_max_entries: Option<usize>,

    #[serde(flatten, default)]
    batch: BatchConfig,
    #[serde(flatten, default)]
    ui: UIConfig,
    #[serde(default)]
    keys: KeyBindingsConfig,
    #[serde(default)]
    servicebus: ServicebusConfig,
    #[serde(default)]
    azure_ad: AzureAdConfig,
    #[serde(default)]
    logging: LoggingConfig,
    theme: Option<ThemeConfig>,
}

impl AppConfig {
    /// Validate the configuration against defined limits
    pub fn validate(&self) -> Result<(), Vec<ConfigValidationError>> {
        let mut errors = Vec::new();

        // Check page size limits
        let page_size = self.page_size();
        if page_size < MIN_PAGE_SIZE {
            errors.push(ConfigValidationError::PageSize {
                configured: page_size,
                min_limit: MIN_PAGE_SIZE,
                max_limit: MAX_PAGE_SIZE,
            });
        }
        if page_size > MAX_PAGE_SIZE {
            errors.push(ConfigValidationError::PageSize {
                configured: page_size,
                min_limit: MIN_PAGE_SIZE,
                max_limit: MAX_PAGE_SIZE,
            });
        }

        // Check batch configuration limits
        if self.batch.max_batch_size() > AZURE_SERVICE_BUS_MAX_BATCH_SIZE {
            errors.push(ConfigValidationError::BatchSize {
                configured: self.batch.max_batch_size(),
                limit: AZURE_SERVICE_BUS_MAX_BATCH_SIZE,
            });
        }

        if self.batch.operation_timeout_secs() > MAX_OPERATION_TIMEOUT_SECS {
            errors.push(ConfigValidationError::OperationTimeout {
                configured: self.batch.operation_timeout_secs(),
                limit: MAX_OPERATION_TIMEOUT_SECS,
            });
        }

        if self.batch.bulk_chunk_size() > MAX_BULK_CHUNK_SIZE {
            errors.push(ConfigValidationError::BulkChunkSize {
                configured: self.batch.bulk_chunk_size(),
                limit: MAX_BULK_CHUNK_SIZE,
            });
        }

        if self.batch.bulk_processing_time_secs() > MAX_BULK_PROCESSING_TIME_SECS {
            errors.push(ConfigValidationError::BulkProcessingTime {
                configured: self.batch.bulk_processing_time_secs(),
                limit: MAX_BULK_PROCESSING_TIME_SECS,
            });
        }

        if self.batch.lock_timeout_secs() > MAX_LOCK_TIMEOUT_SECS {
            errors.push(ConfigValidationError::LockTimeout {
                configured: self.batch.lock_timeout_secs(),
                limit: MAX_LOCK_TIMEOUT_SECS,
            });
        }

        if self.batch.max_messages_to_process() > MAX_MESSAGES_TO_PROCESS_LIMIT {
            errors.push(ConfigValidationError::MaxMessagesToProcess {
                configured: self.batch.max_messages_to_process(),
                limit: MAX_MESSAGES_TO_PROCESS_LIMIT,
            });
        }

        // Validate queue statistics cache TTL
        let ttl = self.queue_stats_cache_ttl_seconds();
        if !(MIN_QUEUE_STATS_CACHE_TTL_SECONDS..=MAX_QUEUE_STATS_CACHE_TTL_SECONDS).contains(&ttl) {
            errors.push(ConfigValidationError::QueueStatsCacheTtl {
                configured: ttl,
                min_limit: MIN_QUEUE_STATS_CACHE_TTL_SECONDS,
                max_limit: MAX_QUEUE_STATS_CACHE_TTL_SECONDS,
            });
        }

        // Validate authentication configuration
        self.validate_auth_config(&mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    // App-specific configuration accessors
    pub fn page_size(&self) -> u32 {
        self.page_size.unwrap_or(100)
    }

    // Backward compatibility - max_messages now refers to page_size
    pub fn max_messages(&self) -> u32 {
        self.page_size()
    }

    pub fn crossterm_input_listener_interval(&self) -> Duration {
        Duration::from_millis(self.crossterm_input_listener_interval_ms.unwrap_or(10))
    }

    pub fn crossterm_input_listener_retries(&self) -> usize {
        self.crossterm_input_listener_retries.unwrap_or(10)
    }

    pub fn poll_timeout(&self) -> Duration {
        Duration::from_millis(self.poll_timeout_ms.unwrap_or(50))
    }

    pub fn tick_interval(&self) -> Duration {
        Duration::from_millis(self.tick_interval_millis.unwrap_or(50))
    }

    // Queue statistics configuration accessors
    pub fn queue_stats_display_enabled(&self) -> bool {
        self.queue_stats_display_enabled.unwrap_or(true)
    }

    pub fn queue_stats_cache_ttl_seconds(&self) -> u64 {
        self.queue_stats_cache_ttl_seconds.unwrap_or(60)
    }

    pub fn queue_stats_use_management_api(&self) -> bool {
        self.queue_stats_use_management_api.unwrap_or(true)
    }

    pub fn azure_resource_cache_ttl_seconds(&self) -> u64 {
        self.azure_resource_cache_ttl_seconds.unwrap_or(300) // 5 minutes default
    }

    pub fn azure_resource_cache_max_entries(&self) -> usize {
        self.azure_resource_cache_max_entries.unwrap_or(100) // 100 entries default
    }

    // Configuration section accessors
    pub fn batch(&self) -> &BatchConfig {
        &self.batch
    }

    pub fn ui(&self) -> &UIConfig {
        &self.ui
    }

    pub fn keys(&self) -> &KeyBindingsConfig {
        &self.keys
    }

    pub fn servicebus(&self) -> &ServicebusConfig {
        &self.servicebus
    }

    pub fn azure_ad(&self) -> &AzureAdConfig {
        &self.azure_ad
    }

    pub fn logging(&self) -> &LoggingConfig {
        &self.logging
    }

    pub fn theme(&self) -> ThemeConfig {
        self.theme.clone().unwrap_or_default()
    }

    /// Validate authentication configuration
    fn validate_auth_config(&self, errors: &mut Vec<ConfigValidationError>) {
        let auth_method = &self.azure_ad.auth_method;

        // Validate authentication method
        match auth_method.as_str() {
            AUTH_METHOD_CONNECTION_STRING => {
                // When using connection_string, ensure we have an encrypted connection string
                if !self.servicebus.has_connection_string() {
                    errors.push(ConfigValidationError::ConflictingAuthConfig {
                        message: "Authentication method is set to 'connection_string' but no encrypted Service Bus connection string is provided.\n\n\
                                  Please either:\n\
                                  1. Add servicebus.encrypted_connection_string and servicebus.encryption_salt to your config.toml\n\
                                  2. Set SERVICEBUS__ENCRYPTED_CONNECTION_STRING and SERVICEBUS__ENCRYPTION_SALT environment variables\n\
                                  3. Change azure_ad.auth_method to 'device_code' for Azure AD authentication".to_string()
                    });
                }
            }
            AUTH_METHOD_DEVICE_CODE | AUTH_METHOD_CLIENT_SECRET => {
                // Validate Azure AD configuration for these auth methods
                self.validate_azure_ad_config(errors);
            }
            method => {
                errors.push(ConfigValidationError::InvalidAuthMethod {
                    method: method.to_string(),
                });
            }
        }
    }

    /// Validate Azure AD specific configuration
    fn validate_azure_ad_config(&self, errors: &mut Vec<ConfigValidationError>) {
        let auth_method = &self.azure_ad.auth_method;

        // Validate authentication method
        match auth_method.as_str() {
            AUTH_METHOD_DEVICE_CODE => self.validate_device_code_config(errors),
            AUTH_METHOD_CLIENT_SECRET => self.validate_client_secret_config(errors),
            _ => {
                errors.push(ConfigValidationError::InvalidAzureAdFlow {
                    flow: auth_method.clone(),
                });
            }
        }
    }

    /// Validate device code flow configuration
    fn validate_device_code_config(&self, errors: &mut Vec<ConfigValidationError>) {
        // Required fields for device code flow
        if !self.azure_ad.has_tenant_id() && std::env::var(AZURE_AD_TENANT_ID).is_err() {
            errors.push(ConfigValidationError::MissingAzureAdField {
                field: "tenant_id".to_string(),
            });
        }
        if !self.azure_ad.has_client_id() && std::env::var(AZURE_AD_CLIENT_ID).is_err() {
            errors.push(ConfigValidationError::MissingAzureAdField {
                field: "client_id".to_string(),
            });
        }

        // Optional fields for management API operations
        if self.queue_stats_use_management_api() {
            // These are optional with device code flow as they can be discovered interactively
            log::debug!(
                "Device code flow with management API - optional fields can be discovered interactively"
            );
        }
    }

    /// Validate client secret flow configuration
    fn validate_client_secret_config(&self, errors: &mut Vec<ConfigValidationError>) {
        // Required fields for client secret flow
        if !self.azure_ad.has_tenant_id() && std::env::var(AZURE_AD_TENANT_ID).is_err() {
            errors.push(ConfigValidationError::MissingAzureAdField {
                field: "tenant_id".to_string(),
            });
        }
        if !self.azure_ad.has_client_id() && std::env::var(AZURE_AD_CLIENT_ID).is_err() {
            errors.push(ConfigValidationError::MissingAzureAdField {
                field: "client_id".to_string(),
            });
        }
        if !self.azure_ad.has_client_secret()
            && std::env::var(AZURE_AD_CLIENT_SECRET).is_err()
            && std::env::var(AZURE_AD_ENCRYPTED_CLIENT_SECRET).is_err()
        {
            errors.push(ConfigValidationError::MissingAzureAdField {
                field: "client_secret".to_string(),
            });
        }

        // Optional fields for management API operations
        if self.queue_stats_use_management_api() {
            // These are optional with client secret flow as they can be discovered interactively
            log::debug!(
                "Client secret flow with management API - optional fields can be discovered interactively"
            );
        }
    }

    /// Check if all required fields are present for the configured authentication method
    pub fn has_required_auth_fields(&self) -> bool {
        if AuthUtils::is_connection_string_auth(self) {
            self.servicebus.has_connection_string()
        } else if AuthUtils::is_device_code_auth(self) {
            (self.azure_ad.has_tenant_id() || std::env::var(AZURE_AD_TENANT_ID).is_ok())
                && (self.azure_ad.has_client_id() || std::env::var(AZURE_AD_CLIENT_ID).is_ok())
        } else if AuthUtils::is_client_secret_auth(self) {
            (self.azure_ad.has_tenant_id() || std::env::var(AZURE_AD_TENANT_ID).is_ok())
                && (self.azure_ad.has_client_id() || std::env::var(AZURE_AD_CLIENT_ID).is_ok())
                && (self.azure_ad.has_client_secret()
                    || std::env::var(AZURE_AD_CLIENT_SECRET).is_ok()
                    || std::env::var(AZURE_AD_ENCRYPTED_CLIENT_SECRET).is_ok())
        } else {
            false
        }
    }

    /// Get a list of encrypted authentication methods that require password decryption
    /// Only returns methods relevant to the current authentication method
    pub fn get_encrypted_auth_methods(&self) -> Vec<String> {
        let mut methods = Vec::new();
        let auth_method = &self.azure_ad().auth_method;

        // Only include connection string if using connection_string auth method
        if auth_method == "connection_string"
            && std::env::var(SERVICEBUS_ENCRYPTED_CONNECTION_STRING).is_ok()
            && std::env::var(SERVICEBUS_ENCRYPTION_SALT).is_ok()
        {
            methods.push("Connection String".to_string());
        }

        // Only include client secret if using client_secret auth method
        if auth_method == "client_secret"
            && std::env::var(AZURE_AD_ENCRYPTED_CLIENT_SECRET).is_ok()
            && std::env::var(AZURE_AD_CLIENT_SECRET_ENCRYPTION_SALT).is_ok()
        {
            methods.push("Azure AD Client Secret".to_string());
        }

        methods
    }
}
