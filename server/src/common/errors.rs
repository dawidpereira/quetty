use thiserror::Error;

/// HTTP-related errors with detailed context for network operations.
///
/// This enum provides comprehensive error classification for HTTP operations
/// throughout the application, including Azure API calls, authentication requests,
/// and other network operations. Each error variant includes relevant context
/// to aid in debugging and error handling.
///
/// # Error Categories
///
/// ## Client Configuration Errors
/// - [`ClientCreation`] - HTTP client initialization failures
///
/// ## Request Execution Errors
/// - [`RequestFailed`] - General request failures with URL and reason
/// - [`Timeout`] - Request timeout with duration and target URL
/// - [`InvalidResponse`] - Unexpected response format or content
///
/// ## Rate Limiting and Service Errors
/// - [`RateLimited`] - Rate limiting with retry timing information
///
/// # Examples
///
/// ## Basic Error Handling
/// ```no_run
/// use server::common::errors::HttpError;
///
/// async fn handle_http_error(error: HttpError) {
///     match error {
///         HttpError::Timeout { url, seconds } => {
///             eprintln!("Request to {} timed out after {}s", url, seconds);
///             // Implement retry with longer timeout
///         }
///         HttpError::RateLimited { retry_after_seconds } => {
///             println!("Rate limited. Retrying after {}s", retry_after_seconds);
///             // Wait and retry
///         }
///         HttpError::RequestFailed { url, reason } => {
///             eprintln!("Request to {} failed: {}", url, reason);
///             // Log and handle specific failure
///         }
///         HttpError::ClientCreation { reason } => {
///             eprintln!("Failed to create HTTP client: {}", reason);
///             // Reinitialize client with different configuration
///         }
///         HttpError::InvalidResponse { expected, actual } => {
///             eprintln!("Invalid response: expected {}, got {}", expected, actual);
///             // Handle unexpected response format
///         }
///     }
/// }
/// ```
///
/// ## Retry Logic Implementation
/// ```no_run
/// use server::common::errors::HttpError;
/// use std::time::Duration;
/// use tokio::time::sleep;
///
/// async fn http_request_with_retry<T>(
///     request_fn: impl Fn() -> Result<T, HttpError>
/// ) -> Result<T, HttpError> {
///     let mut attempts = 0;
///     let max_attempts = 3;
///
///     loop {
///         attempts += 1;
///
///         match request_fn() {
///             Ok(result) => return Ok(result),
///             Err(HttpError::RateLimited { retry_after_seconds }) => {
///                 if attempts < max_attempts {
///                     sleep(Duration::from_secs(retry_after_seconds)).await;
///                     continue;
///                 }
///                 return Err(HttpError::RateLimited { retry_after_seconds });
///             }
///             Err(HttpError::Timeout { url, seconds }) => {
///                 if attempts < max_attempts {
///                     // Exponential backoff for timeouts
///                     sleep(Duration::from_secs(2_u64.pow(attempts))).await;
///                     continue;
///                 }
///                 return Err(HttpError::Timeout { url, seconds });
///             }
///             Err(other) => return Err(other), // Don't retry client errors
///         }
///     }
/// }
/// ```
///
/// ## Azure API Error Handling
/// ```no_run
/// use server::common::errors::HttpError;
///
/// async fn call_azure_api(endpoint: &str) -> Result<String, HttpError> {
///     // Simulated Azure API call
///     match make_request(endpoint).await {
///         Ok(response) => Ok(response),
///         Err(e) => {
///             // Convert to structured HttpError
///             Err(HttpError::RequestFailed {
///                 url: endpoint.to_string(),
///                 reason: e.to_string(),
///             })
///         }
///     }
/// }
///
/// // Usage with error context
/// let result = call_azure_api("https://management.azure.com/subscriptions").await;
/// match result {
///     Ok(data) => println!("API call successful: {}", data),
///     Err(HttpError::RequestFailed { url, reason }) => {
///         if reason.contains("401") {
///             // Handle authentication error
///             println!("Authentication required for {}", url);
///         } else if reason.contains("404") {
///             // Handle resource not found
///             println!("Resource not found: {}", url);
///         } else {
///             // Handle other errors
///             println!("Request failed: {} - {}", url, reason);
///         }
///     }
///     Err(other) => {
///         println!("HTTP error: {}", other);
///     }
/// }
/// ```
///
/// # Integration Patterns
///
/// ## Error Conversion
/// This error type is designed to be easily converted to higher-level error types:
///
/// ```no_run
/// use server::common::errors::HttpError;
/// use server::service_bus_manager::ServiceBusError;
///
/// impl From<HttpError> for ServiceBusError {
///     fn from(http_error: HttpError) -> Self {
///         match http_error {
///             HttpError::Timeout { .. } => ServiceBusError::OperationTimeout(http_error.to_string()),
///             HttpError::RateLimited { .. } => ServiceBusError::OperationTimeout(http_error.to_string()),
///             _ => ServiceBusError::ConnectionFailed(http_error.to_string()),
///         }
///     }
/// }
/// ```
///
/// ## Logging Integration
/// ```no_run
/// use server::common::errors::HttpError;
///
/// fn log_http_error(error: &HttpError) {
///     match error {
///         HttpError::RequestFailed { url, reason } => {
///             log::error!("HTTP request failed: url={}, reason={}", url, reason);
///         }
///         HttpError::Timeout { url, seconds } => {
///             log::warn!("HTTP request timeout: url={}, duration={}s", url, seconds);
///         }
///         HttpError::RateLimited { retry_after_seconds } => {
///             log::info!("HTTP rate limited: retry_after={}s", retry_after_seconds);
///         }
///         _ => {
///             log::error!("HTTP error: {}", error);
///         }
///     }
/// }
/// ```
///
/// [`ClientCreation`]: HttpError::ClientCreation
/// [`RequestFailed`]: HttpError::RequestFailed
/// [`Timeout`]: HttpError::Timeout
/// [`InvalidResponse`]: HttpError::InvalidResponse
/// [`RateLimited`]: HttpError::RateLimited
#[derive(Debug, Error)]
pub enum HttpError {
    /// HTTP client initialization failed.
    ///
    /// This error occurs when creating or configuring the HTTP client fails,
    /// typically due to invalid configuration, SSL/TLS setup issues, or
    /// system resource constraints.
    ///
    /// # Fields
    /// - `reason`: Detailed description of the client creation failure
    ///
    /// # Recovery
    /// - Validate HTTP client configuration
    /// - Check system resources and network settings
    /// - Retry with alternative client configuration
    #[error("HTTP client creation failed: {reason}")]
    ClientCreation { reason: String },

