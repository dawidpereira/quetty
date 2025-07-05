use super::types::CachedToken;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct TokenCache {
    cache: Arc<RwLock<HashMap<String, CachedToken>>>,
}

impl TokenCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        let cache = self.cache.read().await;
        cache
            .get(key)
            .filter(|token| !token.is_expired())
            .map(|token| token.token.clone())
    }

    pub async fn set(&self, key: String, token: CachedToken) {
        let mut cache = self.cache.write().await;
        cache.insert(key, token);
    }

    pub async fn invalidate(&self, key: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(key);
    }

    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

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
