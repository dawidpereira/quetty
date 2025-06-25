use serde::Deserialize;

/// Dead Letter Queue (DLQ) configuration
#[derive(Debug, Deserialize)]
pub struct DLQConfig {
    /// Batch size for receiving messages in DLQ operations (default: 10)
    dlq_batch_size: Option<u32>,
}

impl DLQConfig {
    /// Get the DLQ batch size
    pub fn batch_size(&self) -> u32 {
        self.dlq_batch_size.unwrap_or(10)
    }
}