    /// HTTP request execution failed.
    ///
    /// This is a general request failure that can occur due to various
    /// reasons including network issues, server errors, authentication
    /// problems, or malformed requests.
    ///
    /// # Fields
    /// - `url`: The URL that was being requested
    /// - `reason`: Detailed description of the failure
    ///
    /// # Recovery
    /// - Check network connectivity
    /// - Validate request parameters and authentication
    /// - Implement retry logic for transient failures
    #[error("Request failed: {url} - {reason}")]
    RequestFailed { url: String, reason: String },

    /// HTTP request timed out.
    ///
    /// This error occurs when a request takes longer than the configured
    /// timeout duration. This can happen due to slow network conditions,
    /// overloaded servers, or network connectivity issues.
    ///
    /// # Fields
    /// - `url`: The URL that timed out
    /// - `seconds`: The timeout duration that was exceeded
    ///
    /// # Recovery
    /// - Retry with longer timeout
    /// - Check network connectivity
    /// - Consider alternative endpoints if available
    #[error("Request timeout after {seconds}s: {url}")]
    Timeout { url: String, seconds: u64 },

    /// Rate limiting is active for HTTP requests.
    ///
    /// This error occurs when the server has rate-limited the client
    /// due to too many requests in a short period. The server provides
    /// guidance on when to retry.
    ///
    /// # Fields
    /// - `retry_after_seconds`: Duration to wait before retrying
    ///
    /// # Recovery
    /// - Wait for the specified duration before retrying
    /// - Implement request throttling to prevent future rate limiting
    /// - Consider using exponential backoff for subsequent requests
    #[error("Rate limit exceeded: retry after {retry_after_seconds}s")]
    RateLimited { retry_after_seconds: u64 },

    /// Received response doesn't match expected format.
    ///
    /// This error occurs when the server returns a response that doesn't
    /// match the expected format, content type, or structure. This can
    /// indicate API changes, server errors, or client-side parsing issues.
    ///
    /// # Fields
    /// - `expected`: Description of what was expected
    /// - `actual`: Description of what was actually received
    ///
    /// # Recovery
    /// - Validate API endpoint and version compatibility
    /// - Check response parsing logic
    /// - Consider graceful degradation for unexpected responses
    #[error("Invalid response: expected {expected}, got {actual}")]
    InvalidResponse { expected: String, actual: String },
}

