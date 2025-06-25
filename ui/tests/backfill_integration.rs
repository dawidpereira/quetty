use quetty::app::queue_state::QueueState;
use quetty::app::updates::messages::MessagePaginationState;
use quetty::config;
use server::model::{BodyData, MessageModel, MessageState};
use server::service_bus_manager::QueueType;
use std::collections::HashSet;

// Helper modules for backfill integration tests
mod helpers {
    use super::*;

    /// Create a mock MessageModel for testing
    pub fn create_test_message(id: &str, sequence: i64) -> MessageModel {
        // Create a default timestamp using chrono (which azure_core uses)
        use std::time::{Duration, SystemTime};

        // Convert SystemTime to OffsetDateTime for testing
        let timestamp = SystemTime::UNIX_EPOCH + Duration::from_secs(1700000000 + sequence as u64);

        MessageModel {
            sequence,
            id: id.to_string(),
            enqueued_at: timestamp.into(),
            delivery_count: 1,
            state: MessageState::Active,
            body: BodyData::RawString(format!("Test message {}", id)),
        }
    }

    /// Set up pagination with test messages
    pub fn setup_pagination_with_messages(
        queue_state: &mut QueueState,
        messages: Vec<MessageModel>,
    ) {
        queue_state.message_pagination.reset();
        queue_state.message_pagination.all_loaded_messages = messages.clone();
        queue_state.messages = Some(messages.clone());

        // Calculate total_pages_loaded correctly based on message count
        let page_size = config::get_config_or_panic().max_messages() as usize;
        let total_messages = messages.len();
        let total_pages = if total_messages == 0 {
            0
        } else {
            total_messages.div_ceil(page_size)
        };
        queue_state.message_pagination.total_pages_loaded = total_pages;

        // Update last sequence
        if let Some(last_msg) = messages.last() {
            queue_state.message_pagination.last_loaded_sequence = Some(last_msg.sequence);
        }

        queue_state
            .message_pagination
            .update(config::get_config_or_panic().max_messages());
    }

    /// Create a test QueueState with pagination
    pub fn create_test_queue_state(queue_type: QueueType) -> QueueState {
        QueueState {
            current_queue_type: queue_type.clone(),
            current_queue_name: match queue_type {
                QueueType::Main => Some("test-queue".to_string()),
                QueueType::DeadLetter => Some("test-queue/$deadletterqueue".to_string()),
            },
            ..Default::default()
        }
    }

    /// Simulate bulk message removal from pagination state
    pub fn simulate_bulk_remove_messages(
        pagination: &mut MessagePaginationState,
        message_ids: &[&str],
    ) -> usize {
        let initial_count = pagination.all_loaded_messages.len();

        // Convert string IDs to set for lookup
        let ids_to_remove: HashSet<String> = message_ids.iter().map(|s| s.to_string()).collect();

        // Remove messages with matching IDs
        pagination
            .all_loaded_messages
            .retain(|msg| !ids_to_remove.contains(&msg.id));

        let final_count = pagination.all_loaded_messages.len();
        initial_count.saturating_sub(final_count)
    }

    /// Update pagination state after message removal
    pub fn update_pagination_after_removal(pagination: &mut MessagePaginationState) {
        let page_size = config::get_config_or_panic().max_messages();
        let total_messages = pagination.all_loaded_messages.len();

        let new_total_pages = if total_messages == 0 {
            0
        } else {
            total_messages.div_ceil(page_size as usize)
        };

        pagination.total_pages_loaded = new_total_pages;

        // Ensure current page is within bounds
        if new_total_pages == 0 {
            pagination.current_page = 0;
        } else if pagination.current_page >= new_total_pages {
            pagination.current_page = new_total_pages - 1;
        }

        // Update pagination controls
        pagination.has_previous_page = pagination.current_page > 0;
        pagination.has_next_page = pagination.current_page < new_total_pages.saturating_sub(1);
    }

    /// Check if current page is under-filled and needs backfill
    pub fn check_needs_backfill(pagination: &MessagePaginationState) -> (bool, usize) {
        let page_size = config::get_config_or_panic().max_messages() as usize;
        let current_page_messages =
            pagination.get_current_page_messages(config::get_config_or_panic().max_messages());
        let current_page_size = current_page_messages.len();
        let page_is_under_filled = current_page_size < page_size;

        let messages_needed = if page_is_under_filled {
            page_size - current_page_size
        } else {
            0
        };

        (page_is_under_filled, messages_needed)
    }

