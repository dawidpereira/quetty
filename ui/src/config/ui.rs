use serde::Deserialize;

/// UI-specific configuration
#[derive(Debug, Deserialize, Default)]
pub struct UIConfig {
    /// Duration between animation frames for loading indicators (default: 100ms)
    ui_loading_frame_duration_ms: Option<u64>,
    /// Age threshold in seconds for displaying queue statistics age (default: 60s)
    queue_stats_age_threshold_seconds: Option<u64>,
}

impl UIConfig {
    /// Get the loading frame duration in milliseconds
    pub fn loading_frame_duration_ms(&self) -> u64 {
        self.ui_loading_frame_duration_ms.unwrap_or(100)
    }

    /// Get the queue statistics age threshold in seconds
    pub fn queue_stats_age_threshold_seconds(&self) -> u64 {
        self.queue_stats_age_threshold_seconds.unwrap_or(60)
    }
}
