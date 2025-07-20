use quetty::app::managers::queue_stats_manager::QueueStatsManager;
use quetty::app::updates::messages::pagination::{MessagePaginationState, QueueStatsCache};
use quetty_server::model::{BodyData, MessageModel, MessageState};
use quetty_server::service_bus_manager::QueueType;

/// Create a test message
fn create_test_message(id: &str, sequence: i64, body: &str) -> MessageModel {
    MessageModel::new(
        sequence,
        id.to_string(),
        azure_core::date::OffsetDateTime::now_utc(),
        1,
        MessageState::Active,
        BodyData::RawString(body.to_string()),
    )
}

/// Create multiple test messages
fn create_test_messages(count: usize, start_sequence: i64) -> Vec<MessageModel> {
    (0..count)
        .map(|i| {
            create_test_message(
                &format!("msg-{}", start_sequence + i as i64),
                start_sequence + i as i64,
                &format!("Test message body {i}"),
            )
        })
        .collect()
}

/// Create test queue statistics cache using the constructor
fn create_test_stats_cache(queue_name: &str, active: u64, dlq: u64) -> QueueStatsCache {
    QueueStatsCache::new(queue_name.to_string(), active, dlq)
}

#[test]
fn test_pagination_basic_functionality() {
    let mut pagination = MessagePaginationState::default();

    // Test initial state
    assert_eq!(pagination.current_page, 0);
    assert_eq!(pagination.total_messages(), 0);
    assert!(!pagination.is_loading());

    // Add messages
    let messages = create_test_messages(25, 1);
    pagination.append_messages(messages);

    assert_eq!(pagination.total_messages(), 25);
}

#[test]
fn test_statistics_cache_functionality() {
    let mut stats_manager = QueueStatsManager::new();

    // Test initial state
    assert!(!stats_manager.has_valid_cache("test-queue"));

    // Add cache
    let cache = create_test_stats_cache("test-queue", 358, 42);
    stats_manager.update_stats_cache(cache);

    assert!(stats_manager.has_valid_cache("test-queue"));

    let cached_stats = stats_manager.get_cached_stats("test-queue").unwrap();
    assert_eq!(cached_stats.active_count, 358);
    assert_eq!(cached_stats.dlq_count, 42);
}

#[test]
fn test_pagination_and_statistics_integration() {
    let mut pagination = MessagePaginationState::default();
    let mut stats_manager = QueueStatsManager::new();

    // Setup pagination with messages
    let messages = create_test_messages(50, 1);
    pagination.append_messages(messages);

    // Setup statistics
    let cache = create_test_stats_cache("test-queue", 500, 25);
    stats_manager.update_stats_cache(cache);

    // Verify both systems work together
    assert_eq!(pagination.total_messages(), 50);
    assert!(stats_manager.has_valid_cache("test-queue"));

    // Test page navigation
    let page_size = 10;
    pagination.current_page = 2;
    let page_messages = pagination.get_current_page_messages(page_size);
    assert_eq!(page_messages.len(), 10);

    // Test statistics retrieval
    let stats = stats_manager.get_cached_stats("test-queue").unwrap();
    assert_eq!(stats.get_count_for_type(&QueueType::Main), 500);
    assert_eq!(stats.get_count_for_type(&QueueType::DeadLetter), 25);
}

#[test]
fn test_cache_expiration_workflow() {
    let mut stats_manager = QueueStatsManager::new();

    // Add cache (fresh by default when created)
    let cache = create_test_stats_cache("test-queue", 100, 10);
    assert!(!cache.is_expired());
    stats_manager.update_stats_cache(cache);

    assert!(stats_manager.has_valid_cache("test-queue"));

    // Test that age calculation works
    let cached_stats = stats_manager.get_cached_stats("test-queue").unwrap();
    assert!(cached_stats.age_seconds() >= 0);
}

#[test]
fn test_queue_switching_simulation() {
    let mut pagination = MessagePaginationState::default();
    let mut stats_manager = QueueStatsManager::new();

    // Setup for queue 1
    let messages1 = create_test_messages(15, 1);
    pagination.append_messages(messages1);
    let cache1 = create_test_stats_cache("queue-1", 100, 10);
    stats_manager.update_stats_cache(cache1);

    assert_eq!(pagination.total_messages(), 15);
    assert!(stats_manager.has_valid_cache("queue-1"));

    // Simulate queue switch - reset pagination
    pagination.reset();

    // Setup for queue 2
    let messages2 = create_test_messages(8, 101);
    pagination.append_messages(messages2);
    let cache2 = create_test_stats_cache("queue-2", 200, 20);
    stats_manager.update_stats_cache(cache2);

    // Verify new state
    assert_eq!(pagination.total_messages(), 8);
    assert_eq!(pagination.current_page, 0);

    // Both queue caches should be maintained
    assert!(stats_manager.has_valid_cache("queue-1"));
    assert!(stats_manager.has_valid_cache("queue-2"));
}

#[test]
fn test_cache_invalidation() {
    let mut stats_manager = QueueStatsManager::new();

    // Add cache for a queue
    let cache = create_test_stats_cache("test-queue", 100, 50);
    stats_manager.update_stats_cache(cache);
    assert!(stats_manager.has_valid_cache("test-queue"));

    // Invalidate the cache
    stats_manager.invalidate_stats_cache_for_queue("test-queue");
    assert!(!stats_manager.has_valid_cache("test-queue"));

    // Invalidating non-existent queue should not panic
    stats_manager.invalidate_stats_cache_for_queue("nonexistent-queue");
}

#[test]
fn test_message_model_creation() {
    let message = create_test_message("test-id", 123, "test body");

    assert_eq!(message.sequence, 123);
    assert_eq!(message.id, "test-id");
    assert_eq!(message.delivery_count, 1);
    assert_eq!(message.state, MessageState::Active);

    match &message.body {
        BodyData::RawString(body) => assert_eq!(body, "test body"),
        _ => panic!("Expected RawString body"),
    }
}

#[test]
fn test_pagination_page_navigation() {
    let mut pagination = MessagePaginationState::default();

    // Add enough messages for multiple pages
    let messages = create_test_messages(25, 1);
    pagination.append_messages(messages);

    let page_size = 10;
    pagination.update(page_size);

    // Test first page
    assert_eq!(pagination.get_current_page_messages(page_size).len(), 10);

    // Navigate to second page
    pagination.current_page = 1;
    assert_eq!(pagination.get_current_page_messages(page_size).len(), 10);

    // Navigate to last page (partial)
    pagination.current_page = 2;
    assert_eq!(pagination.get_current_page_messages(page_size).len(), 5);
}