/// Cache-related errors for token and data caching operations.
///
/// This enum provides detailed error classification for caching operations
/// throughout the application, particularly for authentication token caching
/// and other temporary data storage. Each error variant includes relevant
/// context to aid in cache management and error recovery.
///
/// # Error Categories
///
/// ## Cache Entry Lifecycle Errors
/// - [`Expired`] - Cache entry has exceeded its time-to-live
/// - [`Miss`] - Requested cache entry doesn't exist
///
/// ## Cache Capacity and Management Errors
/// - [`Full`] - Cache has reached capacity limits
/// - [`OperationFailed`] - General cache operation failures
///
/// # Examples
///
/// ## Token Cache Error Handling
/// ```no_run
/// use server::common::errors::CacheError;
///
/// async fn handle_token_cache_error(error: CacheError, token_key: &str) {
///     match error {
///         CacheError::Expired { key } => {
///             println!("Token expired for key: {}", key);
///             // Trigger token refresh
///             refresh_token(&key).await;
///         }
///         CacheError::Miss { key } => {
///             println!("Token not found in cache: {}", key);
///             // Authenticate and cache new token
///             authenticate_and_cache(&key).await;
///         }
///         CacheError::Full { key } => {
///             println!("Cache full, cannot store token for: {}", key);
///             // Implement cache eviction strategy
///             evict_oldest_entries().await;
///             retry_cache_operation(&key).await;
///         }
///         CacheError::OperationFailed { reason } => {
///             eprintln!("Cache operation failed: {}", reason);
///             // Log error and use alternative storage
///             fallback_to_memory_cache(&token_key).await;
///         }
///     }
/// }
/// ```
///
/// ## Cache Management Patterns
/// ```no_run
/// use server::common::errors::CacheError;
///
/// async fn get_or_create_cached_item<T>(
///     cache_key: &str,
///     create_fn: impl Fn() -> Result<T, String>
/// ) -> Result<T, CacheError> {
///     // Try to get from cache first
///     match get_from_cache(cache_key).await {
///         Ok(item) => Ok(item),
///         Err(CacheError::Miss { .. }) => {
///             // Cache miss - create and cache the item
///             match create_fn() {
///                 Ok(item) => {
///                     // Attempt to cache the new item
///                     if let Err(cache_err) = cache_item(cache_key, &item).await {
///                         // Log cache failure but return the item anyway
///                         log::warn!("Failed to cache item: {}", cache_err);
///                     }
///                     Ok(item)
///                 }
///                 Err(create_error) => {
///                     Err(CacheError::OperationFailed {
///                         reason: format!("Item creation failed: {}", create_error)
///                     })
///                 }
///             }
///         }
///         Err(CacheError::Expired { key }) => {
///             // Cache expired - remove and recreate
///             remove_from_cache(&key).await;
///             create_fn().map_err(|e| CacheError::OperationFailed {
///                 reason: format!("Recreation after expiry failed: {}", e)
///             })
///         }
///         Err(other) => Err(other),
///     }
/// }
/// ```
///
/// ## Cache Health Monitoring
/// ```no_run
/// use server::common::errors::CacheError;
///
/// struct CacheMetrics {
///     hits: u64,
///     misses: u64,
///     expirations: u64,
///     failures: u64,
/// }
///
/// fn update_cache_metrics(error: &CacheError, metrics: &mut CacheMetrics) {
///     match error {
///         CacheError::Miss { .. } => {
///             metrics.misses += 1;
///             log::debug!("Cache miss recorded");
///         }
///         CacheError::Expired { .. } => {
///             metrics.expirations += 1;
///             log::debug!("Cache expiration recorded");
///         }
///         CacheError::Full { .. } | CacheError::OperationFailed { .. } => {
///             metrics.failures += 1;
///             log::warn!("Cache failure recorded: {}", error);
///         }
///     }
/// }
///
/// fn calculate_cache_hit_rate(metrics: &CacheMetrics) -> f64 {
///     let total_requests = metrics.hits + metrics.misses;
///     if total_requests == 0 {
///         0.0
///     } else {
///         metrics.hits as f64 / total_requests as f64
///     }
///  }
/// ```
///
/// ## Integration with Authentication
/// ```no_run
/// use server::common::errors::CacheError;
/// use server::auth::TokenRefreshError;
///
/// async fn get_valid_token(user_id: &str) -> Result<String, TokenRefreshError> {
///     match get_cached_token(user_id).await {
///         Ok(token) => Ok(token),
///         Err(CacheError::Miss { .. }) | Err(CacheError::Expired { .. }) => {
///             // Cache miss or expiry - refresh token
///             let new_token = refresh_user_token(user_id).await?;
///
///             // Attempt to cache the new token
///             if let Err(cache_err) = cache_token(user_id, &new_token).await {
///                 log::warn!("Failed to cache refreshed token: {}", cache_err);
///                 // Continue anyway - token is still valid
///             }
///
///             Ok(new_token)
///         }
///         Err(CacheError::OperationFailed { reason }) => {
///             // Cache operation failed - try refresh anyway
///             log::error!("Cache operation failed: {}", reason);
///             refresh_user_token(user_id).await
///         }
///         Err(CacheError::Full { .. }) => {
///             // Cache full - evict and retry
///             evict_expired_tokens().await;
///             match get_cached_token(user_id).await {
///                 Ok(token) => Ok(token),
///                 Err(_) => refresh_user_token(user_id).await,
///             }
///         }
///     }
/// }
/// ```
///
/// # Cache Strategies
///
/// ## Error-Based Cache Management
/// - **Miss**: Create and cache new data
/// - **Expired**: Remove expired entry and recreate
/// - **Full**: Implement LRU or TTL-based eviction
/// - **Operation Failed**: Fall back to direct data access
///
/// ## Performance Considerations
/// - Cache errors should not block critical operations
/// - Implement graceful degradation when cache is unavailable
/// - Monitor cache hit rates and error frequencies
/// - Use appropriate TTL values to balance freshness and performance
///
/// [`Expired`]: CacheError::Expired
/// [`Miss`]: CacheError::Miss
/// [`Full`]: CacheError::Full
/// [`OperationFailed`]: CacheError::OperationFailed
#[derive(Debug, Error)]
pub enum CacheError {
    /// Cache entry has expired and is no longer valid.
    ///
    /// This error occurs when attempting to access a cache entry that
    /// has exceeded its time-to-live (TTL). The entry should be removed
    /// and recreated if needed.
    ///
    /// # Fields
    /// - `key`: The cache key for the expired entry
    ///
    /// # Recovery
    /// - Remove the expired entry from cache
    /// - Recreate the data if needed
    /// - Update cache with fresh data and appropriate TTL
    #[error("Cache entry expired for key: {key}")]
    Expired { key: String },

