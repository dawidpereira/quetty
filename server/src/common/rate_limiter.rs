use governor::{
    Quota, RateLimiter as GovernorRateLimiter,
    clock::{Clock, DefaultClock},
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

/// Rate limiter for API requests
pub struct RateLimiter {
    inner: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
}

impl std::fmt::Debug for RateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RateLimiter")
            .field("available_capacity", &self.available_capacity())
            .finish()
    }
}

impl RateLimiter {
    /// Create a new rate limiter with specified requests per second
    pub fn new(requests_per_second: u32) -> Self {
        let quota = Quota::per_second(
            NonZeroU32::new(requests_per_second).expect("requests_per_second must be > 0"),
        );

        Self {
            inner: Arc::new(GovernorRateLimiter::direct(quota)),
        }
    }

    /// Create a rate limiter with custom quota
    pub fn with_quota(quota: Quota) -> Self {
        Self {
            inner: Arc::new(GovernorRateLimiter::direct(quota)),
        }
    }

    /// Check if a request can proceed
    pub fn check(&self) -> Result<(), RateLimitError> {
        match self.inner.check() {
            Ok(_) => Ok(()),
            Err(not_until) => {
                let wait_duration = not_until.wait_time_from(DefaultClock::default().now());
                Err(RateLimitError::TooManyRequests {
                    retry_after: wait_duration,
                })
            }
        }
    }

    /// Wait until a request can proceed
    pub async fn wait_until_ready(&self) {
        self.inner.until_ready().await;
    }

    /// Get the current available capacity
    pub fn available_capacity(&self) -> u32 {
        // Use binary search to find the maximum available capacity
        let mut low = 0;
        let mut high = 10000; // reasonable upper bound for most use cases

        #[allow(clippy::manual_div_ceil)]
        while low < high {
            let mid = low + (high - low + 1) / 2;
            if let Some(nz) = NonZeroU32::new(mid) {
                if self.inner.check_n(nz).is_ok() {
                    low = mid;
                } else {
                    high = mid - 1;
                }
            } else {
                break;
            }
        }

        low
    }
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

/// Rate limiting errors
#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("Too many requests, retry after {retry_after:?}")]
    TooManyRequests { retry_after: Duration },
}

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// Maximum requests per second
    pub requests_per_second: u32,
    /// Maximum burst size (defaults to requests_per_second)
    pub burst_size: Option<u32>,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 10,
            burst_size: None,
        }
    }
}

impl RateLimiterConfig {
    /// Create a rate limiter from this configuration
    pub fn build(&self) -> RateLimiter {
        let burst_size = self.burst_size.unwrap_or(self.requests_per_second);
        let quota = Quota::per_second(
            NonZeroU32::new(self.requests_per_second).expect("requests_per_second must be > 0"),
        )
        .allow_burst(NonZeroU32::new(burst_size).expect("burst_size must be > 0"));

        RateLimiter::with_quota(quota)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new(2); // 2 requests per second

        // First two requests should succeed
        assert!(limiter.check().is_ok());
        assert!(limiter.check().is_ok());

        // Third request should fail
        assert!(matches!(
            limiter.check(),
            Err(RateLimitError::TooManyRequests { .. })
        ));

        // Wait for rate limit to reset
        sleep(Duration::from_secs(1)).await;

        // Should succeed again
        assert!(limiter.check().is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_wait() {
        let limiter = RateLimiter::new(1); // 1 request per second

        // First request succeeds
        assert!(limiter.check().is_ok());

        // Wait until ready
        let start = std::time::Instant::now();
        limiter.wait_until_ready().await;
        let elapsed = start.elapsed();

        // Should have waited approximately 1 second
        assert!(elapsed >= Duration::from_millis(900));
        assert!(elapsed <= Duration::from_millis(1100));
    }
}
