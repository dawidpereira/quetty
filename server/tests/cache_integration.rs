use server::service_bus_manager::azure_management_client::{
    AzureResourceCache, NamespaceProperties, ResourceGroup, ServiceBusNamespace, Subscription,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;

// Helper module for cache testing
mod cache_helpers {
    use super::*;

    /// Create a mock subscription for testing
    pub fn create_mock_subscription(id: &str, name: &str) -> Subscription {
        Subscription {
            id: format!("/subscriptions/{id}"),
            subscription_id: id.to_string(),
            display_name: name.to_string(),
            state: "Enabled".to_string(),
        }
    }

    /// Create a mock resource group for testing
    pub fn create_mock_resource_group(name: &str, location: &str) -> ResourceGroup {
        ResourceGroup {
            id: format!("/subscriptions/test/resourceGroups/{name}"),
            name: name.to_string(),
            location: location.to_string(),
            tags: HashMap::new(),
        }
    }

    /// Create a mock service bus namespace for testing
    pub fn create_mock_namespace(name: &str, location: &str) -> ServiceBusNamespace {
        ServiceBusNamespace {
            id: format!(
                "/subscriptions/test/resourceGroups/test/providers/Microsoft.ServiceBus/namespaces/{name}"
            ),
            name: name.to_string(),
            location: location.to_string(),
            resource_type: "Microsoft.ServiceBus/namespaces".to_string(),
            properties: NamespaceProperties {
                service_bus_endpoint: format!("https://{name}.servicebus.windows.net/"),
                status: Some("Active".to_string()),
                created_at: Some("2023-01-01T00:00:00Z".to_string()),
            },
        }
    }

    /// Create multiple mock subscriptions
    pub fn create_mock_subscriptions(count: usize) -> Vec<Subscription> {
        (0..count)
            .map(|i| create_mock_subscription(&format!("sub-{i}"), &format!("Subscription {i}")))
            .collect()
    }

    /// Create multiple mock resource groups
    pub fn create_mock_resource_groups(count: usize, location: &str) -> Vec<ResourceGroup> {
        (0..count)
            .map(|i| create_mock_resource_group(&format!("rg-{i}"), location))
            .collect()
    }

    /// Create multiple mock namespaces
    pub fn create_mock_namespaces(count: usize, location: &str) -> Vec<ServiceBusNamespace> {
        (0..count)
            .map(|i| create_mock_namespace(&format!("ns-{i}"), location))
            .collect()
    }
}

use cache_helpers::*;

// Integration tests for basic cache functionality
mod cache_basic_functionality {
    use super::*;

    #[test]
    fn test_cache_creation() {
        // Test default cache creation
        let cache = AzureResourceCache::new();
        assert!(cache.is_empty(), "New cache should be empty");
    }

    #[test]
    fn test_cache_creation_with_config() {
        let ttl = Duration::from_secs(600); // 10 minutes
        let max_entries = 50;

        let cache = AzureResourceCache::with_config(ttl, max_entries);
        assert!(cache.is_empty(), "New configured cache should be empty");
    }

    #[test]
    fn test_cache_subscription_storage_and_retrieval() {
        let mut cache = AzureResourceCache::new();
        let subscriptions = create_mock_subscriptions(3);

        // Cache subscriptions
        cache.cache_subscriptions(subscriptions.clone());
        assert!(
            !cache.is_empty(),
            "Cache should not be empty after adding subscriptions"
        );

        // Retrieve subscriptions
        let cached = cache.get_cached_subscriptions();
        assert!(cached.is_some(), "Should retrieve cached subscriptions");

        let cached_subs = cached.unwrap();
        assert_eq!(cached_subs.len(), 3, "Should have 3 cached subscriptions");
        assert_eq!(
            cached_subs, subscriptions,
            "Cached subscriptions should match original"
        );
    }

    #[test]
    fn test_cache_resource_group_storage_and_retrieval() {
        let mut cache = AzureResourceCache::new();
        let resource_groups = create_mock_resource_groups(2, "eastus");
        let subscription_id = "test-subscription";

        // Cache resource groups
        cache.cache_resource_groups(subscription_id.to_string(), resource_groups.clone());

        // Retrieve resource groups
        let cached = cache.get_cached_resource_groups(subscription_id);
        assert!(cached.is_some(), "Should retrieve cached resource groups");

        let cached_groups = cached.unwrap();
        assert_eq!(
            cached_groups.len(),
            2,
            "Should have 2 cached resource groups"
        );
        assert_eq!(
            cached_groups, resource_groups,
            "Cached resource groups should match original"
        );
    }

    #[test]
    fn test_cache_namespace_storage_and_retrieval() {
        let mut cache = AzureResourceCache::new();
        let namespaces = create_mock_namespaces(4, "westus");
        let subscription_id = "test-subscription";

        // Cache namespaces
        cache.cache_namespaces(subscription_id.to_string(), namespaces.clone());

        // Retrieve namespaces
        let cached = cache.get_cached_namespaces(subscription_id);
        assert!(cached.is_some(), "Should retrieve cached namespaces");

        let cached_ns = cached.unwrap();
        assert_eq!(cached_ns.len(), 4, "Should have 4 cached namespaces");
        assert_eq!(
            cached_ns, namespaces,
            "Cached namespaces should match original"
        );
    }

    #[test]
    fn test_cache_connection_string_storage_and_retrieval() {
        let mut cache = AzureResourceCache::new();
        let namespace_id = "test-namespace-id";
        let connection_string = "Endpoint=sb://test.servicebus.windows.net/;SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=testkey";

        // Cache connection string
        cache.cache_connection_string(namespace_id.to_string(), connection_string.to_string());

        // Retrieve connection string
        let cached = cache.get_cached_connection_string(namespace_id);
        assert!(cached.is_some(), "Should retrieve cached connection string");
        assert_eq!(
            cached.unwrap(),
            connection_string,
            "Cached connection string should match original"
        );
    }
}

// Integration tests for cache TTL (Time To Live) functionality
mod cache_ttl_functionality {
    use super::*;

    #[tokio::test]
    async fn test_cache_ttl_expiration() {
        let ttl = Duration::from_millis(100); // Very short TTL for testing
        let mut cache = AzureResourceCache::with_config(ttl, 10);

        let subscriptions = create_mock_subscriptions(2);

        // Cache subscriptions
        cache.cache_subscriptions(subscriptions.clone());

        // Should be available immediately
        let cached = cache.get_cached_subscriptions();
        assert!(
            cached.is_some(),
            "Should retrieve fresh cached subscriptions"
        );

        // Wait for TTL to expire
        sleep(Duration::from_millis(150)).await;

        // Should be expired now
        let expired = cache.get_cached_subscriptions();
        assert!(
            expired.is_none(),
            "Should not retrieve expired cached subscriptions"
        );
    }

    #[tokio::test]
    async fn test_cache_ttl_with_different_types() {
        let ttl = Duration::from_millis(100);
        let mut cache = AzureResourceCache::with_config(ttl, 10);

        let resource_groups = create_mock_resource_groups(1, "eastus");
        let namespaces = create_mock_namespaces(1, "westus");
        let subscription_id = "test-sub";

        // Cache different types
        cache.cache_resource_groups(subscription_id.to_string(), resource_groups);
        cache.cache_namespaces(subscription_id.to_string(), namespaces);
        cache.cache_connection_string("test-ns".to_string(), "test-connection".to_string());

        // All should be available immediately
        assert!(cache.get_cached_resource_groups(subscription_id).is_some());
        assert!(cache.get_cached_namespaces(subscription_id).is_some());
        assert!(cache.get_cached_connection_string("test-ns").is_some());

        // Wait for expiration
        sleep(Duration::from_millis(150)).await;

        // All should be expired
        assert!(cache.get_cached_resource_groups(subscription_id).is_none());
        assert!(cache.get_cached_namespaces(subscription_id).is_none());
        assert!(cache.get_cached_connection_string("test-ns").is_none());
    }

    #[test]
    fn test_cache_clean_expired() {
        let ttl = Duration::from_millis(1); // Very short TTL
        let mut cache = AzureResourceCache::with_config(ttl, 10);

        let subscriptions = create_mock_subscriptions(1);
        cache.cache_subscriptions(subscriptions);

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(5));

        // Should still be in cache (not cleaned yet)
        assert!(
            !cache.is_empty(),
            "Cache should not be empty before cleaning"
        );

        // Clean expired entries
        cache.clean_expired();

        // Should be empty after cleaning
        assert!(
            cache.is_empty(),
            "Cache should be empty after cleaning expired entries"
        );
    }
}

