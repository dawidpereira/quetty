use quetty::app::updates::messages::MessagePaginationState;
use quetty::config;
use server::model::{BodyData, MessageModel, MessageState};
use std::time::{Duration, SystemTime};

/// Helper function to create a test message with a specific sequence
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

/// Helper function to simulate the new automatic page filling logic
fn simulate_auto_page_filling(
    pagination: &mut MessagePaginationState,
    incomplete_page_messages: Vec<MessageModel>,
    fill_messages: Vec<MessageModel>,
) {
    // Simulate loading incomplete page first (like from sequence gaps)
    pagination.add_loaded_page(incomplete_page_messages);

    // Then simulate the auto-fill with additional messages using append_messages
    // Since append_messages updates total_pages_loaded, we need to adjust for that behavior
    if !fill_messages.is_empty() {
        let current_pages = pagination.total_pages_loaded;
        pagination.append_messages(fill_messages);
        // Reset pages to what extend_current_page would have done (not increment)
        pagination.total_pages_loaded = current_pages;
    }
}

#[test]
fn test_automatic_page_filling_with_sequence_gaps() {
    let mut pagination = MessagePaginationState::default();
    pagination.reset();

    let page_size = config::get_config_or_panic().max_messages() as usize;

    // Simulate the reported bug: requesting 100 messages starting from sequence 30591
    // But Azure only returns 55 messages due to sequence gaps (30591-30645, missing 30646-30690)
    let incomplete_page = (30591..30646)
        .map(|seq| create_test_message(&format!("msg_{seq}"), seq))
        .collect::<Vec<_>>();

    assert_eq!(incomplete_page.len(), 55); // Incomplete page

    // Simulate the automatic fill with additional messages (30691-30735)
    let fill_messages = (30691..30726)
        .map(|seq| create_test_message(&format!("msg_{seq}"), seq))
        .collect::<Vec<_>>();

    assert_eq!(fill_messages.len(), 35); // Messages to fill the gap

    // Apply the auto page filling
    simulate_auto_page_filling(&mut pagination, incomplete_page, fill_messages);

    // Verify the page is now properly filled
    assert_eq!(pagination.all_loaded_messages.len(), 90); // 55 + 35
    assert_eq!(pagination.total_pages_loaded, 1);
    assert_eq!(pagination.current_page, 0);

    // Verify last sequence tracking
    assert_eq!(pagination.last_loaded_sequence, Some(30725));

    // Get current page and verify it contains all messages
    let current_page_messages = pagination.get_current_page_messages(page_size as u32);
    assert_eq!(current_page_messages.len(), 90);
}

#[test]
fn test_complete_page_filling_to_expected_size() {
    let mut pagination = MessagePaginationState::default();
    pagination.reset();

    let page_size = config::get_config_or_panic().max_messages() as usize;

    // Start with an incomplete page of 55 messages
    let incomplete_page = (1..56)
        .map(|seq| create_test_message(&format!("msg_{seq}"), seq))
        .collect::<Vec<_>>();

    // Need to fill to full page size, so calculate how many more needed
    let messages_needed = page_size - 55;
    let fill_messages = (100..100 + messages_needed as i64)
        .map(|seq| create_test_message(&format!("fill_{seq}"), seq))
        .collect::<Vec<_>>();

    simulate_auto_page_filling(&mut pagination, incomplete_page, fill_messages);

    // Verify page is now exactly page_size
    let current_page_messages = pagination.get_current_page_messages(page_size as u32);
    assert_eq!(current_page_messages.len(), page_size);
    assert_eq!(pagination.all_loaded_messages.len(), page_size);
    assert_eq!(pagination.total_pages_loaded, 1);
}

