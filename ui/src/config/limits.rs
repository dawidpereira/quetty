/// Global hard limits for Azure Service Bus operations
/// Azure Service Bus hard limit for batch operations
pub const AZURE_SERVICE_BUS_MAX_BATCH_SIZE: u32 = 2048;

/// Maximum reasonable timeout for operations (20 minutes for large operations)
pub const MAX_OPERATION_TIMEOUT_SECS: u64 = 1200;

/// Maximum chunk size for bulk processing (conservative for stability)
pub const MAX_BULK_CHUNK_SIZE: usize = 500;

/// Maximum processing time for bulk operations (seconds) - increased for large batches
pub const MAX_BULK_PROCESSING_TIME_SECS: u64 = 300;

/// Maximum lock timeout for lock operations
pub const MAX_LOCK_TIMEOUT_SECS: u64 = 30;

/// Maximum messages to process in bulk operations
pub const MAX_MESSAGES_TO_PROCESS_LIMIT: usize = 10_000;

/// Page size configuration limits
/// Minimum page size for message display
pub const MIN_PAGE_SIZE: u32 = 100;

/// Maximum page size for message display
pub const MAX_PAGE_SIZE: u32 = 1000;

/// Queue statistics configuration limits
/// Minimum TTL for queue statistics cache (30 seconds)
pub const MIN_QUEUE_STATS_CACHE_TTL_SECONDS: u64 = 30;

/// Maximum TTL for queue statistics cache (1 hour)
pub const MAX_QUEUE_STATS_CACHE_TTL_SECONDS: u64 = 3600;
