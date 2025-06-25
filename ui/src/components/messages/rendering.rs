use crate::components::messages::PaginationInfo;
use crate::theme::ThemeManager;
use server::bulk_operations::MessageIdentifier;
use server::model::{MessageModel, MessageState};
use tuirealm::props::{TableBuilder, TextSpan};
use tuirealm::ratatui::style::Color;

/// Calculate responsive column widths based on available screen width
pub fn calculate_responsive_layout(
    available_width: u16,
    bulk_mode: bool,
) -> (Vec<String>, Vec<u16>, bool) {
    let headers = if bulk_mode {
        vec![
            "".to_string(), // Checkbox column
            "Sequence".to_string(),
            "Message ID".to_string(),
            "Enqueued At".to_string(),
            "State".to_string(),
            "Delivery Count".to_string(),
        ]
    } else {
        vec![
            "Sequence".to_string(),
            "Message ID".to_string(),
            "Enqueued At".to_string(),
            "State".to_string(),
            "Delivery Count".to_string(),
        ]
    };

    // Account for borders (4 chars), column spacing, and some padding
    let usable_width = available_width.saturating_sub(10);
    let num_cols = if bulk_mode { 6 } else { 5 };
    let spacing = (num_cols - 1) * 2; // 2 chars between columns
    let content_width = usable_width.saturating_sub(spacing);

    // Define proportional widths that scale with screen size
    let widths = if bulk_mode {
        let checkbox = 3;
        let remaining = content_width.saturating_sub(checkbox);

        // Proportional distribution: seq(8%), msg_id(35%), enqueued(40%), state(10%), delivery(7%)
        let seq_width = (remaining * 8 / 100).max(6);
        let msg_id_width = (remaining * 35 / 100).max(15);
        let enqueued_width = (remaining * 40 / 100).max(20);
        let state_width = (remaining * 10 / 100).max(8);
        let delivery_width = remaining
            .saturating_sub(seq_width + msg_id_width + enqueued_width + state_width)
            .max(6);

        vec![
            checkbox,
            seq_width,
            msg_id_width,
            enqueued_width,
            state_width,
            delivery_width,
        ]
    } else {
        // Proportional distribution: seq(10%), msg_id(35%), enqueued(40%), state(10%), delivery(5%)
        let seq_width = (content_width * 10 / 100).max(8);
        let msg_id_width = (content_width * 35 / 100).max(15);
        let enqueued_width = (content_width * 40 / 100).max(20);
        let state_width = (content_width * 10 / 100).max(8);
        let delivery_width = content_width
            .saturating_sub(seq_width + msg_id_width + enqueued_width + state_width)
            .max(6);

        vec![
            seq_width,
            msg_id_width,
            enqueued_width,
            state_width,
            delivery_width,
        ]
    };

    // Always use wide layout behavior (right-aligned delivery count)
    let use_narrow_layout = false;

    (headers, widths, use_narrow_layout)
}

/// Format delivery count with right alignment (always)
pub fn format_delivery_count_responsive(
    count: usize,
    width: usize,
    _narrow_layout: bool,
) -> String {
    let count_str = count.to_string();
    // Always use right alignment for better visual hierarchy
    let padding = width.saturating_sub(count_str.len());
    format!("{}{}", " ".repeat(padding), count_str)
}

/// Get the appropriate color for a message state based on its group
pub fn get_state_color(state: &MessageState) -> Color {
    match state {
        MessageState::Active | MessageState::Scheduled => ThemeManager::message_state_ready(),
        MessageState::Deferred => ThemeManager::message_state_deferred(),
        MessageState::Completed | MessageState::Abandoned => ThemeManager::message_state_outcome(),
        MessageState::DeadLettered => ThemeManager::message_state_failed(),
    }
}

/// Get display text for a message state
pub fn get_state_display(state: &MessageState) -> &'static str {
    match state {
        MessageState::Active => "Active",
        MessageState::Deferred => "Deferred",
        MessageState::Scheduled => "Scheduled",
        MessageState::DeadLettered => "Dead-lettered",
        MessageState::Completed => "Completed",
        MessageState::Abandoned => "Abandoned",
    }
}

/// Build table data from messages for the Table component
pub fn build_table_from_messages(
    messages: Option<&Vec<MessageModel>>,
    pagination_info: Option<&PaginationInfo>,
    selected_messages: &[MessageIdentifier],
    widths: &[u16],
    narrow_layout: bool,
) -> Vec<Vec<TextSpan>> {
    if let Some(messages) = messages {
        let mut builder = TableBuilder::default();
        let bulk_mode = pagination_info.is_some_and(|info| info.bulk_mode);

        for msg in messages {
            if bulk_mode {
                // Add checkbox column in bulk mode with themed checkboxes
                let message_id = MessageIdentifier::from_message(msg);
                let checkbox_text = if selected_messages.contains(&message_id) {
                    "● " // Filled circle for checked
                } else {
                    "○ " // Empty circle for unchecked
                };
                builder.add_col(TextSpan::from(checkbox_text));
            }

            let delivery_width = widths[if bulk_mode { 5 } else { 4 }];

            builder
                .add_col(TextSpan::from(msg.sequence.to_string()))
                .add_col(TextSpan::from(msg.id.to_string()))
                .add_col(TextSpan::from(msg.enqueued_at.to_string()))
                .add_col(TextSpan::from(get_state_display(&msg.state)))
                .add_col(TextSpan::from(format_delivery_count_responsive(
                    msg.delivery_count,
                    delivery_width as usize,
                    narrow_layout,
                )))
                .add_row();
        }
        return builder.build();
    }
    Vec::new()
}