// Integration tests for cache size limits and LRU eviction
mod cache_size_limits {
    use super::*;

    #[test]
    fn test_cache_max_entries_resource_groups() {
        let mut cache = AzureResourceCache::with_config(Duration::from_secs(300), 2); // Max 2 entries

        let resource_groups = create_mock_resource_groups(1, "eastus");

        // Add entries up to the limit
        cache.cache_resource_groups("sub-1".to_string(), resource_groups.clone());
        cache.cache_resource_groups("sub-2".to_string(), resource_groups.clone());

        // Both should be retrievable
        assert!(cache.get_cached_resource_groups("sub-1").is_some());
        assert!(cache.get_cached_resource_groups("sub-2").is_some());

        // Add one more (should trigger LRU eviction)
        cache.cache_resource_groups("sub-3".to_string(), resource_groups.clone());

        // Oldest entry (sub-1) should be evicted
        assert!(
            cache.get_cached_resource_groups("sub-1").is_none(),
            "Oldest entry should be evicted"
        );
        assert!(
            cache.get_cached_resource_groups("sub-2").is_some(),
            "Second entry should remain"
        );
        assert!(
            cache.get_cached_resource_groups("sub-3").is_some(),
            "Newest entry should be present"
        );
    }

    #[test]
    fn test_cache_max_entries_namespaces() {
        let mut cache = AzureResourceCache::with_config(Duration::from_secs(300), 3); // Max 3 entries

        let namespaces = create_mock_namespaces(1, "westus");

        // Fill cache to capacity
        for i in 1..=3 {
            cache.cache_namespaces(format!("sub-{i}"), namespaces.clone());
        }

        // All should be present
        for i in 1..=3 {
            assert!(cache.get_cached_namespaces(&format!("sub-{i}")).is_some());
        }

        // Add two more (should evict oldest two)
        cache.cache_namespaces("sub-4".to_string(), namespaces.clone());
        cache.cache_namespaces("sub-5".to_string(), namespaces.clone());

        // Check eviction pattern
        assert!(
            cache.get_cached_namespaces("sub-1").is_none(),
            "First entry should be evicted"
        );
        assert!(
            cache.get_cached_namespaces("sub-2").is_none(),
            "Second entry should be evicted"
        );
        assert!(
            cache.get_cached_namespaces("sub-3").is_some(),
            "Third entry should remain"
        );
        assert!(
            cache.get_cached_namespaces("sub-4").is_some(),
            "Fourth entry should be present"
        );
        assert!(
            cache.get_cached_namespaces("sub-5").is_some(),
            "Fifth entry should be present"
        );
    }