    /// Simulate adding backfill messages (mirrors ensure_pagination_consistency_after_backfill)
    pub fn add_backfill_messages(
        pagination: &mut MessagePaginationState,
        backfill_messages: Vec<MessageModel>,
    ) {
        // Add the messages to the state
        pagination.all_loaded_messages.extend(backfill_messages);

        // Update last sequence if messages were added
        if let Some(last_msg) = pagination.all_loaded_messages.last() {
            pagination.last_loaded_sequence = Some(last_msg.sequence);
        }

        // Recalculate total_pages_loaded based on new message count (like the real implementation)
        let total_messages = pagination.all_loaded_messages.len();
        let messages_per_page = config::get_config_or_panic().max_messages() as usize;

        let new_total_pages = if total_messages == 0 {
            0
        } else {
            total_messages.div_ceil(messages_per_page)
        };

        pagination.total_pages_loaded = new_total_pages;

        // Ensure current page is within bounds
        if new_total_pages > 0 && pagination.current_page >= new_total_pages {
            pagination.current_page = new_total_pages - 1;
        }

        // Update pagination controls (like the real implementation)
        pagination.has_previous_page = pagination.current_page > 0;
        pagination.has_next_page = pagination.current_page < new_total_pages.saturating_sub(1);
    }
}

use helpers::*;

#[test]
fn test_backfill_after_delete_from_main_queue() {
    let mut queue_state = create_test_queue_state(QueueType::Main);

    // Set up main queue with 7 messages (assuming page size is 10)
    let initial_messages = vec![
        create_test_message("msg1", 1),
        create_test_message("msg2", 2),
        create_test_message("msg3", 3),
        create_test_message("msg4", 4),
        create_test_message("msg5", 5),
        create_test_message("msg6", 6),
        create_test_message("msg7", 7),
    ];

    setup_pagination_with_messages(&mut queue_state, initial_messages);

    // Verify initial state
    assert_eq!(queue_state.message_pagination.all_loaded_messages.len(), 7);
    assert_eq!(queue_state.message_pagination.current_page, 0);

    // Simulate deleting 2 messages from main queue
    let removed_count =
        simulate_bulk_remove_messages(&mut queue_state.message_pagination, &["msg1", "msg3"]);
    assert_eq!(removed_count, 2);
    assert_eq!(queue_state.message_pagination.all_loaded_messages.len(), 5);

    // Update pagination state after removal
    update_pagination_after_removal(&mut queue_state.message_pagination);

    // Check if backfill is needed
    let (needs_backfill, messages_needed) = check_needs_backfill(&queue_state.message_pagination);
    let page_size = config::get_config_or_panic().max_messages() as usize;

    if page_size > 5 {
        assert!(
            needs_backfill,
            "Should need backfill after deleting messages"
        );
        assert_eq!(
            messages_needed,
            page_size - 5,
            "Should need {} messages",
            page_size - 5
        );

        // Simulate backfill response
        let backfill_messages = vec![
            create_test_message("msg8", 8),
            create_test_message("msg9", 9),
        ];

        add_backfill_messages(&mut queue_state.message_pagination, backfill_messages);

        // Verify backfill was applied correctly
        assert_eq!(queue_state.message_pagination.all_loaded_messages.len(), 7);

        // Verify pagination consistency
        assert_eq!(queue_state.message_pagination.current_page, 0);
        assert!(!queue_state.message_pagination.has_previous_page);
    }
}

#[test]
fn test_backfill_after_send_with_delete_from_main() {
    let mut queue_state = create_test_queue_state(QueueType::Main);

    // Set up main queue with 5 messages
    let initial_messages = vec![
        create_test_message("msg1", 1),
        create_test_message("msg2", 2),
        create_test_message("msg3", 3),
        create_test_message("msg4", 4),
        create_test_message("msg5", 5),
    ];

    setup_pagination_with_messages(&mut queue_state, initial_messages);

    // Simulate sending 3 messages to DLQ with delete from main
    let removed_count = simulate_bulk_remove_messages(
        &mut queue_state.message_pagination,
        &["msg1", "msg2", "msg4"],
    );
    assert_eq!(removed_count, 3);
    assert_eq!(queue_state.message_pagination.all_loaded_messages.len(), 2);

    // Update pagination state
    update_pagination_after_removal(&mut queue_state.message_pagination);

    // Check if backfill is needed
    let (needs_backfill, messages_needed) = check_needs_backfill(&queue_state.message_pagination);
    let page_size = config::get_config_or_panic().max_messages() as usize;

    if page_size > 2 {
        assert!(
            needs_backfill,
            "Should need backfill after send with delete"
        );
        assert_eq!(
            messages_needed,
            page_size - 2,
            "Should need {} messages",
            page_size - 2
        );

        // Simulate backfill
        let mut backfill_messages = Vec::new();
        for i in 1..=messages_needed {
            backfill_messages.push(create_test_message(
                &format!("backfill{}", i),
                10 + i as i64,
            ));
        }

        add_backfill_messages(&mut queue_state.message_pagination, backfill_messages);

        // Verify page is now properly filled
        let final_count = queue_state.message_pagination.all_loaded_messages.len();
        assert_eq!(final_count, 2 + messages_needed);
    }
}

