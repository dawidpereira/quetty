use crate::app::updates::messages::pagination::QueueStatsCache;
use std::collections::HashMap;

/// Manages queue statistics caching and retrieval
#[derive(Debug)]
pub struct QueueStatsManager {
    /// Cache for queue statistics - supports multiple queues
    stats_cache: HashMap<String, QueueStatsCache>,
}

impl QueueStatsManager {
    /// Create a new queue statistics manager
    pub fn new() -> Self {
        Self {
            stats_cache: HashMap::new(),
        }
    }

    /// Generate cache key for a queue name using current authentication method
    fn make_cache_key_for_queue(&self, queue_name: &str) -> String {
        let config = crate::config::get_config_or_panic();
        crate::app::updates::messages::pagination::QueueStatsCache::make_cache_key(
            queue_name,
            &config.azure_ad().auth_method,
        )
    }

    /// Check if stats cache is expired for current queue
    pub fn is_stats_cache_expired(&self, queue_name: &str) -> bool {
        let cache_key = self.make_cache_key_for_queue(queue_name);
        match self.stats_cache.get(&cache_key) {
            Some(cache) => cache.is_expired(),
            None => true,
        }
    }

    /// Update stats cache
    pub fn update_stats_cache(&mut self, cache: QueueStatsCache) {
        let cache_key = cache.cache_key();
        log::info!(
            "Updating cache for {} (auth: {}): active={}, dlq={}",
            cache.queue_name,
            cache.auth_method,
            cache.active_count,
            cache.dlq_count
        );
        self.stats_cache.insert(cache_key, cache);
    }

    /// Get cached stats if valid for specific queue
    pub fn get_cached_stats(&self, queue_name: &str) -> Option<&QueueStatsCache> {
        let cache_key = self.make_cache_key_for_queue(queue_name);
        self.stats_cache.get(&cache_key).filter(|c| !c.is_expired())
    }

    /// Invalidate stats cache for specific queue
    pub fn invalidate_stats_cache_for_queue(&mut self, queue_name: &str) {
        let cache_key = self.make_cache_key_for_queue(queue_name);
        let config = crate::config::get_config_or_panic();
        if self.stats_cache.remove(&cache_key).is_some() {
            log::debug!(
                "Invalidated stats cache for queue: {queue_name} (auth: {})",
                config.azure_ad().auth_method
            );
        }
    }

    /// Check if we have valid (non-expired) cache for queue
    pub fn has_valid_cache(&self, queue_name: &str) -> bool {
        let cache_key = self.make_cache_key_for_queue(queue_name);
        if let Some(cache) = self.stats_cache.get(&cache_key) {
            !cache.is_expired()
        } else {
            false
        }
    }

    /// Clear all cached statistics
    pub fn clear_all_cache(&mut self) {
        log::info!("Clearing all queue statistics cache");
        self.stats_cache.clear();
    }
}

impl Default for QueueStatsManager {
    fn default() -> Self {
        Self::new()
    }
}