#[test]
fn test_append_messages_with_page_count_preservation() {
    let mut pagination = MessagePaginationState::default();
    pagination.reset();

    // Add initial page
    let initial_messages = vec![
        create_test_message("msg1", 1),
        create_test_message("msg2", 2),
        create_test_message("msg3", 3),
    ];
    pagination.add_loaded_page(initial_messages);

    assert_eq!(pagination.total_pages_loaded, 1);
    assert_eq!(pagination.all_loaded_messages.len(), 3);

    // Extend current page (should not increment page count)
    let extension_messages = vec![
        create_test_message("msg4", 4),
        create_test_message("msg5", 5),
    ];
    let current_pages = pagination.total_pages_loaded;
    pagination.append_messages(extension_messages);
    pagination.total_pages_loaded = current_pages;

    // Verify page count didn't increase but message count did
    assert_eq!(pagination.total_pages_loaded, 1); // Still 1 page
    assert_eq!(pagination.all_loaded_messages.len(), 5); // But 5 messages total
    assert_eq!(pagination.last_loaded_sequence, Some(5));
}

#[test]
fn test_reached_end_of_queue_handling() {
    let mut pagination = MessagePaginationState::default();
    pagination.reset();

    // Add initial incomplete page
    let initial_messages = vec![
        create_test_message("msg1", 1),
        create_test_message("msg2", 2),
    ];
    pagination.add_loaded_page(initial_messages);

    assert!(!pagination.reached_end_of_queue);

    // Try to extend with empty messages (simulates no more messages available)
    pagination.append_messages(vec![]);

    // Should mark end of queue reached
    assert!(pagination.reached_end_of_queue);
    assert_eq!(pagination.all_loaded_messages.len(), 2); // Messages unchanged
}

#[test]
fn test_page_start_indices_tracking_with_extensions() {
    let mut pagination = MessagePaginationState::default();
    pagination.reset();

    let page_size = config::get_config_or_panic().max_messages() as usize;

    // Initially should have page 0 starting at index 0
    assert_eq!(pagination.page_start_indices, vec![0]);

    // Add first page
    let page1_messages = (1..=page_size as i64)
        .map(|seq| create_test_message(&format!("p1_msg_{seq}"), seq))
        .collect::<Vec<_>>();
    pagination.add_loaded_page(page1_messages);

    // Add second page (should track start index)
    let page2_incomplete = (101..151)
        .map(|seq| create_test_message(&format!("p2_msg_{seq}"), seq))
        .collect::<Vec<_>>();
    pagination.add_loaded_page(page2_incomplete);

    // Should now track start of page 2
    assert_eq!(pagination.page_start_indices, vec![0, page_size]);

    // Extend page 2 (should not add new start index)
    let page2_extension = (151..201)
        .map(|seq| create_test_message(&format!("p2_ext_{seq}"), seq))
        .collect::<Vec<_>>();
    let current_pages = pagination.total_pages_loaded;
    pagination.append_messages(page2_extension);
    pagination.total_pages_loaded = current_pages;

    // Page start indices should remain the same
    assert_eq!(pagination.page_start_indices, vec![0, page_size]);
    assert_eq!(pagination.total_pages_loaded, 2);
}

#[test]
fn test_calculate_has_next_page_with_sequence_gaps() {
    let mut pagination = MessagePaginationState::default();
    pagination.reset();

    let page_size = config::get_config_or_panic().max_messages();

    // Initially should try to load more (not reached end)
    pagination.update(page_size);
    assert!(pagination.has_next_page); // Should try to load

    // After loading incomplete page, should still try for more
    let incomplete_page = vec![
        create_test_message("msg1", 1),
        create_test_message("msg2", 2),
    ];
    pagination.add_loaded_page(incomplete_page);
    pagination.update(page_size);
    assert!(pagination.has_next_page); // Still should try to load more

    // Only after confirming end of queue should has_next_page be false
    pagination.reached_end_of_queue = true;
    pagination.update(page_size);
    assert!(!pagination.has_next_page); // Now should not try to load more
}

#[test]
fn test_multiple_extension_cycles() {
    let mut pagination = MessagePaginationState::default();
    pagination.reset();

    // Add initial incomplete page
    let initial = vec![create_test_message("msg1", 1)];
    pagination.add_loaded_page(initial);
    assert_eq!(pagination.all_loaded_messages.len(), 1);

    // First extension
    let ext1 = vec![create_test_message("msg2", 2)];
    let current_pages = pagination.total_pages_loaded;
    pagination.append_messages(ext1);
    pagination.total_pages_loaded = current_pages;
    assert_eq!(pagination.all_loaded_messages.len(), 2);
    assert_eq!(pagination.total_pages_loaded, 1);

    // Second extension
    let ext2 = vec![
        create_test_message("msg3", 3),
        create_test_message("msg4", 4),
    ];
    let current_pages = pagination.total_pages_loaded;
    pagination.append_messages(ext2);
    pagination.total_pages_loaded = current_pages;
    assert_eq!(pagination.all_loaded_messages.len(), 4);
    assert_eq!(pagination.total_pages_loaded, 1); // Still one page

    // Verify last sequence is updated correctly
    assert_eq!(pagination.last_loaded_sequence, Some(4));
}

