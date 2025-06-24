use super::component::MessageDetails;
use crate::theme::ThemeManager;
use tuirealm::{
    Frame,
    ratatui::{
        layout::{Alignment, Rect},
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    },
};

pub fn render_message_details(details: &mut MessageDetails, frame: &mut Frame, area: Rect) {
    // Calculate available area for content (excluding borders)
    let content_height = area.height.saturating_sub(2); // 2 for borders only
    let visible_lines = content_height as usize;

    // Store visible_lines for use in keyboard events
    details.visible_lines = visible_lines;

    // Create and render the main content
    let content_lines = create_content_lines(details, visible_lines);
    let block = create_block(details);
    let paragraph = Paragraph::new(content_lines)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);

    // Create and render the status bar overlay
    let status_bar = create_status_bar(details);
    let status_area = calculate_status_area(area);
    frame.render_widget(status_bar, status_area);
}

/// Create the block widget with proper styling
fn create_block(details: &MessageDetails) -> Block {
    let border_color = if details.is_focused {
        if details.is_editing {
            Color::Red // Red border when editing
        } else {
            ThemeManager::primary_accent() // Teal when focused
        }
    } else {
        Color::White // White when not focused
    };

    let title = if details.is_editing {
        if details.is_dirty {
            " ✏️ Message Details - EDITING (modified) "
        } else {
            " ✏️ Message Details - EDITING "
        }
    } else {
        " 📄 Message Details "
    };

    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(title)
        .title_alignment(Alignment::Center)
        .title_style(
            Style::default()
                .fg(ThemeManager::title_accent())
                .add_modifier(Modifier::BOLD),
        )
}

/// Create content lines for display
fn create_content_lines(details: &MessageDetails, visible_lines: usize) -> Vec<Line> {
    let mut lines = Vec::new();

    // Calculate display range
    let start = details.scroll_offset;
    let end = (start + visible_lines).min(details.message_content.len());

    for (display_line, line_idx) in (start..end).enumerate() {
        if let Some(content) = details.message_content.get(line_idx) {
            let line = create_single_line(details, content, display_line);
            lines.push(line);
        }
    }

    // Fill remaining lines if needed
    while lines.len() < visible_lines {
        lines.push(Line::from(""));
    }

    lines
}

/// Create a single line with cursor highlighting
fn create_single_line<'a>(
    details: &MessageDetails,
    content: &'a str,
    display_line: usize,
) -> Line<'a> {
    let is_cursor_line = details.is_focused && display_line == details.cursor_line;

    if is_cursor_line {
        Line::from(create_cursor_highlighted_spans(details, content))
    } else {
        Line::from(content)
    }
}

/// Create spans with cursor highlighting for the current line
fn create_cursor_highlighted_spans<'a>(
    details: &MessageDetails,
    content: &'a str,
) -> Vec<Span<'a>> {
    let mut spans = Vec::new();
    let cursor_pos = details.cursor_col;

    // Split the content at cursor position
    let (before_cursor, at_and_after_cursor) = content.split_at(cursor_pos.min(content.len()));

    // Add text before cursor
    if !before_cursor.is_empty() {
        spans.push(Span::styled(
            before_cursor,
            Style::default().fg(ThemeManager::text_primary()),
        ));
    }

    // Add cursor character with highlighting
    if let Some(cursor_char) = at_and_after_cursor.chars().next() {
        spans.push(Span::styled(
            cursor_char.to_string(),
            Style::default()
                .bg(ThemeManager::selection_bg()) // Same as selected message row
                .fg(ThemeManager::selection_fg())
                .add_modifier(Modifier::REVERSED),
        ));

        // Add remaining text after cursor
        let after_cursor = &at_and_after_cursor[cursor_char.len_utf8()..];
        if !after_cursor.is_empty() {
            spans.push(Span::styled(
                after_cursor,
                Style::default().fg(ThemeManager::text_primary()),
            ));
        }
    } else {
        // Cursor at end of line - show a space with cursor styling
        spans.push(Span::styled(
            " ",
            Style::default()
                .bg(ThemeManager::selection_bg())
                .fg(ThemeManager::selection_fg())
                .add_modifier(Modifier::REVERSED),
        ));
    }

    spans
}

/// Create the status bar showing current position and mode
fn create_status_bar(details: &MessageDetails) -> Paragraph {
    let status_text = if details.is_editing {
        let keys = crate::config::get_config_or_panic().keys();

        // Add repeat count info if we're in composition mode
        let repeat_info = if let Some(count) = details.repeat_count {
            if count == 1 {
                " | Will send 1 time".to_string()
            } else {
                format!(" | Will send {} times", count)
            }
        } else {
            String::new()
        };

        format!(
            "Ln {}, Col {} | EDIT MODE{} | Ctrl+{}: Send | Ctrl+{}: Replace | ESC: Cancel",
            details.cursor_line + details.scroll_offset + 1,
            details.cursor_col + 1,
            repeat_info,
            keys.send_edited_message(),
            keys.replace_edited_message()
        )
    } else {
        format!(
            "Ln {}, Col {} | Press 'e' or 'i' to edit | ESC: Back to messages",
            details.cursor_line + details.scroll_offset + 1,
            details.cursor_col + 1
        )
    };

    Paragraph::new(status_text)
        .style(
            Style::default().fg(if details.is_focused {
                if details.is_editing {
                    Color::Red // Red text when editing
                } else {
                    ThemeManager::primary_accent() // Teal text when focused
                }
            } else {
                Color::White // White text when not focused
            }), // No background - clean and transparent
        )
        .alignment(Alignment::Center)
}

/// Calculate the area for the status bar overlay
fn calculate_status_area(area: Rect) -> Rect {
    Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    }
}
