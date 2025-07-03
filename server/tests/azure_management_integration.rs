use server::service_bus_manager::azure_management_client::{AzureManagementClient, ManagementApiError, StatisticsConfig};
use server::service_bus_manager::AzureAdConfig;
use tokio::time::Duration;

// Helper module for server-side integration tests
mod azure_helpers {
    use super::*;

    /// Create a mock Azure AD config for server testing
    pub fn create_mock_server_azure_config() -> AzureAdConfig {
        serde_json::from_str(r#"{
            "tenant_id": "test-tenant-id-server",
            "client_id": "test-client-id-server", 
            "client_secret": "test-client-secret-server",
            "subscription_id": "test-subscription-id-server",
            "resource_group": "test-resource-group-server",
            "namespace": "test-namespace-server"
        }"#).expect("Failed to create mock Azure AD config for server tests")
    }

    /// Create a mock Azure AD config with missing fields
    pub fn create_incomplete_azure_config() -> AzureAdConfig {
        serde_json::from_str(r#"{
            "tenant_id": "test-tenant-id"
        }"#).expect("Failed to create incomplete Azure AD config")
    }
}

use azure_helpers::*;

// Integration tests for AzureManagementClient creation and configuration
mod azure_management_client_creation {
    use super::*;

    #[test]
    fn test_azure_management_client_creation_with_valid_config() {
        let azure_config = create_mock_server_azure_config();
        
        // Test that client creation doesn't panic with valid config
        let result = AzureManagementClient::from_config(azure_config);
        
        // With mock data, we expect this to succeed in creation
        // (actual API calls will fail, but creation should work)
        assert!(result.is_ok(), "Client creation should succeed with valid config");
    }

    #[test]
    fn test_azure_management_client_creation_with_missing_fields() {
        let azure_config = create_incomplete_azure_config();
        
        // Test that client creation fails gracefully with incomplete config
        let result = AzureManagementClient::from_config(azure_config);
        
        assert!(result.is_err(), "Client creation should fail with incomplete config");
        
        // Verify the error type
        match result {
            Err(ManagementApiError::MissingConfiguration(_)) => {
                // Expected error type
            }
            Err(other) => panic!("Expected MissingConfiguration error, got: {:?}", other),
            Ok(_) => panic!("Expected error, got success"),
        }
    }

    #[test]
    fn test_azure_management_client_direct_creation() {
        let azure_config = create_mock_server_azure_config();
        
        // Test direct client creation
        AzureManagementClient::new(
            "test-subscription".to_string(),
            "test-rg".to_string(),
            "test-namespace".to_string(),
            azure_config,
        );
        
        // Client creation should always succeed (it's just storing the values)
        // Actual API calls will fail with mock data, but that's expected
    }
}

// Integration tests for Azure Management API error handling
mod error_handling_integration {
    use super::*;

    #[test]
    fn test_management_api_error_display() {
        let errors = vec![
            ManagementApiError::NotConfigured,
            ManagementApiError::AuthenticationFailed("Invalid token".to_string()),
            ManagementApiError::RequestFailed("Network error".to_string()),
            ManagementApiError::QueueNotFound("test-queue".to_string()),
            ManagementApiError::JsonParsingFailed("Invalid JSON".to_string()),
            ManagementApiError::MissingConfiguration("Missing field".to_string()),
        ];

        for error in errors {
            let error_string = error.to_string();
            assert!(!error_string.is_empty(), "Error display should not be empty");
            
            // Verify error messages contain expected content
            match error {
                ManagementApiError::NotConfigured => {
                    assert!(error_string.contains("not configured"));
                }
                ManagementApiError::AuthenticationFailed(_) => {
                    assert!(error_string.contains("Authentication failed"));
                }
                ManagementApiError::RequestFailed(_) => {
                    assert!(error_string.contains("HTTP request failed"));
                }
                ManagementApiError::QueueNotFound(_) => {
                    assert!(error_string.contains("Queue not found"));
                }
                ManagementApiError::JsonParsingFailed(_) => {
                    assert!(error_string.contains("JSON parsing failed"));
                }
                ManagementApiError::MissingConfiguration(_) => {
                    assert!(error_string.contains("Missing required configuration"));
                }
            }
        }
    }

    #[tokio::test]
    async fn test_azure_client_graceful_error_handling() {
        let azure_config = create_mock_server_azure_config();
        let client = AzureManagementClient::from_config(azure_config).unwrap();
        
        // Test that API calls fail gracefully with mock credentials
        let result = client.get_queue_message_count("test-queue").await;
        
        // We expect this to fail with mock credentials, but it should be a proper error
        assert!(result.is_err(), "Mock credentials should result in error");
        
        // The error should be authentication failure or request failure
        match result {
            Err(ManagementApiError::AuthenticationFailed(_)) |
            Err(ManagementApiError::RequestFailed(_)) => {
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
        let client = AzureManagementClient::from_config(azure_config).unwrap();
        
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
            (true, 30, true),    // Enabled with short TTL
            (false, 60, true),   // Disabled display
            (true, 300, false),  // Enabled but no API
            (false, 60, false),  // Fully disabled
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
        let client = AzureManagementClient::from_config(azure_config).unwrap();
        
        let start_time = std::time::Instant::now();
        
        // This will fail with mock credentials, but should respect retry logic
        let _result = client.get_queue_counts("test-queue").await;
        
        let elapsed = start_time.elapsed();
        
        // The retry logic should add some delay, but not too much
        // With exponential backoff and max 3 retries, it should complete within reasonable time
        assert!(elapsed < Duration::from_secs(10), 
                "Retry logic should complete within reasonable time, took: {:?}", elapsed);
    }
}

// Performance and load testing
mod performance_integration {
    use super::*;

    #[tokio::test]
    async fn test_client_creation_performance() {
        let start = std::time::Instant::now();
        
        // Create multiple clients rapidly
        for _i in 0..100 {
            let azure_config = create_mock_server_azure_config();
            let _client = AzureManagementClient::from_config(azure_config);
        }
        
        let duration = start.elapsed();
        
        // Client creation should be fast
        assert!(duration < Duration::from_millis(5000), 
                "Creating 100 clients should be fast, took: {:?}", duration);
    }

    #[test]
    fn test_error_creation_performance() {
        let start = std::time::Instant::now();
        
        // Create many errors rapidly
        for i in 0..1000 {
            let _error = ManagementApiError::QueueNotFound(format!("queue-{}", i));
        }
        
        let duration = start.elapsed();
        
        // Error creation should be very fast
        assert!(duration < Duration::from_millis(100), 
                "Creating 1000 errors should be very fast, took: {:?}", duration);
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

 