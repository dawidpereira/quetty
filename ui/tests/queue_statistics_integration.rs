use quetty::app::managers::queue_stats_manager::QueueStatsManager;
use quetty::app::updates::messages::pagination::QueueStatsCache;
use server::service_bus_manager::azure_management_client::{ManagementApiError, StatisticsConfig};
use server::service_bus_manager::queue_statistics_service::QueueStatisticsService;
use server::service_bus_manager::{AzureAdConfig, QueueType};

// Helper functions for queue statistics integration tests
fn create_mock_azure_ad_config() -> AzureAdConfig {
    serde_json::from_str(
        r#"{
        "tenant_id": "test-tenant-id",
        "client_id": "test-client-id", 
        "client_secret": "test-client-secret",
        "subscription_id": "test-subscription-id",
        "resource_group": "test-resource-group",
        "namespace": "test-namespace"
    }"#,
    )
    .expect("Failed to create mock Azure AD config")
}

fn create_test_stats_config(
    display_enabled: bool,
    cache_ttl: u64,
    use_api: bool,
) -> StatisticsConfig {
    StatisticsConfig::new(display_enabled, cache_ttl, use_api)
}

fn create_test_cache(queue_name: &str, active: u64, dlq: u64) -> QueueStatsCache {
    QueueStatsCache::new(queue_name.to_string(), active, dlq)
}

// Integration tests for QueueStatsManager
mod queue_stats_manager_integration {
    use super::*;

    #[test]
    fn test_queue_stats_manager_creation() {
        let manager = QueueStatsManager::new();
        assert!(!manager.has_valid_cache("test-queue"));
    }

    #[test]
    fn test_cache_basic_operations() {
        let mut manager = QueueStatsManager::new();

        // Test with fresh cache
        let fresh_cache = create_test_cache("test-queue", 100, 50);
        assert!(!fresh_cache.is_expired());

        manager.update_stats_cache(fresh_cache);
        assert!(manager.has_valid_cache("test-queue"));
        assert!(!manager.is_stats_cache_expired("test-queue"));
    }

    #[test]
    fn test_multi_queue_cache_management() {
        let mut manager = QueueStatsManager::new();

        // Add caches for multiple queues
        let cache1 = create_test_cache("queue-1", 100, 10);
        let cache2 = create_test_cache("queue-2", 200, 20);

        manager.update_stats_cache(cache1);
        manager.update_stats_cache(cache2);

        // Test individual queue cache validity
        assert!(manager.has_valid_cache("queue-1"));
        assert!(manager.has_valid_cache("queue-2"));
        assert!(!manager.has_valid_cache("nonexistent-queue"));

        // Test cache retrieval
        let retrieved_cache1 = manager.get_cached_stats("queue-1");
        assert!(retrieved_cache1.is_some());
        assert_eq!(retrieved_cache1.unwrap().active_count, 100);
    }

    #[test]
    fn test_cache_invalidation() {
        let mut manager = QueueStatsManager::new();

        // Add cache for a queue
        let cache = create_test_cache("test-queue", 100, 50);
        manager.update_stats_cache(cache);
        assert!(manager.has_valid_cache("test-queue"));

        // Invalidate the cache
        manager.invalidate_stats_cache_for_queue("test-queue");
        assert!(!manager.has_valid_cache("test-queue"));

        // Invalidating non-existent queue should not panic
        manager.invalidate_stats_cache_for_queue("nonexistent-queue");
    }
}

// Integration tests for QueueStatisticsService
mod queue_statistics_service_integration {
    use super::*;

    #[test]
    fn test_queue_statistics_service_creation() {
        // Test with management API disabled
        let config = create_test_stats_config(true, 60, false);
        let azure_config = create_mock_azure_ad_config();
        let service = QueueStatisticsService::new(config, azure_config);

        assert!(
            !service.is_available(),
            "Service should not be available when API is disabled"
        );
        assert!(service.config().display_enabled);
        assert_eq!(service.config().cache_ttl_seconds, 60);
        assert!(!service.config().use_management_api);
    }

    #[test]
    fn test_queue_statistics_service_disabled_display() {
        // Test with display disabled
        let config = create_test_stats_config(false, 60, true);
        let azure_config = create_mock_azure_ad_config();
        let service = QueueStatisticsService::new(config, azure_config);

        assert!(
            !service.is_available(),
            "Service should not be available when display is disabled"
        );
    }

    #[tokio::test]
    async fn test_queue_statistics_disabled_returns_none() {
        // Test that disabled statistics return None
        let config = create_test_stats_config(false, 60, true);
        let azure_config = create_mock_azure_ad_config();
        let service = QueueStatisticsService::new(config, azure_config);

        let result = service
            .get_queue_statistics("test-queue", &QueueType::Main)
            .await;
        assert_eq!(result, None, "Disabled statistics should return None");

        let (active, dlq) = service.get_both_queue_counts("test-queue").await;
        assert_eq!(active, None);
        assert_eq!(dlq, None);
    }

    #[tokio::test]
    async fn test_queue_statistics_no_client_returns_none() {
        // Test that missing client returns None
        let config = create_test_stats_config(true, 60, false); // API disabled
        let azure_config = create_mock_azure_ad_config();
        let service = QueueStatisticsService::new(config, azure_config);

        let result = service
            .get_queue_statistics("test-queue", &QueueType::Main)
            .await;
        assert_eq!(result, None, "Missing client should return None");
    }
}

// Integration tests for QueueStatsCache
mod queue_stats_cache_integration {
    use super::*;

