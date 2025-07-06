use server::service_bus_manager::azure_management_client::{
    AzureManagementClient, StatisticsConfig,
};
use server::service_bus_manager::{AzureAdConfig, ServiceBusError};
use tokio::time::Duration;

// Helper module for server-side integration tests
mod azure_helpers {
    use super::*;

    /// Create a mock Azure AD config for server testing
    pub fn create_mock_server_azure_config() -> AzureAdConfig {
        serde_json::from_str(
            r#"{
            "tenant_id": "test-tenant-id-server",
            "client_id": "test-client-id-server", 
            "client_secret": "test-client-secret-server",
            "subscription_id": "test-subscription-id-server",
            "resource_group": "test-resource-group-server",
            "namespace": "test-namespace-server"
        }"#,
        )
        .expect("Failed to create mock Azure AD config for server tests")
    }

    /// Create a mock Azure AD config with missing fields
    pub fn create_incomplete_azure_config() -> AzureAdConfig {
        serde_json::from_str(
            r#"{
            "tenant_id": "test-tenant-id"
        }"#,
        )
        .expect("Failed to create incomplete Azure AD config")
    }
}

use azure_helpers::*;

// Integration tests for AzureManagementClient creation and configuration
mod azure_management_client_creation {
    use super::*;

    #[test]
    fn test_azure_management_client_creation_with_valid_config() {
        let azure_config = create_mock_server_azure_config();

        // Test that client creation succeeds with valid config containing all required fields
        let http_client = reqwest::Client::new();
        let result = AzureManagementClient::from_config(http_client, azure_config);

        // With complete mock data containing all required fields, creation should succeed
        assert!(
            result.is_ok(),
            "Client creation should succeed with complete Azure configuration"
        );
    }

    #[test]
    fn test_azure_management_client_creation_with_missing_fields() {
        let azure_config = create_incomplete_azure_config();

        // Test that client creation fails gracefully with incomplete config
        let http_client = reqwest::Client::new();
        let result = AzureManagementClient::from_config(http_client, azure_config);

        assert!(
            result.is_err(),
            "Client creation should fail with incomplete config"
        );

        // Verify the error type
        match result {
            Err(ServiceBusError::ConfigurationError(_)) => {
                // Expected error type
            }
            Err(other) => panic!("Expected ConfigurationError, got: {other:?}"),
            Ok(_) => panic!("Expected error, got success"),
        }
    }

    #[test]
    fn test_azure_management_client_direct_creation() {
        let azure_config = create_mock_server_azure_config();

        // Test client creation with config
        let http_client = reqwest::Client::new();
        let _client = AzureManagementClient::with_config(http_client, azure_config);

        // Client creation should always succeed (it's just storing the values)
        // Actual API calls will fail with mock data, but that's expected
    }
}

// Integration tests for Azure Management API error handling
mod error_handling_integration {
    use super::*;

    #[test]
    fn test_service_bus_error_display() {
        let errors = vec![
            ServiceBusError::ConfigurationError("Not configured".to_string()),
            ServiceBusError::AuthenticationFailed("Invalid token".to_string()),
            ServiceBusError::InternalError("Network error".to_string()),
            ServiceBusError::QueueNotFound("test-queue".to_string()),
            ServiceBusError::InternalError("Invalid JSON".to_string()),
            ServiceBusError::ConfigurationError("Missing field".to_string()),
        ];

        for error in errors {
            let error_string = error.to_string();
            assert!(
                !error_string.is_empty(),
                "Error display should not be empty"
            );

            // Verify error messages contain expected content
            match error {
                ServiceBusError::ConfigurationError(msg) => {
                    assert!(error_string.contains("Configuration error"));
                    assert!(error_string.contains(&msg));
                }
                ServiceBusError::AuthenticationFailed(msg) => {
                    assert!(error_string.contains("Authentication failed"));
                    assert!(error_string.contains(&msg));
                }
                ServiceBusError::InternalError(msg) => {
                    assert!(error_string.contains("Internal error"));
                    assert!(error_string.contains(&msg));
                }
                ServiceBusError::QueueNotFound(queue) => {
                    assert!(error_string.contains("Queue not found"));
                    assert!(error_string.contains(&queue));
                }
                _ => {}
            }
        }
    }

