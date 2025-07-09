// Bulk selection UI logic for messages table
use crate::components::common::QueueType;
use crate::components::common::{MessageActivityMsg, Msg};
use crate::components::messages::PaginationInfo;

/// Create a message identifier from index - this will send a message to get the actual message data
pub fn create_toggle_message_selection(index: usize) -> Msg {
    Msg::MessageActivity(MessageActivityMsg::ToggleMessageSelectionByIndex(index))
}

/// Format queue display string for the title
pub fn format_queue_display(info: &PaginationInfo) -> String {
    let queue_name = info.queue_name.as_deref().unwrap_or("Unknown Queue");
    match info.queue_type {
        QueueType::Main => format!("📬 Messages ({queue_name}) [Main → d:DLQ]"),
        QueueType::DeadLetter => {
            format!("💀 Dead Letter Queue ({queue_name}) [DLQ → d:Main]")
        }
    }
}

/// Format bulk selection info for display
pub fn format_bulk_info(info: &PaginationInfo) -> String {
    if info.bulk_mode && info.selected_count > 0 {
        format!("• {} selected", info.selected_count)
    } else if info.bulk_mode {
        "• Bulk mode".to_string()
    } else {
        "".to_string()
    }
}

/// Format navigation hints for pagination
pub fn format_navigation_hints(info: &PaginationInfo) -> String {
    let mut hints = Vec::new();

    if info.has_previous_page {
        hints.push("◀[p]");
    }
    if info.has_next_page {
        hints.push("[n]▶");
    }

    if hints.is_empty() {
        "• End of pages".to_string()
    } else {
        format!("• {}", hints.join(" "))
    }
}

/// Format complete pagination status line
pub fn format_pagination_status(info: &PaginationInfo) -> String {
    let bulk_info = format_bulk_info(info);
    let navigation_hints = format_navigation_hints(info);

    if info.total_messages_loaded == 0 {
        format!("No messages available {bulk_info}")
    } else {
        let base_status = format!(
            "Page {}/{} • {} loaded • {} on page",
            info.current_page + 1, // Display as 1-based
            info.total_pages_loaded.max(1),
            info.total_messages_loaded,
            info.current_page_size
        );

        // Add queue statistics if available and enabled
        let queue_info = if crate::config::get_config_or_panic().queue_stats_display_enabled() {
            if let Some(total) = info.queue_total_messages {
                if let Some(age) = info.queue_stats_age_seconds {
                    let config = crate::config::get_config_or_panic();
                    let age_threshold = config.ui().queue_stats_age_threshold_seconds() as i64;
                    if age < age_threshold {
                        format!(" • {total} in queue")
                    } else {
                        format!(" • ~{} in queue ({}m ago)", total, age / age_threshold)
                    }
                } else {
                    format!(" • {total} in queue")
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        format!("{base_status}{queue_info} {navigation_hints} {bulk_info}")
    }
}