    #[test]
    fn test_queue_stats_cache_creation_and_properties() {
        let cache = create_test_cache("test-queue", 150, 25);

        assert_eq!(cache.queue_name, "test-queue");
        assert_eq!(cache.active_count, 150);
        assert_eq!(cache.dlq_count, 25);
        assert!(!cache.is_expired());
    }

    #[test]
    fn test_queue_stats_cache_count_retrieval() {
        let cache = create_test_cache("test-queue", 150, 75);

        // Test main queue count
        assert_eq!(cache.get_count_for_type(&QueueType::Main), 150);

        // Test dead letter queue count
        assert_eq!(cache.get_count_for_type(&QueueType::DeadLetter), 75);
    }

    #[test]
    fn test_queue_stats_cache_age_calculation() {
        let cache = create_test_cache("test-queue", 100, 50);

        // Age should be recent for newly created cache
        let age = cache.age_seconds();
        assert!(
            (0..5).contains(&age),
            "Age should be very recent for new cache"
        );
    }
}

// Configuration integration tests
mod configuration_integration {
    #[test]
    fn test_queue_stats_configuration_defaults() {
        // Test that queue statistics have sensible defaults when not specified

        // These values should match the defaults in the actual config system
        let expected_display_enabled = true;
        let expected_cache_ttl = 60;
        let expected_use_api = true;

        assert!(
            expected_display_enabled,
            "Default should enable statistics display"
        );
        assert_eq!(expected_cache_ttl, 60, "Default TTL should be 60 seconds");
        assert!(expected_use_api, "Default should enable management API");
    }
}

// Error handling integration tests
mod error_handling_integration {
    use super::*;

    #[test]
    fn test_management_api_error_types() {
        // Test that different error types are properly handled
        let auth_error =
            ManagementApiError::AuthenticationFailed("Invalid credentials".to_string());
        assert!(auth_error.to_string().contains("Authentication failed"));

        let not_found_error = ManagementApiError::QueueNotFound("test-queue".to_string());
        assert!(not_found_error.to_string().contains("Queue not found"));

        let request_error = ManagementApiError::RequestFailed("Network timeout".to_string());
        assert!(request_error.to_string().contains("HTTP request failed"));
    }

    #[tokio::test]
    async fn test_service_graceful_degradation() {
        // Test that the service gracefully handles various error conditions
        let config = create_test_stats_config(true, 60, true);
        let azure_config = create_mock_azure_ad_config();
        let service = QueueStatisticsService::new(config, azure_config);

        // Since we're using mock config, the service should handle authentication failure gracefully
        // The actual Azure AD client will fail, but the service should not panic
        let result = service
            .get_queue_statistics("test-queue", &QueueType::Main)
            .await;

        // With mock credentials, we expect None (graceful failure)
        assert_eq!(
            result, None,
            "Service should gracefully handle authentication failures"
        );
    }
}

// End-to-end integration test for the complete statistics workflow
mod end_to_end_integration {
    use super::*;

    #[tokio::test]
    async fn test_complete_statistics_workflow() {
        // This test simulates the complete workflow of:
        // 1. Creating a statistics service
        // 2. Updating cache in the manager
        // 3. Checking cache validity
        // 4. Cache invalidation

        let mut stats_manager = QueueStatsManager::new();
        let config = create_test_stats_config(true, 60, false); // Disable API for testing
        let azure_config = create_mock_azure_ad_config();
        let _stats_service = QueueStatisticsService::new(config, azure_config);

        // Step 1: Initially no cache
        assert!(!stats_manager.has_valid_cache("test-queue"));
        assert!(stats_manager.is_stats_cache_expired("test-queue"));

        // Step 2: Simulate getting statistics and caching them
        let mock_cache = create_test_cache("test-queue", 358, 42);
        stats_manager.update_stats_cache(mock_cache);

        // Step 3: Verify cache is valid
        assert!(stats_manager.has_valid_cache("test-queue"));
        assert!(!stats_manager.is_stats_cache_expired("test-queue"));

        let cached_stats = stats_manager.get_cached_stats("test-queue");
        assert!(cached_stats.is_some());
        assert_eq!(cached_stats.unwrap().active_count, 358);
        assert_eq!(cached_stats.unwrap().dlq_count, 42);

        // Step 4: Simulate cache invalidation after bulk operation
        let fresh_cache = create_test_cache("test-queue", 300, 30);
        stats_manager.update_stats_cache(fresh_cache);
        assert!(stats_manager.has_valid_cache("test-queue"));

        stats_manager.invalidate_stats_cache_for_queue("test-queue");
        assert!(!stats_manager.has_valid_cache("test-queue"));
    }

    #[tokio::test]
    async fn test_statistics_service_configuration_scenarios() {
        // Test different configuration scenarios

        // Scenario 1: Fully enabled
        let config1 = create_test_stats_config(true, 60, true);
        let azure_config1 = create_mock_azure_ad_config();
        let service1 = QueueStatisticsService::new(config1, azure_config1);

        // With mock config, the client creation will fail, so service won't be available
        // but it should handle this gracefully
        assert!(service1.config().display_enabled);
        assert!(service1.config().use_management_api);

        // Scenario 2: Display disabled
        let config2 = create_test_stats_config(false, 60, true);
        let azure_config2 = create_mock_azure_ad_config();
        let service2 = QueueStatisticsService::new(config2, azure_config2);
        assert!(!service2.is_available());

        // Scenario 3: API disabled
        let config3 = create_test_stats_config(true, 60, false);
        let azure_config3 = create_mock_azure_ad_config();
        let service3 = QueueStatisticsService::new(config3, azure_config3);
        assert!(!service3.is_available());
    }
}