    #[tokio::test]
    async fn test_azure_client_graceful_error_handling() {
        let azure_config = create_mock_server_azure_config();
        // from_config will fail with incomplete config, so use with_config instead
        let http_client = reqwest::Client::new();
        let client = AzureManagementClient::with_config(http_client, azure_config);

        // Test that API calls fail gracefully with mock credentials
        let result = client.get_queue_message_count("test-queue").await;

        // We expect this to fail with mock credentials, but it should be a proper error
        assert!(result.is_err(), "Mock credentials should result in error");

        // The error should be authentication failure or internal error
        match result {
            Err(ServiceBusError::AuthenticationFailed(_))
            | Err(ServiceBusError::AuthenticationError(_))
            | Err(ServiceBusError::InternalError(_)) => {
                // Expected error types for mock credentials
            }
            Err(_) => {
                // Other errors are also acceptable (e.g., network issues)
            }
            Ok(_) => panic!("Expected error with mock credentials"),
        }
    }

    #[tokio::test]
    async fn test_azure_client_both_counts_error_handling() {
        let azure_config = create_mock_server_azure_config();
        // from_config will fail with incomplete config, so use with_config instead
        let http_client = reqwest::Client::new();
        let client = AzureManagementClient::with_config(http_client, azure_config);

        // Test both counts API with mock credentials
        let result = client.get_queue_counts("test-queue").await;

        // Should fail gracefully
        assert!(result.is_err(), "Mock credentials should result in error");
    }
}

// Integration tests for StatisticsConfig
mod statistics_config_integration {
    use super::*;

    #[test]
    fn test_statistics_config_creation() {
        let config = StatisticsConfig::new(true, 120, true);

        assert!(config.display_enabled);
        assert_eq!(config.cache_ttl_seconds, 120);
        assert!(config.use_management_api);
    }

    #[test]
    fn test_statistics_config_variations() {
        // Test different configuration combinations
        let configs = vec![
            (true, 30, true),   // Enabled with short TTL
            (false, 60, true),  // Disabled display
            (true, 300, false), // Enabled but no API
            (false, 60, false), // Fully disabled
        ];

        for (display, ttl, use_api) in configs {
            let config = StatisticsConfig::new(display, ttl, use_api);
            assert_eq!(config.display_enabled, display);
            assert_eq!(config.cache_ttl_seconds, ttl);
            assert_eq!(config.use_management_api, use_api);
        }
    }
}

// Integration tests for retry logic and resilience
mod resilience_integration {
    use super::*;

    #[tokio::test]
    async fn test_retry_logic_timeout() {
        let azure_config = create_mock_server_azure_config();
        // from_config will fail with incomplete config, so use with_config instead
        let http_client = reqwest::Client::new();
        let client = AzureManagementClient::with_config(http_client, azure_config);

        let start_time = std::time::Instant::now();

        // This will fail with mock credentials, but should respect retry logic
        let _result = client.get_queue_counts("test-queue").await;

        let elapsed = start_time.elapsed();

        // The retry logic should add some delay, but not too much
        // With exponential backoff and max 3 retries, it should complete within reasonable time
        assert!(
            elapsed < Duration::from_secs(10),
            "Retry logic should complete within reasonable time, took: {elapsed:?}"
        );
    }
}

// Performance and load testing
mod performance_integration {
    use super::*;

    #[test]
    fn test_error_creation_performance() {
        let start = std::time::Instant::now();

        // Create many errors rapidly
        for i in 0..1000 {
            let _error = ServiceBusError::QueueNotFound(format!("queue-{i}"));
        }

        let duration = start.elapsed();

        // Error creation should be very fast
        assert!(
            duration < Duration::from_millis(100),
            "Creating 1000 errors should be very fast, took: {duration:?}"
        );
    }
}

// Integration tests for configuration validation
mod config_validation_integration {
    use super::*;

    #[test]
    fn test_azure_ad_config_required_fields() {
        let azure_config = create_mock_server_azure_config();

        // Test that all required getters work
        assert!(azure_config.tenant_id().is_ok());
        assert!(azure_config.client_id().is_ok());
        assert!(azure_config.client_secret().is_ok());
        assert!(azure_config.subscription_id().is_ok());
        assert!(azure_config.resource_group().is_ok());
        assert!(azure_config.namespace().is_ok());
    }

    #[test]
    fn test_azure_ad_config_missing_fields_error_messages() {
        let incomplete_config = create_incomplete_azure_config();

        // Test that missing fields produce helpful error messages
        let client_id_result = incomplete_config.client_id();
        assert!(client_id_result.is_err());

        let error_message = client_id_result.unwrap_err().to_string();
        assert!(error_message.contains("AZURE_AD__CLIENT_ID"));
        assert!(error_message.contains("required"));
    }
}
