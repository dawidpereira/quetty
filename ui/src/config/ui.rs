use serde::Deserialize;

/// UI-specific configuration
#[derive(Debug, Deserialize)]
pub struct UIConfig {
    /// Duration between animation frames for loading indicators (default: 100ms)
    ui_loading_frame_duration_ms: Option<u64>,
}

impl UIConfig {
    /// Get the loading frame duration in milliseconds
    pub fn loading_frame_duration_ms(&self) -> u64 {
        self.ui_loading_frame_duration_ms.unwrap_or(100)
    }
}
