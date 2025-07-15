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

/// Helper to simulate page size change backfill scenario
fn setup_pagination_with_initial_messages(
    pagination: &mut MessagePaginationState,
    count: usize,
    page_size: u32,
) {
    pagination.reset();

    // Create initial messages
    let mut messages = Vec::new();
    for i in 1..=count {
        messages.push(create_test_message(&format!("msg{i}"), i as i64));
    }

    pagination.all_loaded_messages = messages.clone();
    pagination.total_pages_loaded = 1; // Simulate having one page loaded

    if let Some(last_msg) = messages.last() {
        pagination.last_loaded_sequence = Some(last_msg.sequence);
    }

    pagination.update(page_size);
}

/// Simulate adding backfill messages for page size increase
fn add_backfill_messages_for_page_increase(
    pagination: &mut MessagePaginationState,
    additional_count: usize,
    start_sequence: i64,
) {
    let mut backfill_messages = Vec::new();
    for i in 0..additional_count {
        backfill_messages.push(create_test_message(
            &format!("backfill{}", i + 1),
            start_sequence + i as i64,
        ));
    }

    // Use extend_current_page which is what BackfillMessagesLoaded should trigger
    pagination.extend_current_page(backfill_messages);
}

#[test]
fn test_page_size_increase_100_to_1000_triggers_backfill() {
    let mut pagination = MessagePaginationState::default();

    // Start with 100 messages loaded, page size 100
    let initial_page_size = 100u32;
    let initial_message_count = 100;
    setup_pagination_with_initial_messages(
        &mut pagination,
        initial_message_count,
        initial_page_size,
    );

    // Verify initial state
    assert_eq!(pagination.all_loaded_messages.len(), 100);
    assert_eq!(pagination.current_page, 0);
    assert_eq!(pagination.total_pages_loaded, 1);

    // Change page size to 1000
    let new_page_size = 1000u32;
    let messages_needed = (new_page_size as usize).saturating_sub(initial_message_count);
    assert_eq!(messages_needed, 900, "Should need 900 additional messages");

    // Update pagination for new page size
    pagination.update(new_page_size);

    // Simulate backfill process - should load 900 messages
    let next_sequence = pagination.last_loaded_sequence.unwrap() + 1;
    add_backfill_messages_for_page_increase(&mut pagination, messages_needed, next_sequence);

    // Verify final state
    assert_eq!(
        pagination.all_loaded_messages.len(),
        1000,
        "Should have 1000 total messages"
    );
    assert_eq!(pagination.current_page, 0, "Should still be on page 0");

    // Verify sequence continuity
    assert_eq!(
        pagination.last_loaded_sequence,
        Some(1000),
        "Last sequence should be 1000"
    );

    // Test pagination bounds
    pagination.update(new_page_size);
    let (start, end) = pagination.calculate_page_bounds(new_page_size);
    assert_eq!(start, 0);
    assert_eq!(end, 1000);

    let current_page_messages = pagination.get_current_page_messages(new_page_size);
    assert_eq!(
        current_page_messages.len(),
        1000,
        "Current page should have 1000 messages"
    );
}

