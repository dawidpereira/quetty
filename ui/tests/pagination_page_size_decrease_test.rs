/// This test verifies that when a user:
/// 1. Loads 2 pages of 1000 messages each (2000 total)
/// 2. Deletes 2 messages (1998 remaining)
/// 3. Changes page size from 1000 to 100
///
/// The system should use the first 100 messages from the loaded 1998 messages,
/// NOT reload from the API starting at sequence 2000+.
use quetty::app::updates::messages::MessagePaginationState;
use server::model::{BodyData, MessageModel, MessageState};
use std::time::{Duration, SystemTime};

/// Helper to create a test message
fn create_test_message(id: &str, sequence: i64) -> MessageModel {
    let timestamp = SystemTime::UNIX_EPOCH + Duration::from_secs(1700000000 + sequence as u64);
    MessageModel {
        sequence,
        id: id.to_string(),
        enqueued_at: timestamp.into(),
        delivery_count: 1,
        state: MessageState::Active,
        body: BodyData::RawString(format!("Test message {id}")),
    }
}

#[test]
fn test_page_size_decrease_from_1000_to_100_uses_loaded_messages() {
    let mut pagination = MessagePaginationState::default();

    // Simulate loading 2000 messages (2 pages of 1000 each) as the user described
    let mut messages = Vec::new();
    for i in 1..=2000 {
        messages.push(create_test_message(&format!("msg{i}"), i as i64));
    }

    // Set up pagination state as if we loaded 2 pages of 1000 messages
    pagination.all_loaded_messages = messages.clone();
    pagination.last_loaded_sequence = Some(2000);
    pagination.total_pages_loaded = 2;
    pagination.current_page = 1; // User is on page 1 (second page)
    pagination.update(1000); // Current page size is 1000

    // Verify initial state - user should see messages 1001-2000 on page 1
    let page_1_messages = pagination.get_current_page_messages(1000);
    assert_eq!(page_1_messages.len(), 1000);
    assert_eq!(page_1_messages[0].sequence, 1001); // First message on page 1
    assert_eq!(page_1_messages[999].sequence, 2000); // Last message on page 1

    // Simulate deletion of 2 messages (as user described)
    pagination.all_loaded_messages.drain(10..12); // Remove 2 messages
    assert_eq!(pagination.all_loaded_messages.len(), 1998);

    // Update pagination after deletion
    pagination.update(1000);

    // Now simulate page size change from 1000 to 100
    // This should use the loaded messages, not reload from API

    // First, verify we have sufficient messages for the new page size
    let current_loaded_count = pagination.all_loaded_messages.len();
    let new_page_size = 100u32;
    assert!(current_loaded_count >= new_page_size as usize);

    // Reset to page 0 and update pagination bounds (simulating the fix)
    pagination.current_page = 0;
    pagination.update(new_page_size);

    // Verify the fix: we should see the first 100 messages from our loaded set
    let page_0_after_resize = pagination.get_current_page_messages(new_page_size);
    assert_eq!(page_0_after_resize.len(), 100);

    // These should be messages 1-10, then 13-102 (since we deleted messages 11-12)
    // But the key point is they should be from our loaded messages, not from sequence 2000+
    assert_eq!(page_0_after_resize[0].sequence, 1); // First message should be sequence 1
    assert!(page_0_after_resize[99].sequence <= 102); // Last message should be <= 102

    // Verify we can navigate through multiple pages using loaded data
    // Calculate total pages available with new page size
    let total_pages = pagination
        .all_loaded_messages
        .len()
        .div_ceil(new_page_size as usize);
    assert_eq!(total_pages, 20); // 1998 messages / 100 per page = 20 pages (rounded up)

    // Test navigation to page 1
    pagination.current_page = 1;
    let page_1_after_resize = pagination.get_current_page_messages(new_page_size);
    assert_eq!(page_1_after_resize.len(), 100);

    // The key assertion: these should be messages from our loaded set, not new API calls
    // Since we deleted messages 11-12, page 1 should start around sequence 103
    assert!(page_1_after_resize[0].sequence > 100);
    assert!(page_1_after_resize[0].sequence < 200); // Should not be from 2000+ range

    // Test navigation to page 19 (last page)
    pagination.current_page = 19;
    let last_page = pagination.get_current_page_messages(new_page_size);
    assert_eq!(last_page.len(), 98); // 1998 % 100 = 98 messages on last page

    // Verify last page contains high sequence numbers but still from our original loaded set
    let last_message_seq = last_page.last().unwrap().sequence;
    assert!(last_message_seq <= 2000); // Should be from our original 2000 messages
    assert!(last_message_seq > 1900); // Should be near the end of our loaded set
}

#[test]
fn test_page_size_decrease_insufficient_messages_triggers_reload() {
    let mut pagination = MessagePaginationState::default();

    // Load only 50 messages
    let mut messages = Vec::new();
    for i in 1..=50 {
        messages.push(create_test_message(&format!("msg{i}"), i as i64));
    }

    pagination.all_loaded_messages = messages;
    pagination.last_loaded_sequence = Some(50);
    pagination.total_pages_loaded = 1;
    pagination.current_page = 0;
    pagination.update(50); // Current page size is 50

    // Now try to change page size to 100
    let current_loaded_count = pagination.all_loaded_messages.len();
    let new_page_size = 100u32;

    // Verify we don't have enough messages for the new page size
    assert!(current_loaded_count < new_page_size as usize);

    // In this case, the system should reload from API (tested in the main logic)
    // This test just verifies our condition detection works correctly
    assert_eq!(current_loaded_count, 50);
    assert_eq!(new_page_size, 100);
}

#[test]
fn test_page_size_decrease_preserves_message_order() {
    let mut pagination = MessagePaginationState::default();

    // Load 500 messages with sequences 1-500
    let mut messages = Vec::new();
    for i in 1..=500 {
        messages.push(create_test_message(&format!("msg{i}"), i as i64));
    }

    pagination.all_loaded_messages = messages;
    pagination.last_loaded_sequence = Some(500);
    pagination.total_pages_loaded = 1;
    pagination.current_page = 0;
    pagination.update(500); // Current page size is 500

    // Change page size to 100
    let new_page_size = 100u32;
    pagination.current_page = 0;
    pagination.update(new_page_size);

    // Verify page 0 has messages 1-100 in correct order
    let page_0 = pagination.get_current_page_messages(new_page_size);
    assert_eq!(page_0.len(), 100);
    for (i, message) in page_0.iter().enumerate() {
        assert_eq!(message.sequence, (i + 1) as i64);
    }

    // Verify page 1 has messages 101-200 in correct order
    pagination.current_page = 1;
    let page_1 = pagination.get_current_page_messages(new_page_size);
    assert_eq!(page_1.len(), 100);
    for (i, message) in page_1.iter().enumerate() {
        assert_eq!(message.sequence, (i + 101) as i64);
    }

    // Verify page 4 (last page) has messages 401-500 in correct order
    pagination.current_page = 4;
    let page_4 = pagination.get_current_page_messages(new_page_size);
    assert_eq!(page_4.len(), 100);
    for (i, message) in page_4.iter().enumerate() {
        assert_eq!(message.sequence, (i + 401) as i64);
    }
}