#[test]
fn test_backfill_after_delete_from_dlq() {
    let mut queue_state = create_test_queue_state(QueueType::DeadLetter);

    // Set up DLQ with 6 messages
    let initial_messages = vec![
        create_test_message("dlq1", 1),
        create_test_message("dlq2", 2),
        create_test_message("dlq3", 3),
        create_test_message("dlq4", 4),
        create_test_message("dlq5", 5),
        create_test_message("dlq6", 6),
    ];

    setup_pagination_with_messages(&mut queue_state, initial_messages);

    // Verify we're in DLQ
    assert_eq!(queue_state.current_queue_type, QueueType::DeadLetter);

    // Simulate deleting 3 messages from DLQ
    let removed_count = simulate_bulk_remove_messages(
        &mut queue_state.message_pagination,
        &["dlq1", "dlq3", "dlq5"],
    );
    assert_eq!(removed_count, 3);
    assert_eq!(queue_state.message_pagination.all_loaded_messages.len(), 3);

    // Update pagination state
    update_pagination_after_removal(&mut queue_state.message_pagination);

    // Check if backfill is needed
    let (needs_backfill, messages_needed) = check_needs_backfill(&queue_state.message_pagination);
    let page_size = config::get_config_or_panic().max_messages() as usize;

    if page_size > 3 {
        assert!(needs_backfill, "DLQ should also support backfill");
        assert_eq!(messages_needed, page_size - 3);

        // Simulate backfill for DLQ
        let backfill_messages = vec![
            create_test_message("dlq7", 7),
            create_test_message("dlq8", 8),
        ];

        add_backfill_messages(&mut queue_state.message_pagination, backfill_messages);
        assert_eq!(queue_state.message_pagination.all_loaded_messages.len(), 5);
    }
}

#[test]
fn test_backfill_after_resend_with_delete_from_dlq() {
    let mut queue_state = create_test_queue_state(QueueType::DeadLetter);

    // Set up DLQ with 8 messages
    let initial_messages = vec![
        create_test_message("dlq1", 1),
        create_test_message("dlq2", 2),
        create_test_message("dlq3", 3),
        create_test_message("dlq4", 4),
        create_test_message("dlq5", 5),
        create_test_message("dlq6", 6),
        create_test_message("dlq7", 7),
        create_test_message("dlq8", 8),
    ];

    setup_pagination_with_messages(&mut queue_state, initial_messages);

    // Simulate resending 4 messages from DLQ with delete (removes from DLQ)
    let removed_count = simulate_bulk_remove_messages(
        &mut queue_state.message_pagination,
        &["dlq1", "dlq3", "dlq5", "dlq7"],
    );
    assert_eq!(removed_count, 4);
    assert_eq!(queue_state.message_pagination.all_loaded_messages.len(), 4);

    // Update pagination state
    update_pagination_after_removal(&mut queue_state.message_pagination);

    // Check if backfill is needed
    let (needs_backfill, _messages_needed) = check_needs_backfill(&queue_state.message_pagination);
    let page_size = config::get_config_or_panic().max_messages() as usize;

    if page_size > 4 {
        assert!(
            needs_backfill,
            "Should need backfill after resend with delete"
        );

        // Simulate backfill
        let backfill_messages = vec![
            create_test_message("dlq9", 9),
            create_test_message("dlq10", 10),
            create_test_message("dlq11", 11),
        ];

        add_backfill_messages(&mut queue_state.message_pagination, backfill_messages);
        assert_eq!(queue_state.message_pagination.all_loaded_messages.len(), 7);
    }
}

#[test]
fn test_small_deletion_threshold_logic() {
    let mut queue_state = create_test_queue_state(QueueType::Main);

    // Set up messages close to page size
    let page_size = config::get_config_or_panic().max_messages() as usize;
    let initial_count = page_size - 2; // Start 2 messages under capacity

    let mut initial_messages = Vec::new();
    for i in 1..=initial_count {
        initial_messages.push(create_test_message(&format!("msg{}", i), i as i64));
    }

    setup_pagination_with_messages(&mut queue_state, initial_messages);

    // Get small deletion threshold from config
    let small_deletion_threshold = config::get_config_or_panic()
        .batch()
        .small_deletion_threshold();

    // Delete exactly the small deletion threshold number of messages
    let mut to_remove = Vec::new();
    let mut remove_strings = Vec::new();
    for i in 1..=small_deletion_threshold {
        let remove_string = format!("msg{}", i);
        remove_strings.push(remove_string);
    }
    for s in &remove_strings {
        to_remove.push(&s[..]);
    }

    let removed_count =
        simulate_bulk_remove_messages(&mut queue_state.message_pagination, &to_remove);
    assert_eq!(removed_count, small_deletion_threshold);

    // Update pagination state
    update_pagination_after_removal(&mut queue_state.message_pagination);

    // Check backfill logic
    let (needs_backfill, _messages_needed) = check_needs_backfill(&queue_state.message_pagination);
    let remaining_messages = initial_count - small_deletion_threshold;

    if remaining_messages < page_size {
        assert!(needs_backfill, "Small deletions should trigger backfill");
        assert_eq!(_messages_needed, page_size - remaining_messages);
    }
}