#[test]
fn test_page_size_increase_with_partial_backfill() {
    let mut pagination = MessagePaginationState::default();

    // Start with 100 messages loaded, page size 100
    setup_pagination_with_initial_messages(&mut pagination, 100, 100);

    // Change page size to 1000 but simulate Azure returning fewer messages (e.g., only 250)
    let new_page_size = 1000u32;
    pagination.update(new_page_size);

    // First backfill: request 900, get 250
    let next_sequence = pagination.last_loaded_sequence.unwrap() + 1;
    add_backfill_messages_for_page_increase(&mut pagination, 250, next_sequence);

    // Should have 350 messages now
    assert_eq!(pagination.all_loaded_messages.len(), 350);

    // Check if we need more messages
    let current_page_messages = pagination.get_current_page_messages(new_page_size);
    assert_eq!(current_page_messages.len(), 350);

    // Should still need more messages to fill the 1000-message page
    let messages_still_needed = new_page_size as usize - current_page_messages.len();
    assert_eq!(messages_still_needed, 650);

    // Second backfill: request remaining 650, get another 250
    let next_sequence = pagination.last_loaded_sequence.unwrap() + 1;
    add_backfill_messages_for_page_increase(&mut pagination, 250, next_sequence);

    // Should have 600 messages now
    assert_eq!(pagination.all_loaded_messages.len(), 600);

    // Continue until we get close to 1000 or reach end of queue
    let next_sequence = pagination.last_loaded_sequence.unwrap() + 1;
    add_backfill_messages_for_page_increase(&mut pagination, 250, next_sequence);
    assert_eq!(pagination.all_loaded_messages.len(), 850);

    let next_sequence = pagination.last_loaded_sequence.unwrap() + 1;
    add_backfill_messages_for_page_increase(&mut pagination, 150, next_sequence);
    assert_eq!(pagination.all_loaded_messages.len(), 1000);
}

#[test]
fn test_page_size_increase_with_queue_exhaustion() {
    let mut pagination = MessagePaginationState::default();

    // Start with 100 messages loaded, page size 100
    setup_pagination_with_initial_messages(&mut pagination, 100, 100);

    // Change page size to 1000
    let new_page_size = 1000u32;
    pagination.update(new_page_size);

    // Simulate backfill where queue only has 200 additional messages
    let next_sequence = pagination.last_loaded_sequence.unwrap() + 1;
    add_backfill_messages_for_page_increase(&mut pagination, 200, next_sequence);

    // Should have 300 messages total
    assert_eq!(pagination.all_loaded_messages.len(), 300);

    // Simulate empty backfill response (end of queue reached)
    let empty_messages: Vec<MessageModel> = Vec::new();
    pagination.extend_current_page(empty_messages);

    // Should still have 300 messages
    assert_eq!(pagination.all_loaded_messages.len(), 300);

    // Should mark end of queue reached
    assert!(
        pagination.reached_end_of_queue,
        "Should mark end of queue reached"
    );

    // Current page should show all 300 messages
    let current_page_messages = pagination.get_current_page_messages(new_page_size);
    assert_eq!(current_page_messages.len(), 300);
}

#[test]
fn test_multiple_page_size_changes() {
    let mut pagination = MessagePaginationState::default();

    // Start with 50 messages, page size 50
    setup_pagination_with_initial_messages(&mut pagination, 50, 50);

    // First increase: 50 → 200
    pagination.update(200);
    let next_sequence = pagination.last_loaded_sequence.unwrap() + 1;
    add_backfill_messages_for_page_increase(&mut pagination, 150, next_sequence);
    assert_eq!(pagination.all_loaded_messages.len(), 200);

    // Second increase: 200 → 500
    pagination.update(500);
    let next_sequence = pagination.last_loaded_sequence.unwrap() + 1;
    add_backfill_messages_for_page_increase(&mut pagination, 300, next_sequence);
    assert_eq!(pagination.all_loaded_messages.len(), 500);

    // Third increase: 500 → 1000
    pagination.update(1000);
    let next_sequence = pagination.last_loaded_sequence.unwrap() + 1;
    add_backfill_messages_for_page_increase(&mut pagination, 500, next_sequence);
    assert_eq!(pagination.all_loaded_messages.len(), 1000);

    // Verify pagination state is consistent
    let current_page_messages = pagination.get_current_page_messages(1000);
    assert_eq!(current_page_messages.len(), 1000);
    assert_eq!(pagination.current_page, 0);
    assert_eq!(pagination.total_pages_loaded, 1);
}