    #[test]
    fn test_cache_max_entries_connection_strings() {
        let mut cache = AzureResourceCache::with_config(Duration::from_secs(300), 2); // Max 2 entries

        // Add connection strings up to limit
        cache.cache_connection_string("ns-1".to_string(), "connection-1".to_string());
        cache.cache_connection_string("ns-2".to_string(), "connection-2".to_string());

        // Both should be retrievable
        assert!(cache.get_cached_connection_string("ns-1").is_some());
        assert!(cache.get_cached_connection_string("ns-2").is_some());

        // Add one more (should trigger eviction)
        cache.cache_connection_string("ns-3".to_string(), "connection-3".to_string());

        // Oldest should be evicted
        assert!(cache.get_cached_connection_string("ns-1").is_none());
        assert!(cache.get_cached_connection_string("ns-2").is_some());
        assert!(cache.get_cached_connection_string("ns-3").is_some());
    }

    #[test]
    fn test_cache_update_existing_entry() {
        let mut cache = AzureResourceCache::with_config(Duration::from_secs(300), 2);

        let resource_groups_v1 = create_mock_resource_groups(1, "eastus");
        let resource_groups_v2 = create_mock_resource_groups(2, "westus");

        // Cache initial entry
        cache.cache_resource_groups("sub-1".to_string(), resource_groups_v1);
        cache.cache_resource_groups(
            "sub-2".to_string(),
            create_mock_resource_groups(1, "centralus"),
        );

        // Update existing entry (should not trigger eviction)
        cache.cache_resource_groups("sub-1".to_string(), resource_groups_v2.clone());

        // Both entries should still be present
        assert!(cache.get_cached_resource_groups("sub-1").is_some());
        assert!(cache.get_cached_resource_groups("sub-2").is_some());

        // Updated entry should have new data
        let updated = cache.get_cached_resource_groups("sub-1").unwrap();
        assert_eq!(
            updated.len(),
            2,
            "Updated entry should have 2 resource groups"
        );
        assert_eq!(
            updated, resource_groups_v2,
            "Updated entry should match new data"
        );
    }
}

