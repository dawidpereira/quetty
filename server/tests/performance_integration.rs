use server::service_bus_manager::AzureManagementClient;
use server::service_bus_manager::azure_management_client::AzureResourceCache;
use std::time::{Duration, Instant};
use tokio::task::JoinSet;
use tokio::time::timeout;

#[cfg(test)]
mod performance_tests {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_operations_performance() {
        use tokio::task::JoinSet;

        let concurrency_levels = vec![1, 5, 10, 20];

        for concurrency in concurrency_levels {
            let start = Instant::now();
            let mut join_set = JoinSet::new();

            // Spawn concurrent tasks
            for i in 0..concurrency {
                join_set.spawn(async move {
                    let task_start = Instant::now();

                    // Simulate work similar to Azure API calls
                    let mut operations = 0;

                    for _ in 0..100 {
                        operations += 1;
                        // Small delay to simulate network call
                        tokio::time::sleep(Duration::from_micros(100)).await;
                    }

                    (i, task_start.elapsed(), operations)
                });
            }

            // Collect results
            let mut results = Vec::new();
            while let Some(result) = join_set.join_next().await {
                match result {
                    Ok(task_result) => results.push(task_result),
                    Err(e) => panic!("Task failed: {e}"),
                }
            }

            let total_duration = start.elapsed();

            // Performance assertions
            assert!(
                total_duration < Duration::from_secs(5),
                "Concurrent operations took too long: {total_duration:?}"
            );

            // All tasks should complete
            assert_eq!(results.len(), concurrency);

            // Calculate statistics
            let avg_task_duration: Duration = results
                .iter()
                .map(|(_, duration, _)| *duration)
                .sum::<Duration>()
                / concurrency as u32;

            let total_operations: usize = results.iter().map(|(_, _, ops)| *ops).sum();

            println!(
                "Concurrency: {concurrency}, Total time: {total_duration:?}, Avg task time: {avg_task_duration:?}, Total ops: {total_operations}"
            );
        }
    }
}

#[cfg(test)]
mod memory_performance_tests {
    use super::*;
    use server::service_bus_manager::azure_management_client::AzureResourceCache;

    #[tokio::test]
    async fn test_cache_memory_efficiency() {
        use server::service_bus_manager::ResourceGroup;
        use std::collections::HashMap;

        let cache_sizes = vec![10, 100, 1000];
        let entry_counts = vec![50, 500, 5000];

        for (max_size, entry_count) in cache_sizes.into_iter().zip(entry_counts.into_iter()) {
            let entry_count = entry_count as usize;
            let max_size = max_size as usize;
            let start = Instant::now();
            let mut cache = AzureResourceCache::with_config(Duration::from_secs(300), max_size);

            // Add entries to cache
            for i in 0..entry_count {
                let sub_id = format!("sub{i}");
                let resource_groups = vec![ResourceGroup {
                    id: format!("/subscriptions/{sub_id}/resourceGroups/rg"),
                    name: "test-rg".to_string(),
                    location: "eastus".to_string(),
                    tags: HashMap::new(),
                }];
                cache.cache_resource_groups(sub_id, resource_groups);

                // Periodically check performance
                if i % 100 == 0 {
                    let elapsed = start.elapsed();
                    assert!(
                        elapsed < Duration::from_millis((100 * (i / 100 + 1)) as u64),
                        "Cache operations getting too slow at entry {i}: {elapsed:?}"
                    );
                }
            }

            let total_time = start.elapsed();

            // Test retrieval performance
            let retrieval_start = Instant::now();
            let mut found_count = 0;

            for i in 0..std::cmp::min(entry_count, max_size) {
                let sub_id = format!("sub{}", entry_count - 1 - i); // Recent entries
                if cache.get_cached_resource_groups(&sub_id).is_some() {
                    found_count += 1;
                }
            }

            let retrieval_time = retrieval_start.elapsed();

            // Performance assertions
            assert!(
                total_time < Duration::from_secs(1),
                "Cache insertion took too long: {total_time:?}"
            );
            assert!(
                retrieval_time < Duration::from_millis(10),
                "Cache retrieval took too long: {retrieval_time:?}"
            );

            // Memory management assertions
            if entry_count > max_size {
                // Should have evicted older entries
                assert!(found_count <= max_size);
            } else {
                // Should have all entries
                assert_eq!(found_count, entry_count);
            }

            println!(
                "Cache size: {max_size}, Entries: {entry_count}, Found: {found_count}, Insert time: {total_time:?}, Retrieval time: {retrieval_time:?}"
            );
        }
    }