#[test]
fn test_pagination_consistency_after_backfill() {
    let mut queue_state = create_test_queue_state(QueueType::Main);

    // Set up messages for multiple pages
    let page_size = config::get_config_or_panic().max_messages() as usize;
    let total_messages = page_size * 2 + 5; // 2+ pages

    let mut initial_messages = Vec::new();
    for i in 1..=total_messages {
        initial_messages.push(create_test_message(&format!("msg{}", i), i as i64));
    }

    setup_pagination_with_messages(&mut queue_state, initial_messages);

    // Verify initial pagination state
    let expected_pages = total_messages.div_ceil(page_size);
    assert_eq!(
        queue_state.message_pagination.total_pages_loaded,
        expected_pages
    );
    assert_eq!(queue_state.message_pagination.current_page, 0);

    // Delete several messages from first page
    let delete_count = 7;
    let mut to_remove = Vec::new();
    let mut remove_strings = Vec::new();
    for i in 1..=delete_count {
        let remove_string = format!("msg{}", i);
        remove_strings.push(remove_string);
    }
    for s in &remove_strings {
        to_remove.push(&s[..]);
    }

    let removed_count =
        simulate_bulk_remove_messages(&mut queue_state.message_pagination, &to_remove);
    assert_eq!(removed_count, delete_count);

    // Update pagination state
    update_pagination_after_removal(&mut queue_state.message_pagination);

    // Verify pagination was recalculated correctly
    let remaining_messages = total_messages - delete_count;
    assert_eq!(
        queue_state.message_pagination.all_loaded_messages.len(),
        remaining_messages
    );

    // Add backfill messages
    let backfill_messages = vec![
        create_test_message("backfill1", 1000),
        create_test_message("backfill2", 1001),
        create_test_message("backfill3", 1002),
    ];

    add_backfill_messages(&mut queue_state.message_pagination, backfill_messages);

    // Verify pagination consistency after backfill
    let final_message_count = remaining_messages + 3;
    assert_eq!(
        queue_state.message_pagination.all_loaded_messages.len(),
        final_message_count
    );

    let expected_final_pages = final_message_count.div_ceil(page_size);
    assert_eq!(
        queue_state.message_pagination.total_pages_loaded,
        expected_final_pages
    );

    // Verify pagination controls
    assert!(!queue_state.message_pagination.has_previous_page);
    assert_eq!(
        queue_state.message_pagination.has_next_page,
        expected_final_pages > 1
    );
}

#[test]
fn test_all_messages_deleted_edge_case() {
    let mut queue_state = create_test_queue_state(QueueType::Main);

    // Set up a small number of messages
    let initial_messages = vec![
        create_test_message("msg1", 1),
        create_test_message("msg2", 2),
        create_test_message("msg3", 3),
    ];

    setup_pagination_with_messages(&mut queue_state, initial_messages);

    // Delete all messages
    let removed_count = simulate_bulk_remove_messages(
        &mut queue_state.message_pagination,
        &["msg1", "msg2", "msg3"],
    );
    assert_eq!(removed_count, 3);
    assert_eq!(queue_state.message_pagination.all_loaded_messages.len(), 0);

    // Update pagination state
    update_pagination_after_removal(&mut queue_state.message_pagination);

    // Verify pagination state for empty queue
    assert_eq!(queue_state.message_pagination.total_pages_loaded, 0);
    assert_eq!(queue_state.message_pagination.current_page, 0);
    assert!(!queue_state.message_pagination.has_previous_page);
    assert!(!queue_state.message_pagination.has_next_page);

    // Check backfill logic for empty queue
    let (needs_backfill, messages_needed) = check_needs_backfill(&queue_state.message_pagination);

    // Empty queue should still allow backfill if messages are available
    let page_size = config::get_config_or_panic().max_messages() as usize;
    assert!(needs_backfill, "Empty queue should need backfill");
    assert_eq!(messages_needed, page_size);
}
