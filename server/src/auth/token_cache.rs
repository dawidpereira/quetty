use super::types::CachedToken;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Thread-safe cache for storing authentication tokens with expiration tracking.
///
/// Provides a simple key-value store for authentication tokens that automatically
/// handles expiration checking and cleanup. The cache is designed to be shared
/// across multiple threads and async tasks safely.
///
/// # Thread Safety
///
/// All operations are thread-safe and can be called concurrently from multiple
/// async tasks. The cache uses RwLock for efficient concurrent read access.
///
/// # Examples
///
/// ```no_run
/// use server::auth::{TokenCache, CachedToken};
/// use std::time::{Duration, Instant};
///
/// let cache = TokenCache::new();
///
/// // Store a token
/// let token = CachedToken {
///     token: "access_token_123".to_string(),
///     expires_at: Instant::now() + Duration::from_secs(3600),
/// };
/// cache.set("user_123".to_string(), token).await;
///
/// // Retrieve a token (returns None if expired)
/// if let Some(token) = cache.get("user_123").await {
///     println!("Token: {}", token);
/// }
/// ```
#[derive(Clone)]
pub struct TokenCache {
    cache: Arc<RwLock<HashMap<String, CachedToken>>>,
}

impl TokenCache {
    /// Creates a new empty token cache.
    ///
    /// # Returns
    ///
    /// A new TokenCache instance ready for use
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Retrieves a valid token from the cache.
    ///
    /// Returns the token only if it exists and has not expired. Expired tokens
    /// are automatically filtered out.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key to look up
    ///
    /// # Returns
    ///
    /// `Some(token)` if a valid token exists, `None` if no token exists or it has expired
    pub async fn get(&self, key: &str) -> Option<String> {
        let cache = self.cache.read().await;
        cache
            .get(key)
            .filter(|token| !token.is_expired())
            .map(|token| token.token.clone())
    }

    /// Stores a token in the cache.
    ///
    /// Overwrites any existing token with the same key.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key to store the token under
    /// * `token` - The cached token with expiration information
    pub async fn set(&self, key: String, token: CachedToken) {
        let mut cache = self.cache.write().await;
        cache.insert(key, token);
    }

    /// Removes a specific token from the cache.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key to remove
    pub async fn invalidate(&self, key: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(key);
    }

    /// Clears all tokens from the cache.
    ///
    /// This is useful for logout operations or when switching authentication contexts.
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Checks if a token needs refresh based on its expiration time.
    ///
    /// Returns `true` if the token doesn't exist, has expired, or is close to expiring.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key to check
    ///
    /// # Returns
    ///
    /// `true` if the token needs refresh, `false` if it's still valid
    pub async fn needs_refresh(&self, key: &str) -> bool {
        let cache = self.cache.read().await;
        cache
            .get(key)
            .map(|token| token.needs_refresh())
            .unwrap_or(true)
    }
}

impl Default for TokenCache {
    fn default() -> Self {
        Self::new()
    }
}