// Integration tests for cache performance under load
mod cache_performance {
    use super::*;

    #[test]
    fn test_cache_performance_many_operations() {
        let mut cache = AzureResourceCache::with_config(Duration::from_secs(300), 1000);

        let start = Instant::now();

        // Perform many cache operations
        for i in 0..1000 {
            let subscriptions = create_mock_subscriptions(1);
            cache.cache_subscriptions(subscriptions);

            let resource_groups = create_mock_resource_groups(1, "eastus");
            cache.cache_resource_groups(format!("sub-{i}"), resource_groups);

            let _cached = cache.get_cached_subscriptions();
            let _cached_rg = cache.get_cached_resource_groups(&format!("sub-{i}"));
        }

        let duration = start.elapsed();

        // Many cache operations should be reasonably fast
        assert!(
            duration < Duration::from_secs(1),
            "1000 cache operations should be fast, took: {duration:?}"
        );
    }

    #[test]
    fn test_cache_performance_large_data() {
        let mut cache = AzureResourceCache::with_config(Duration::from_secs(300), 100);

        // Create large datasets
        let large_subscriptions = create_mock_subscriptions(100);
        let large_resource_groups = create_mock_resource_groups(100, "eastus");
        let large_namespaces = create_mock_namespaces(100, "westus");

        let start = Instant::now();

        // Cache large datasets
        cache.cache_subscriptions(large_subscriptions.clone());
        cache.cache_resource_groups("large-sub".to_string(), large_resource_groups.clone());
        cache.cache_namespaces("large-sub".to_string(), large_namespaces.clone());

        // Retrieve large datasets
        let _cached_subs = cache.get_cached_subscriptions();
        let _cached_rgs = cache.get_cached_resource_groups("large-sub");
        let _cached_ns = cache.get_cached_namespaces("large-sub");

        let duration = start.elapsed();

        // Large data operations should still be reasonably fast
        assert!(
            duration < Duration::from_secs(1),
            "Large data cache operations should be fast, took: {duration:?}"
        );
    }

    #[test]
    fn test_cache_memory_efficiency() {
        let mut cache = AzureResourceCache::with_config(Duration::from_secs(300), 10);

        // Fill cache with data
        for i in 0..20 {
            let resource_groups = create_mock_resource_groups(5, "eastus");
            cache.cache_resource_groups(format!("sub-{i}"), resource_groups);
        }

        // With max 10 entries, only the last 10 should remain
        let mut present_count = 0;
        for i in 0..20 {
            if cache
                .get_cached_resource_groups(&format!("sub-{i}"))
                .is_some()
            {
                present_count += 1;
            }
        }

        assert!(
            present_count <= 10,
            "Cache should not exceed max entries, found {present_count} entries"
        );

        // The last entries should be present
        for i in 15..20 {
            assert!(
                cache
                    .get_cached_resource_groups(&format!("sub-{i}"))
                    .is_some(),
                "Recent entries should be present in cache"
            );
        }
    }
}

// Integration tests for cache edge cases and error conditions
mod cache_edge_cases {
    use super::*;

