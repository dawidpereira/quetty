use server::common::{RateLimitError, RateLimiter, RateLimiterConfig};
use server::service_bus_manager::AzureAdConfig;
use server::service_bus_manager::azure_management_client::AzureManagementClient;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

// Helper module for rate limiting testing
mod rate_limiting_helpers {
    use super::*;

    /// Create a mock Azure AD config for rate limiting testing
    pub fn create_mock_azure_config() -> AzureAdConfig {
        serde_json::from_str(
            r#"{
            "tenant_id": "test-tenant-rate-limit",
            "client_id": "test-client-rate-limit",
            "client_secret": "test-client-secret-rate-limit",
            "subscription_id": "test-subscription-rate-limit",
            "resource_group": "test-resource-group-rate-limit",
            "namespace": "test-namespace-rate-limit"
        }"#,
        )
        .expect("Failed to create mock Azure AD config for rate limiting tests")
    }

    /// Create a rate limiter with very permissive settings for testing
    pub fn create_test_rate_limiter(requests_per_second: u32) -> RateLimiter {
        RateLimiter::new(requests_per_second)
    }

    /// Create a rate limiter config for testing
    pub fn create_test_rate_limiter_config(
        requests_per_second: u32,
        burst_size: Option<u32>,
    ) -> RateLimiterConfig {
        RateLimiterConfig {
            requests_per_second,
            burst_size,
        }
    }
}

use rate_limiting_helpers::*;

// Integration tests for RateLimiter basic functionality
mod rate_limiter_basic {
    use super::*;

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = create_test_rate_limiter(10);