    /// Requested cache entry was not found.
    ///
    /// This error occurs when attempting to retrieve a cache entry that
    /// doesn't exist. This is a normal condition for cold cache scenarios
    /// or when entries have been evicted.
    ///
    /// # Fields
    /// - `key`: The cache key that was not found
    ///
    /// # Recovery
    /// - Create the data using the original source
    /// - Cache the newly created data for future requests
    /// - Consider pre-warming cache for frequently accessed items
    #[error("Cache miss for key: {key}")]
    Miss { key: String },

    /// Cache has reached its capacity limit.
    ///
    /// This error occurs when attempting to add a new entry to a cache
    /// that has reached its maximum capacity. This requires cache
    /// management strategies like eviction.
    ///
    /// # Fields
    /// - `key`: The cache key that couldn't be added
    ///
    /// # Recovery
    /// - Implement cache eviction strategy (LRU, TTL-based, etc.)
    /// - Remove expired or least recently used entries
    /// - Consider increasing cache capacity if appropriate
    /// - Retry the cache operation after eviction
    #[error("Cache full, unable to add entry for key: {key}")]
    Full { key: String },

    /// General cache operation failure.
    ///
    /// This error represents various cache operation failures that don't
    /// fit other categories, such as I/O errors, serialization failures,
    /// or cache system unavailability.
    ///
    /// # Fields
    /// - `reason`: Detailed description of the operation failure
    ///
    /// # Recovery
    /// - Log the detailed error for debugging
    /// - Fall back to direct data access without caching
    /// - Consider cache system health checks
    /// - Implement retry logic for transient failures
    #[error("Cache operation failed: {reason}")]
    OperationFailed { reason: String },
}

/// Helper trait for adding context to errors
pub trait ErrorContext<T> {
    /// Add context to an error result
    fn context(self, msg: &str) -> Result<T, String>;

    /// Add lazy context to an error result
    fn with_context<F>(self, f: F) -> Result<T, String>
    where
        F: FnOnce() -> String;
}

impl<T, E> ErrorContext<T> for Result<T, E>
where
    E: std::fmt::Display,
{
    fn context(self, msg: &str) -> Result<T, String> {
        self.map_err(|e| format!("{msg}: {e}"))
    }

    fn with_context<F>(self, f: F) -> Result<T, String>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| format!("{}: {e}", f()))
    }
}