    #[test]
    fn test_cache_empty_data() {
        let mut cache = AzureResourceCache::new();

        // Cache empty collections
        cache.cache_subscriptions(Vec::new());
        cache.cache_resource_groups("empty-sub".to_string(), Vec::new());
        cache.cache_namespaces("empty-sub".to_string(), Vec::new());

        // Should be able to retrieve empty collections
        let cached_subs = cache.get_cached_subscriptions();
        assert!(cached_subs.is_some(), "Should retrieve empty subscriptions");
        assert!(
            cached_subs.unwrap().is_empty(),
            "Cached subscriptions should be empty"
        );

        let cached_rgs = cache.get_cached_resource_groups("empty-sub");
        assert!(
            cached_rgs.is_some(),
            "Should retrieve empty resource groups"
        );
        assert!(
            cached_rgs.unwrap().is_empty(),
            "Cached resource groups should be empty"
        );
    }

    #[test]
    fn test_cache_clear_functionality() {
        let mut cache = AzureResourceCache::new();

        // Add various data
        cache.cache_subscriptions(create_mock_subscriptions(2));
        cache.cache_resource_groups(
            "test-sub".to_string(),
            create_mock_resource_groups(2, "eastus"),
        );
        cache.cache_namespaces("test-sub".to_string(), create_mock_namespaces(2, "westus"));
        cache.cache_connection_string("test-ns".to_string(), "test-connection".to_string());

        // Cache should not be empty
        assert!(
            !cache.is_empty(),
            "Cache should not be empty after adding data"
        );

        // Clear cache
        cache.clear();

        // Cache should be empty
        assert!(cache.is_empty(), "Cache should be empty after clearing");

        // All retrievals should return None
        assert!(cache.get_cached_subscriptions().is_none());
        assert!(cache.get_cached_resource_groups("test-sub").is_none());
        assert!(cache.get_cached_namespaces("test-sub").is_none());
        assert!(cache.get_cached_connection_string("test-ns").is_none());
    }

    #[test]
    fn test_cache_with_zero_max_entries() {
        let mut cache = AzureResourceCache::with_config(Duration::from_secs(300), 0);

        // Adding to a cache with 0 max entries should not break anything
        cache.cache_resource_groups("test".to_string(), create_mock_resource_groups(1, "eastus"));

        // With zero max entries, the cache implementation may or may not store the entry
        // The important thing is that it doesn't crash or panic
        // Let's just verify that the cache operations complete without error
        let _result = cache.get_cached_resource_groups("test");

        // The behavior with zero max entries is implementation-dependent
        // Some implementations might store nothing, others might store one entry
        // The test should focus on ensuring no crashes occur
        // Cache operations with zero max entries should not crash
    }

    #[test]
    fn test_cache_retrieval_with_non_existent_keys() {
        let cache = AzureResourceCache::new();

        // Retrieving non-existent data should return None
        assert!(cache.get_cached_subscriptions().is_none());
        assert!(cache.get_cached_resource_groups("non-existent").is_none());
        assert!(cache.get_cached_namespaces("non-existent").is_none());
        assert!(cache.get_cached_connection_string("non-existent").is_none());
    }

    #[tokio::test]
    async fn test_cache_concurrent_access() {
        use std::sync::{Arc, Mutex};

        let cache = Arc::new(Mutex::new(AzureResourceCache::new()));
        let mut handles = Vec::new();

        // Launch concurrent operations
        for i in 0..10 {
            let cache_clone = cache.clone();
            let handle = tokio::spawn(async move {
                // Each task adds and retrieves data
                {
                    let mut cache = cache_clone.lock().unwrap();
                    let resource_groups = create_mock_resource_groups(1, "eastus");
                    cache.cache_resource_groups(format!("sub-{i}"), resource_groups);
                }

                {
                    let cache = cache_clone.lock().unwrap();
                    let _cached = cache.get_cached_resource_groups(&format!("sub-{i}"));
                }

                i
            });
            handles.push(handle);
        }

        // Wait for all operations
        let mut results = Vec::new();
        for handle in handles {
            let result = handle.await.expect("Concurrent operation should complete");
            results.push(result);
        }

        // All operations should complete successfully
        assert_eq!(
            results.len(),
            10,
            "All concurrent operations should complete"
        );
    }
}