        // Should be able to check capacity immediately after creation
        let capacity = limiter.available_capacity();
        assert!(capacity > 0, "Rate limiter should have initial capacity");
    }

    #[test]
    fn test_rate_limiter_config_creation() {
        let configs = vec![
            (1, None),       // 1 req/sec, default burst
            (10, None),      // 10 req/sec, default burst
            (100, Some(50)), // 100 req/sec, 50 burst
            (5, Some(10)),   // 5 req/sec, 10 burst
        ];

        for (rps, burst) in configs {
            let config = create_test_rate_limiter_config(rps, burst);
            assert_eq!(config.requests_per_second, rps);
            assert_eq!(config.burst_size, burst);

            let limiter = config.build();
            assert!(limiter.available_capacity() > 0);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_basic_checking() {
        let limiter = create_test_rate_limiter(2); // 2 requests per second

        // First few requests should succeed
        assert!(limiter.check().is_ok(), "First request should succeed");
        assert!(limiter.check().is_ok(), "Second request should succeed");

        // Subsequent requests should fail until time passes
        let result = limiter.check();
        match result {
            Ok(_) => {
                // This is also acceptable if burst allows it
            }
            Err(RateLimitError::TooManyRequests { retry_after }) => {
                assert!(
                    retry_after <= Duration::from_secs(1),
                    "Retry after should be reasonable: {retry_after:?}"
                );
            }
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_wait_until_ready() {
        let limiter = create_test_rate_limiter(2); // 2 requests per second

        // Use up initial capacity
        let _ = limiter.check();
        let _ = limiter.check();

        // Wait until ready should not take too long
        let start = Instant::now();
        limiter.wait_until_ready().await;
        let elapsed = start.elapsed();

        // Should wait approximately the rate limit period
        assert!(
            elapsed <= Duration::from_secs(2),
            "Wait should not exceed rate limit period significantly: {elapsed:?}"
        );
    }
}

// Integration tests for rate limiting with Azure Management Client
mod azure_client_rate_limiting {
    use super::*;

    #[test]
    fn test_azure_client_with_rate_limiting() {
        let azure_config = create_mock_azure_config();
        let http_client = reqwest::Client::new();

        // Create client with default rate limiting - should succeed
        AzureManagementClient::with_config(http_client, azure_config);
    }

    #[test]
    fn test_azure_client_with_custom_rate_limiting() {
        let azure_config = create_mock_azure_config();
        let http_client = reqwest::Client::new();

        // Create client with custom rate limiting - should succeed
        let rate_config = create_test_rate_limiter_config(5, Some(10));
        AzureManagementClient::with_config(http_client, azure_config).with_rate_limit(rate_config);
    }

    #[tokio::test]
    async fn test_azure_client_rate_limit_behavior() {
        let azure_config = create_mock_azure_config();
        let http_client = reqwest::Client::new();

        // Create client with very restrictive rate limiting for testing
        let rate_config = create_test_rate_limiter_config(1, Some(1)); // 1 req/sec, 1 burst
        let client = AzureManagementClient::with_config(http_client, azure_config)
            .with_rate_limit(rate_config);

        // These calls will fail with mock credentials, but they should respect rate limiting
        let start = Instant::now();

        // Make multiple calls that should be rate limited
        let mut call_times = Vec::new();

        for i in 0..3 {
            let call_start = Instant::now();

            // This will fail with mock credentials, but the timing should show rate limiting
            let _result = client.list_subscriptions("fake-token").await;

            call_times.push(call_start.elapsed());

            // Only log the first few for debugging
            if i < 2 {
                println!("Call {} completed in: {:?}", i + 1, call_times[i]);
            }
        }

        let total_time = start.elapsed();

        // With rate limiting of 1 req/sec, the calls should be spaced out
        // Even though they fail with mock credentials, the rate limiting should add delay
        assert!(
            total_time >= Duration::from_millis(500), // Allow some variance
            "Rate limiting should add delay between calls, total time: {total_time:?}"
        );
    }
}

// Integration tests for concurrent rate limiting
mod concurrent_rate_limiting {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_rate_limit_checks() {
        let limiter = Arc::new(create_test_rate_limiter(10)); // 10 requests per second

        let mut handles = Vec::new();
        let start = Instant::now();

        // Launch multiple concurrent tasks
        for i in 0..20 {
            let limiter_clone = limiter.clone();
            let handle = tokio::spawn(async move {
                let task_start = Instant::now();
                limiter_clone.wait_until_ready().await;
                let wait_time = task_start.elapsed();
                (i, wait_time)
            });
            handles.push(handle);
        }

        // Collect results
        let mut results = Vec::new();
        for handle in handles {
            let result = handle.await.expect("Task should complete");
            results.push(result);
        }

        let total_time = start.elapsed();

        // With 20 tasks and 10 req/sec rate limit, some tasks should wait
        assert!(
            total_time >= Duration::from_millis(500),
            "Concurrent rate limiting should introduce delays, total time: {total_time:?}"
        );

        // Some tasks should have non-zero wait times
        let tasks_that_waited = results
            .iter()
            .filter(|(_, wait)| *wait > Duration::from_millis(10))
            .count();
        assert!(
            tasks_that_waited > 0,
            "Some tasks should have been rate limited and waited"
        );
    }

    #[tokio::test]
    async fn test_rate_limiter_fairness() {
        let limiter = Arc::new(create_test_rate_limiter(5)); // 5 requests per second

        let task_count = 10;
        let mut handles = Vec::new();

        // Launch tasks with small delays to test fairness
        for i in 0..task_count {
            let limiter_clone = limiter.clone();
            let handle = tokio::spawn(async move {
                // Small staggered start to test ordering
                sleep(Duration::from_millis(i * 10)).await;

                let start = Instant::now();
                limiter_clone.wait_until_ready().await;
                let wait_time = start.elapsed();

                (i, wait_time)
            });
            handles.push(handle);
        }

        // Collect results
        let mut results = Vec::new();
        for handle in handles {
            let result = handle.await.expect("Task should complete");
            results.push(result);
        }

        // Sort by task ID to see the pattern
        results.sort_by_key(|(id, _)| *id);

        // Later tasks should generally wait longer (fairness)
        let early_wait = results[0].1;
        let late_wait = results[task_count as usize - 1].1;

        // This is a rough fairness check - later tasks should wait at least as long
        assert!(
            late_wait >= early_wait,
            "Later tasks should wait at least as long as earlier tasks. Early: {early_wait:?}, Late: {late_wait:?}"
        );
    }
}

// Integration tests for rate limiting error handling
mod rate_limiting_error_handling {
    use super::*;

    #[test]
    fn test_rate_limit_error_display() {
        let errors = vec![
            RateLimitError::TooManyRequests {
                retry_after: Duration::from_secs(1),
            },
            RateLimitError::TooManyRequests {
                retry_after: Duration::from_millis(500),
            },
            RateLimitError::TooManyRequests {
                retry_after: Duration::from_secs(60),
            },
        ];

        for error in errors {
            let error_string = error.to_string();
            assert!(
                !error_string.is_empty(),
                "Rate limit error display should not be empty"
            );

            match error {
                RateLimitError::TooManyRequests { retry_after: _ } => {
                    assert!(error_string.contains("Too many requests"));
                    assert!(error_string.contains("retry after"));
                    // Should contain some representation of the duration
                    assert!(
                        error_string.contains("ms") || error_string.contains("s"),
                        "Error should contain duration info: {error_string}"
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_with_different_burst_sizes() {
        let configs = vec![
            (5, Some(1)),  // Low burst
            (5, Some(5)),  // Default burst
            (5, Some(10)), // High burst
        ];

        for (rps, burst) in configs {
            let config = create_test_rate_limiter_config(rps, burst);
            let limiter = config.build();

            // Test rapid succession of requests
            let mut success_count = 0;

            for _ in 0..15 {
                if limiter.check().is_ok() {
                    success_count += 1;
                }
            }

            // Higher burst should allow more immediate successes
            let expected_min = burst.unwrap_or(rps).min(15);
            assert!(
                success_count >= expected_min as i32 / 2, // Allow some variance
                "Burst size {burst:?} should allow approximately {expected_min} immediate requests, got {success_count}"
            );
        }
    }
}

// Performance and stress tests for rate limiting
mod rate_limiting_performance {
    use super::*;

    #[test]
    fn test_rate_limiter_check_performance() {
        let limiter = create_test_rate_limiter(1000); // High rate for performance testing

        let start = Instant::now();

        // Perform many rate limit checks
        let mut success_count = 0;
        for _ in 0..10000 {
            if limiter.check().is_ok() {
                success_count += 1;
            }
        }

        let duration = start.elapsed();

        // Rate limit checks should be very fast
        assert!(
            duration < Duration::from_millis(500),
            "10000 rate limit checks should be fast, took: {duration:?}"
        );

        // With high rate limit, most should succeed initially
        assert!(
            success_count > 0,
            "Some rate limit checks should succeed with high rate limit"
        );
    }

    #[tokio::test]
    async fn test_rate_limiter_under_load() {
        let limiter = Arc::new(create_test_rate_limiter(50)); // 50 requests per second

        let task_count = 100;
        let mut handles = Vec::new();
        let start = Instant::now();

        // Launch many concurrent tasks
        for i in 0..task_count {
            let limiter_clone = limiter.clone();
            let handle = tokio::spawn(async move {
                let start = Instant::now();
                limiter_clone.wait_until_ready().await;
                let wait_time = start.elapsed();
                (i, wait_time)
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        let mut completed_count = 0;
        for handle in handles {
            if handle.await.is_ok() {
                completed_count += 1;
            }
        }

        let total_time = start.elapsed();

        // All tasks should complete
        assert_eq!(
            completed_count, task_count,
            "All tasks should complete under load"
        );

        // Should complete within reasonable time (100 tasks at 50 req/sec = ~2 seconds minimum)
        assert!(
            total_time < Duration::from_secs(10),
            "Load test should complete within reasonable time: {total_time:?}"
        );

        // Should take at least the theoretical minimum time
        assert!(
            total_time >= Duration::from_millis(1000), // Allow more variance for system timing
            "Load test should respect rate limiting, took: {total_time:?}"
        );
    }

    #[test]
    fn test_rate_limiter_config_build_performance() {
        let start = Instant::now();

        // Build many rate limiters
        for i in 1..1000 {
            let config = create_test_rate_limiter_config(i % 100 + 1, Some(i % 50 + 1));
            let _limiter = config.build();
        }

        let duration = start.elapsed();

        // Building rate limiters should be fast
        assert!(
            duration < Duration::from_millis(100),
            "Building 1000 rate limiters should be fast, took: {duration:?}"
        );
    }
}

// Integration tests for rate limiting edge cases
mod rate_limiting_edge_cases {
    use super::*;

    #[test]
    fn test_rate_limiter_zero_requests_per_second() {
        // This should panic or fail gracefully
        let result = std::panic::catch_unwind(|| create_test_rate_limiter(0));

        assert!(
            result.is_err(),
            "Rate limiter with 0 requests per second should fail"
        );
    }

    #[test]
    fn test_rate_limiter_very_high_rate() {
        let limiter = create_test_rate_limiter(u32::MAX);

        // Should be able to create limiter with very high rate
        assert!(limiter.available_capacity() > 0);

        // Most checks should succeed
        let mut success_count = 0;
        for _ in 0..100 {
            if limiter.check().is_ok() {
                success_count += 1;
            }
        }

        assert!(
            success_count > 50,
            "Very high rate limit should allow most requests"
        );
    }

    #[tokio::test]
    async fn test_rate_limiter_clone_behavior() {
        let original = create_test_rate_limiter(5);
        let cloned = original.clone();

        // Use up capacity on original
        for _ in 0..10 {
            let _ = original.check();
        }

        // Clone should share the same rate limiting state
        let original_result = original.check();
        let cloned_result = cloned.check();

        // Both should have the same behavior (both fail or both succeed)
        match (original_result, cloned_result) {
            (Ok(_), Ok(_)) => {
                // Both succeeded - rate limit not hit yet
            }
            (Err(_), Err(_)) => {
                // Both rate limited - expected behavior
            }
            _ => {
                panic!("Original and cloned rate limiters should have consistent behavior");
            }
        }
    }
}