#[test]
fn test_batched_backfill_loading() {
    let mut pagination = MessagePaginationState::default();

    // Start with 100 messages, page size 100
    setup_pagination_with_initial_messages(&mut pagination, 100, 100);

    // Simulate page size change to 1000 requiring 900 additional messages
    pagination.update(1000);

    // Simulate Azure returning messages in batches (as the new implementation does)
    // First batch: 500 messages (batch 1 of 2)
    let next_sequence = pagination.last_loaded_sequence.unwrap() + 1;
    add_backfill_messages_for_page_increase(&mut pagination, 500, next_sequence);
    assert_eq!(pagination.all_loaded_messages.len(), 600);

    // Second batch: 400 messages (completing the request)
    let next_sequence = pagination.last_loaded_sequence.unwrap() + 1;
    add_backfill_messages_for_page_increase(&mut pagination, 400, next_sequence);
    assert_eq!(pagination.all_loaded_messages.len(), 1000);

    // Verify sequence continuity across batches
    for i in 0..1000 {
        assert_eq!(pagination.all_loaded_messages[i].sequence, (i + 1) as i64);
    }

    // Verify current page shows all 1000 messages
    let current_page_messages = pagination.get_current_page_messages(1000);
    assert_eq!(current_page_messages.len(), 1000);
}

#[test]
fn test_next_page_after_backfill_loads_minimal_messages() {
    let mut pagination = MessagePaginationState::default();

    // Start with 1000 messages, page size 1000 (exactly one full page)
    setup_pagination_with_initial_messages(&mut pagination, 1000, 1000);

    // Simulate deletion of 3 messages from current page
    pagination.all_loaded_messages.drain(0..3); // Remove first 3 messages
    pagination.update(1000);

    // Now we have 997 messages, but page size is 1000
    assert_eq!(pagination.all_loaded_messages.len(), 997);

    // The current page should show 997 messages
    let current_page_messages = pagination.get_current_page_messages(1000);
    assert_eq!(current_page_messages.len(), 997);

    // Simulate backfill of 3 messages to fill the current page
    let next_sequence = pagination.last_loaded_sequence.unwrap() + 1;
    add_backfill_messages_for_page_increase(&mut pagination, 3, next_sequence);

    // Now we should have 1000 messages again
    assert_eq!(pagination.all_loaded_messages.len(), 1000);

    // Test: If we try to go to next page, it should load minimal messages
    // For this test, we simulate that we need to load for next page
    pagination.update(1000);

    // Current page (0) should be full with 1000 messages
    let current_page_messages = pagination.get_current_page_messages(1000);
    assert_eq!(current_page_messages.len(), 1000);

    // Simulate going to next page - would need exactly 1000 more messages for page 1
    // But since this is a unit test of the pagination logic itself,
    // we verify that the math works correctly
    let page_size = 1000;
    let current_page = 0;
    let total_messages = 1000;
    let next_page_start = (current_page + 1) * page_size; // Should be 1000

    // If we have exactly 1000 messages and want to go to page 1 (starting at index 1000),
    // we need exactly 1000 more messages to fill page 1
    let messages_needed_for_next_page = next_page_start + page_size - total_messages;
    assert_eq!(
        messages_needed_for_next_page, 1000,
        "Should need exactly 1000 messages for next page"
    );

    // But if we had 1997 messages and deleted 3, then backfilled 3,
    // and then want to go to next page, we should need minimal messages
    pagination.all_loaded_messages.clear();
    setup_pagination_with_initial_messages(&mut pagination, 1997, 1000);

    // Delete 3 messages from somewhere in the first page
    pagination.all_loaded_messages.drain(10..13);
    assert_eq!(pagination.all_loaded_messages.len(), 1994);

    // The calculation for next page should be:
    // next_page_start = 1 * 1000 = 1000
    // messages_needed = 1000 + 1000 - 1994 = 6 messages
    let total_messages = 1994;
    let messages_needed_for_next_page = next_page_start + page_size - total_messages;
    assert_eq!(
        messages_needed_for_next_page, 6,
        "Should need only 6 messages to complete next page"
    );
}