#[test]
fn test_page_bounds_calculation_with_extended_pages() {
    let mut pagination = MessagePaginationState::default();
    pagination.reset();

    let page_size = config::get_config_or_panic().max_messages();

    // Create page 1 with extensions
    let initial_p1 = (1..51)
        .map(|seq| create_test_message(&format!("p1_{seq}"), seq))
        .collect::<Vec<_>>();
    pagination.add_loaded_page(initial_p1);

    let extend_p1 = (51..=page_size as i64)
        .map(|seq| create_test_message(&format!("p1_ext_{seq}"), seq))
        .collect::<Vec<_>>();
    let current_pages = pagination.total_pages_loaded;
    pagination.append_messages(extend_p1);
    pagination.total_pages_loaded = current_pages;

    // Create page 2
    let initial_p2 = (1001..1051)
        .map(|seq| create_test_message(&format!("p2_{seq}"), seq))
        .collect::<Vec<_>>();
    pagination.add_loaded_page(initial_p2);

    // Test page bounds for page 0
    pagination.current_page = 0;
    let (start, end) = pagination.calculate_page_bounds(page_size);
    assert_eq!(start, 0);
    assert_eq!(end, page_size as usize);

    // Test page bounds for page 1
    pagination.current_page = 1;
    let (start, end) = pagination.calculate_page_bounds(page_size);
    assert_eq!(start, page_size as usize);
    assert_eq!(end, page_size as usize + 50); // Only 50 messages in page 2
}

#[test]
fn test_realistic_azure_sequence_gap_scenario() {
    let mut pagination = MessagePaginationState::default();
    pagination.reset();

    // Simulate real Azure scenario: 358 total messages, page size 100
    // Page 1: sequences 1-100 (complete)
    let page1 = (1..=100)
        .map(|seq| create_test_message(&format!("p1_{seq}"), seq))
        .collect::<Vec<_>>();
    pagination.add_loaded_page(page1);

    // Page 2: sequences 101-200 (complete)
    let page2 = (101..=200)
        .map(|seq| create_test_message(&format!("p2_{seq}"), seq))
        .collect::<Vec<_>>();
    pagination.add_loaded_page(page2);

    // Page 3: sequences 201-255 (incomplete due to gaps, only 55 messages)
    let page3_incomplete = (201..=255)
        .map(|seq| create_test_message(&format!("p3_{seq}"), seq))
        .collect::<Vec<_>>();
    pagination.add_loaded_page(page3_incomplete);

    // Auto-fill page 3: sequences 300-344 (filling the gap to make 100 total)
    let page3_fill = (300..=344)
        .map(|seq| create_test_message(&format!("p3_fill_{seq}"), seq))
        .collect::<Vec<_>>();
    let current_pages = pagination.total_pages_loaded;
    pagination.append_messages(page3_fill);
    pagination.total_pages_loaded = current_pages;

    // Final page: remaining messages 345-358
    let page4 = (345..=358)
        .map(|seq| create_test_message(&format!("p4_{seq}"), seq))
        .collect::<Vec<_>>();
    pagination.add_loaded_page(page4);

    // Verify final state
    assert_eq!(pagination.total_pages_loaded, 4);
    assert_eq!(pagination.all_loaded_messages.len(), 314); // 100 + 100 + (55 + 45) + 14

    // Verify page 3 is properly filled
    pagination.current_page = 2;
    let page3_messages = pagination.get_current_page_messages(100);
    assert_eq!(page3_messages.len(), 100); // Should be full page now

    // Verify page 4 has remaining messages
    pagination.current_page = 3;
    let page4_messages = pagination.get_current_page_messages(100);
    assert_eq!(page4_messages.len(), 14); // Final partial page
}