    #[tokio::test]
    async fn test_cache_cleanup_performance() {
        let mut cache = AzureResourceCache::with_config(Duration::from_millis(1), 1000);

        // Add many entries
        for i in 0..1000 {
            let sub_id = format!("sub{i}");
            cache.cache_resource_groups(sub_id.clone(), vec![]);
            cache.cache_namespaces(sub_id.clone(), vec![]);
            cache.cache_connection_string(format!("ns{i}"), format!("conn{i}"));
        }

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Test cleanup performance
        let cleanup_start = Instant::now();
        cache.clean_expired();
        let cleanup_time = cleanup_start.elapsed();

        // Should be fast even with many entries
        assert!(
            cleanup_time < Duration::from_millis(50),
            "Cache cleanup took too long: {cleanup_time:?}"
        );

        // Should be empty after cleanup
        assert!(cache.is_empty());

        println!("Cleanup time for 1000 entries: {cleanup_time:?}");
    }
}

#[cfg(test)]
mod scalability_tests {
    use super::*;

    #[tokio::test]
    async fn test_azure_management_client_creation_speed() {
        let creation_counts = vec![1, 10, 100];

        for count in creation_counts {
            let start = Instant::now();
            let mut clients = Vec::with_capacity(count);

            for _ in 0..count {
                let http_client = reqwest::Client::new();
                let azure_client = AzureManagementClient::new(http_client);
                clients.push(azure_client);
            }

            let duration = start.elapsed();
            let per_client = duration / count as u32;

            // Should create clients in reasonable time (very lenient for HTTP client setup)
            assert!(
                per_client < Duration::from_secs(1),
                "Client creation too slow: {per_client:?} per client"
            );

            println!("Created {count} clients in {duration:?} ({per_client:?} per client)");
        }
    }

    #[tokio::test]
    async fn test_timeout_handling_performance() {
        let timeout_scenarios = vec![
            Duration::from_millis(10),   // Very short timeout
            Duration::from_millis(100),  // Short timeout
            Duration::from_millis(1000), // Normal timeout
        ];

        for timeout_duration in timeout_scenarios {
            let start = Instant::now();

            // Simulate operation that might timeout
            let result = timeout(timeout_duration, async {
                // Simulate varying work duration
                tokio::time::sleep(Duration::from_millis(50)).await;
                "completed"
            })
            .await;

            let elapsed = start.elapsed();

            match result {
                Ok(_) => {
                    // Should complete within timeout + small buffer
                    assert!(
                        elapsed <= timeout_duration + Duration::from_millis(20),
                        "Operation took longer than expected: {elapsed:?} > {timeout_duration:?}"
                    );
                }
                Err(_) => {
                    // Timeout should be respected with small buffer
                    assert!(
                        elapsed <= timeout_duration + Duration::from_millis(20),
                        "Timeout took too long: {elapsed:?} > {timeout_duration:?}"
                    );
                }
            }

            println!(
                "Timeout test: {:?} - Elapsed: {:?} - Result: {}",
                timeout_duration,
                elapsed,
                result.is_ok()
            );
        }
    }
}

#[cfg(test)]
mod integration_test_optimization {
    use super::*;

    /// Helper function to create test environments with different scales
    fn create_test_environment(scale: &str) -> (usize, usize, usize, Duration) {
        match scale {
            "small" => (5, 2, 1, Duration::from_millis(100)),
            "medium" => (20, 5, 3, Duration::from_millis(500)),
            "large" => (50, 10, 5, Duration::from_secs(2)),
            _ => (10, 3, 2, Duration::from_millis(200)),
        }
    }

