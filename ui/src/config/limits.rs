/// Global hard limits for Azure Service Bus operations
/// Azure Service Bus hard limit for batch operations
pub const AZURE_SERVICE_BUS_MAX_BATCH_SIZE: u32 = 2048;

/// Maximum reasonable timeout for operations (10 minutes)
pub const MAX_OPERATION_TIMEOUT_SECS: u64 = 600;

/// Maximum reasonable DLQ batch size
pub const MAX_DLQ_BATCH_SIZE: u32 = 100;

/// Maximum reasonable buffer percentage (50% = 0.5)
pub const MAX_BUFFER_PERCENTAGE: f64 = 0.5;

/// Maximum reasonable minimum buffer size
pub const MAX_MIN_BUFFER_SIZE: usize = 500;

/// Bulk operation limits (min count is now handled by server's BatchConfig)
pub const BULK_OPERATION_MAX_COUNT: usize = 1000;

/// Maximum threshold for triggering auto-reload after bulk operations
pub const MAX_AUTO_RELOAD_THRESHOLD: usize = 100;

/// Maximum small deletion threshold for backfill operations
pub const MAX_SMALL_DELETION_THRESHOLD: usize = 20;

/// Maximum chunk size for bulk processing
pub const MAX_BULK_CHUNK_SIZE: usize = 500;

/// Maximum processing time for bulk operations (seconds)
pub const MAX_BULK_PROCESSING_TIME_SECS: u64 = 120;

/// Maximum lock timeout for lock operations
pub const MAX_LOCK_TIMEOUT_SECS: u64 = 30;

/// Maximum multiplier for calculating max messages to process
pub const MAX_MESSAGES_MULTIPLIER: usize = 10;

/// Minimum messages to process in bulk operations
pub const MIN_MESSAGES_TO_PROCESS_LIMIT: usize = 10;

/// Maximum messages to process in bulk operations
pub const MAX_MESSAGES_TO_PROCESS_LIMIT: usize = 5000;
