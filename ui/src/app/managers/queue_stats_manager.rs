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

    /// Check if stats cache is expired for current queue
    pub fn is_stats_cache_expired(&self, queue_name: &str) -> bool {
        match self.stats_cache.get(queue_name) {
            Some(cache) => cache.is_expired(),
            None => true,
        }
    }

    /// Update stats cache
    pub fn update_stats_cache(&mut self, cache: QueueStatsCache) {
        log::info!(
            "Updating cache for {}: active={}, dlq={}",
            cache.queue_name,
            cache.active_count,
            cache.dlq_count
        );
        self.stats_cache.insert(cache.queue_name.clone(), cache);
    }

    /// Get cached stats if valid for specific queue
    pub fn get_cached_stats(&self, queue_name: &str) -> Option<&QueueStatsCache> {
        self.stats_cache.get(queue_name).filter(|c| !c.is_expired())
    }

    /// Invalidate stats cache for specific queue
    pub fn invalidate_stats_cache_for_queue(&mut self, queue_name: &str) {
        if self.stats_cache.remove(queue_name).is_some() {
            log::debug!("Invalidated stats cache for queue: {}", queue_name);
        }
    }

    /// Check if we have valid (non-expired) cache for queue
    pub fn has_valid_cache(&self, queue_name: &str) -> bool {
        if let Some(cache) = self.stats_cache.get(queue_name) {
            !cache.is_expired()
        } else {
            false
        }
    }
}

impl Default for QueueStatsManager {
    fn default() -> Self {
        Self::new()
    }
}