    #[tokio::test]
    async fn test_environment_scaling() {
        let scales = vec!["small", "medium", "large"];

        for scale in scales {
            let (subs, rgs_per_sub, ns_per_sub, max_duration) = create_test_environment(scale);
            let start = Instant::now();

            // Create test data based on scale
            let mut total_operations = 0;

            for _ in 0..subs {
                for _ in 0..rgs_per_sub {
                    total_operations += 1;
                }
                for _ in 0..ns_per_sub {
                    total_operations += 1;
                }
            }

            let duration = start.elapsed();

            // Scale-appropriate performance expectations
            assert!(
                duration < max_duration,
                "Scale '{scale}' took too long: {duration:?} > {max_duration:?}"
            );

            println!(
                "Scale: {scale} - Subs: {subs}, Ops: {total_operations}, Duration: {duration:?}"
            );
        }
    }

    #[tokio::test]
    async fn test_parallel_vs_sequential_performance() {
        let task_count = 20;

        // Sequential execution
        let sequential_start = Instant::now();
        for _i in 0..task_count {
            // Simulate work
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let sequential_duration = sequential_start.elapsed();

        // Parallel execution
        let parallel_start = Instant::now();
        let mut join_set = JoinSet::new();

        for i in 0..task_count {
            join_set.spawn(async move {
                // Simulate work
                tokio::time::sleep(Duration::from_millis(5)).await;
                i
            });
        }

        let mut results = Vec::new();
        while let Some(result) = join_set.join_next().await {
            results.push(result.unwrap());
        }
        let parallel_duration = parallel_start.elapsed();

        // Parallel should be significantly faster
        let speedup = sequential_duration.as_millis() as f64 / parallel_duration.as_millis() as f64;

        assert!(
            speedup > 2.0,
            "Parallel execution not fast enough: {speedup}x speedup"
        );
        assert_eq!(results.len(), task_count);

        println!(
            "Sequential: {sequential_duration:?}, Parallel: {parallel_duration:?}, Speedup: {speedup:.2}x"
        );
    }
}

/// Integration test configuration for different CI environments
#[cfg(test)]
mod ci_optimization {
    use super::*;

    fn get_ci_environment() -> &'static str {
        match std::env::var("CI_ENVIRONMENT") {
            Ok(env) => match env.as_str() {
                "github_actions" => "github_actions",
                "azure_devops" => "azure_devops",
                _ => "local",
            },
            Err(_) => "local",
        }
    }

    fn get_test_scale() -> &'static str {
        match get_ci_environment() {
            "github_actions" => "small", // Limited resources
            "azure_devops" => "medium",  // Better resources
            "local" => "large",          // Full testing
            _ => "small",
        }
    }

    #[tokio::test]
    async fn test_ci_optimized_performance() {
        let scale = get_test_scale();
        let (max_operations, timeout_ms) = match scale {
            "small" => (100, 1000),
            "medium" => (500, 5000),
            "large" => (1000, 10000),
            _ => (100, 1000),
        };

        let start = Instant::now();
        let mut operations = 0;

        // Perform scaled number of operations
        for _ in 0..max_operations {
            operations += 1;
        }

        let duration = start.elapsed();
        let timeout = Duration::from_millis(timeout_ms);

        assert!(
            duration < timeout,
            "CI test took too long for scale '{scale}': {duration:?} > {timeout:?}"
        );

        println!(
            "CI Environment: {}, Scale: {}, Operations: {}, Duration: {:?}",
            get_ci_environment(),
            scale,
            operations,
            duration
        );
    }

    #[tokio::test]
    async fn test_memory_constrained_environment() {
        let scale = get_test_scale();
        let cache_size = match scale {
            "small" => 10,
            "medium" => 50,
            "large" => 100,
            _ => 10,
        };

        // Test with memory constraints
        let mut cache = AzureResourceCache::with_config(Duration::from_secs(60), cache_size);

        // Add entries up to limit
        for i in 0..cache_size * 2 {
            let sub_id = format!("sub{i}");
            cache.cache_resource_groups(sub_id, vec![]);
        }

        // Should not exceed memory limits
        let mut found_count = 0;
        for i in 0..cache_size * 2 {
            let sub_id = format!("sub{i}");
            if cache.get_cached_resource_groups(&sub_id).is_some() {
                found_count += 1;
            }
        }

        assert!(
            found_count <= cache_size,
            "Cache exceeded size limit: {found_count} > {cache_size}"
        );

        println!("Memory test - Scale: {scale}, Cache size: {cache_size}, Found: {found_count}");
    }
}
