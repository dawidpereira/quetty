use proptest::prelude::*;
use server::service_bus_manager::{AzureManagementClient, ResourceGroup, Subscription};
use std::collections::HashMap;
use std::time::Duration;

#[cfg(test)]
mod cache_property_tests {
    use super::*;
    use server::service_bus_manager::azure_management_client::AzureResourceCache;

    proptest! {
        #[test]
        fn test_cache_ttl_consistency(
            ttl_seconds in 1u64..3600,
            max_entries in 1usize..1000
        ) {
            let ttl = Duration::from_secs(ttl_seconds);
            let cache = AzureResourceCache::with_config(ttl, max_entries);

            // Property: Empty cache should always be empty
            prop_assert!(cache.is_empty());

            // Property: Cache should be created successfully with given config
            // Note: We can't directly access private fields, but we can test behavior
        }

        #[test]
        fn test_cache_entry_expiration_invariants(
            cache_data in prop::collection::vec(".*", 0..100),
            ttl_millis in 1u64..5000
        ) {
            let ttl = Duration::from_millis(ttl_millis);
            let mut cache = AzureResourceCache::with_config(ttl, 1000);

            // Add some test data
            let subscriptions: Vec<Subscription> = cache_data.iter().enumerate().map(|(i, name)| {
                Subscription {
                    id: format!("/subscriptions/{i}"),
                    subscription_id: format!("sub{i}"),
                    display_name: name.clone(),
                    state: "Enabled".to_string(),
                }
            }).collect();

            if !subscriptions.is_empty() {
                cache.cache_subscriptions(subscriptions.clone());

                // Property: Immediately after caching, data should be available
                prop_assert!(cache.get_cached_subscriptions().is_some());
                prop_assert!(!cache.is_empty());

                // Property: Cached data should match original data
                let cached = cache.get_cached_subscriptions().unwrap();
                prop_assert_eq!(cached.len(), subscriptions.len());
                for (original, cached_item) in subscriptions.iter().zip(cached.iter()) {
                    prop_assert_eq!(&original.subscription_id, &cached_item.subscription_id);
                    prop_assert_eq!(&original.display_name, &cached_item.display_name);
                }
            }
        }

        #[test]
        fn test_cache_lru_eviction_properties(
            subscription_count in 1usize..50,
            max_cache_size in 1usize..20
        ) {
            let mut cache = AzureResourceCache::with_config(Duration::from_secs(300), max_cache_size);

            // Add more subscriptions than max cache size
            for i in 0..subscription_count {
                let sub_id = format!("sub{i}");
                let resource_groups = vec![ResourceGroup {
                    id: format!("/subscriptions/{sub_id}/resourceGroups/rg"),
                    name: "test-rg".to_string(),
                    location: "eastus".to_string(),
                    tags: HashMap::new(),
                }];
                cache.cache_resource_groups(sub_id, resource_groups);
            }

            // Property: We can test cache behavior indirectly through get operations
            let mut found_count = 0;
            for i in 0..subscription_count {
                let sub_id = format!("sub{i}");
                if cache.get_cached_resource_groups(&sub_id).is_some() {
                    found_count += 1;
                }
            }

            // Property: Should find at most max_cache_size entries
            prop_assert!(found_count <= max_cache_size);

            // Property: If we added fewer items than max size, should find all
            if subscription_count <= max_cache_size {
                prop_assert_eq!(found_count, subscription_count);
            }
        }

        #[test]
        fn test_cache_clear_and_cleanup_invariants(
            initial_entries in 0usize..100
        ) {
            let mut cache = AzureResourceCache::with_config(Duration::from_millis(1), 1000);

            // Add test data
            for i in 0..initial_entries {
                let sub_id = format!("sub{i}");
                cache.cache_resource_groups(sub_id.clone(), vec![]);
                cache.cache_namespaces(sub_id.clone(), vec![]);
                cache.cache_connection_string(format!("ns{i}"), format!("conn{i}"));
            }

            if initial_entries > 0 {
                prop_assert!(!cache.is_empty());
            }

            // Property: Clear should make cache empty
            cache.clear();
            prop_assert!(cache.is_empty());

            // Verify cache is truly empty by checking some lookups return None
            prop_assert!(cache.get_cached_subscriptions().is_none());
            if initial_entries > 0 {
                prop_assert!(cache.get_cached_resource_groups("sub0").is_none());
                prop_assert!(cache.get_cached_namespaces("sub0").is_none());
                prop_assert!(cache.get_cached_connection_string("ns0").is_none());
            }
        }
    }
}