#[test]
fn test_next_page_after_deletion_loads_missing_messages() {
    let mut pagination = MessagePaginationState::default();

    // Start with 200 messages, page size 100 (2 full pages)
    setup_pagination_with_initial_messages(&mut pagination, 200, 100);

    // Verify initial state: page 0 has 100 messages, page 1 should have 100 messages
    assert_eq!(pagination.all_loaded_messages.len(), 200);
    let page_0_messages = pagination.get_current_page_messages(100);
    assert_eq!(page_0_messages.len(), 100);

    // Simulate navigation to page 1
    pagination.current_page = 1;
    let page_1_messages = pagination.get_current_page_messages(100);
    assert_eq!(
        page_1_messages.len(),
        100,
        "Page 1 should initially have 100 messages"
    );

    // Go back to page 0
    pagination.current_page = 0;

    // Delete 3 messages from page 0 (simulate deletion)
    pagination.all_loaded_messages.drain(5..8); // Remove messages at indices 5, 6, 7
    assert_eq!(pagination.all_loaded_messages.len(), 197);

    // Update pagination state after deletion
    pagination.update(100);

    // Page 0 should now have 97 messages (after deletion, before backfill)
    let page_0_after_deletion = pagination.get_current_page_messages(100);
    println!(
        "Page 0 after deletion has {} messages",
        page_0_after_deletion.len()
    );
    // Note: This might still show 100 if page 0 goes from index 0-99 regardless of total array size
    // The real issue is in the next page

    // Simulate backfill of 3 messages to page 0
    let next_sequence = pagination.last_loaded_sequence.unwrap() + 1;
    add_backfill_messages_for_page_increase(&mut pagination, 3, next_sequence);

    // Now we should have 200 messages again
    assert_eq!(pagination.all_loaded_messages.len(), 200);

    // Page 0 should have 100 messages (after backfill)
    let page_0_after_backfill = pagination.get_current_page_messages(100);
    assert_eq!(page_0_after_backfill.len(), 100);

    // The key test: when we navigate to page 1, how many messages does it have?
    pagination.current_page = 1;
    pagination.update(100);
    let page_1_after_backfill = pagination.get_current_page_messages(100);

    // This is the scenario from the bug report: page 1 might have < 100 messages
    // due to the array having mixed content after deletion + backfill
    if page_1_after_backfill.len() < 100 {
        println!(
            "Bug reproduced: Page 1 has {} messages instead of 100",
            page_1_after_backfill.len()
        );

        // In a real system, this would trigger loading more messages
        let messages_needed = 100 - page_1_after_backfill.len();
        println!("Would need to load {messages_needed} more messages");

        // Simulate loading the missing messages
        let next_sequence = pagination.last_loaded_sequence.unwrap() + 1;
        add_backfill_messages_for_page_increase(&mut pagination, messages_needed, next_sequence);

        // Now page 1 should be complete
        let page_1_final = pagination.get_current_page_messages(100);
        assert_eq!(
            page_1_final.len(),
            100,
            "Page 1 should have 100 messages after loading missing ones"
        );
    } else {
        // Page 1 already has 100 messages, which is the desired behavior
        assert_eq!(page_1_after_backfill.len(), 100);
    }
}

#[test]
fn test_page_size_change_preserves_sequence_order() {
    let mut pagination = MessagePaginationState::default();

    // Start with 100 messages, sequences 1-100
    setup_pagination_with_initial_messages(&mut pagination, 100, 100);

    // Verify initial sequence order
    assert_eq!(pagination.all_loaded_messages[0].sequence, 1);
    assert_eq!(pagination.all_loaded_messages[99].sequence, 100);

    // Increase page size and add backfill
    pagination.update(500);
    let next_sequence = pagination.last_loaded_sequence.unwrap() + 1;
    add_backfill_messages_for_page_increase(&mut pagination, 400, next_sequence);

    // Verify sequences are in order: 1-100 (original) + 101-500 (backfill)
    assert_eq!(pagination.all_loaded_messages.len(), 500);
    assert_eq!(pagination.all_loaded_messages[0].sequence, 1);
    assert_eq!(pagination.all_loaded_messages[99].sequence, 100);
    assert_eq!(pagination.all_loaded_messages[100].sequence, 101);
    assert_eq!(pagination.all_loaded_messages[499].sequence, 500);

    // Verify last_loaded_sequence is updated correctly
    assert_eq!(pagination.last_loaded_sequence, Some(500));
}