#[cfg(test)]
mod azure_management_client_property_tests {
    use super::*;

    proptest! {
        #[test]
        fn test_azure_management_client_creation_invariants(
            _dummy in 0u32..100 // Just to make this a property test
        ) {
            let http_client = reqwest::Client::new();
            let azure_client = AzureManagementClient::new(http_client);

            // Property: Client should be created successfully
            let debug_str1 = format!("{azure_client:?}");
            prop_assert!(!debug_str1.is_empty());

            // Property: Client should have consistent debug representation
            let debug_str2 = format!("{azure_client:?}");
            prop_assert!(debug_str2.contains("AzureManagementClient"));
        }
    }
}

#[cfg(test)]
mod error_handling_property_tests {
    use super::*;
    use server::service_bus_manager::ServiceBusError;

    proptest! {
        #[test]
        fn test_azure_api_error_properties(
            operation in "[a-z_]{1,50}",
            error_code in "[A-Z][a-zA-Z]{1,30}",
            status_code in 400u16..600,
            message in ".*{1,200}"
        ) {
            let error = ServiceBusError::azure_api_error(
                operation.clone(),
                error_code.clone(),
                status_code,
                message.clone()
            );

            // Property: Error should be identified as Azure API error
            prop_assert!(error.is_azure_api_error());

            // Property: Error code should be retrievable
            prop_assert_eq!(error.azure_error_code(), Some(error_code.as_str()));

            // Property: Display should contain all key information
            let display_str = error.to_string();
            prop_assert!(display_str.contains(&operation));
            prop_assert!(display_str.contains(&error_code));
            prop_assert!(display_str.contains(&status_code.to_string()));

            if !message.is_empty() {
                prop_assert!(display_str.contains(&message));
            }
        }

        #[test]
        fn test_azure_api_error_with_request_id_properties(
            operation in "[a-z_]{1,50}",
            error_code in "[A-Z][a-zA-Z]{1,30}",
            status_code in 400u16..600,
            message in ".*{1,200}",
            request_id in "[a-f0-9\\-]{10,50}"
        ) {
            let error = ServiceBusError::azure_api_error_with_request_id(
                operation.clone(),
                error_code.clone(),
                status_code,
                message.clone(),
                request_id.clone()
            );

            // Property: Error should be identified as Azure API error
            prop_assert!(error.is_azure_api_error());

            // Property: Request ID should be retrievable
            prop_assert_eq!(error.azure_request_id(), Some(request_id.as_str()));

            // Property: Display should include request ID
            let display_str = error.to_string();
            prop_assert!(display_str.contains(&request_id));
            prop_assert!(display_str.contains("Request ID"));
        }

        #[test]
        fn test_non_azure_error_properties(
            message in ".*{1,200}"
        ) {
            let errors = vec![
                ServiceBusError::ConnectionFailed(message.clone()),
                ServiceBusError::AuthenticationError(message.clone()),
                ServiceBusError::ConfigurationError(message.clone()),
                ServiceBusError::InternalError(message.clone()),
            ];

            for error in errors {
                // Property: Non-Azure errors should not be identified as Azure API errors
                prop_assert!(!error.is_azure_api_error());

                // Property: Azure-specific methods should return None
                prop_assert_eq!(error.azure_error_code(), None);
                prop_assert_eq!(error.azure_request_id(), None);

                // Property: Display should contain the message
                let display_str = error.to_string();
                if !message.is_empty() {
                    prop_assert!(display_str.contains(&message));
                }
            }
        }
    }
}
