use crate::components::common::{MessageActivityMsg, Msg};
use crate::theme::ThemeManager;
use server::model::{BodyData, MessageModel};
use tuirealm::{
    AttrValue, Attribute, Component, Frame, MockComponent, NoUserEvent, State, StateValue,
    command::{Cmd, CmdResult},
    event::{Event, Key, KeyEvent, KeyModifiers},
    ratatui::{
        layout::{Alignment, Rect},
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    },
};

pub struct MessageDetails {
    message_content: Vec<String>,
    scroll_offset: usize,
    cursor_line: usize,
    cursor_col: usize,
    is_focused: bool,
    visible_lines: usize,
}

impl MessageDetails {
    pub fn new(message: Option<MessageModel>) -> Self {
        Self::new_with_focus(message, false)
    }

    pub fn new_with_focus(message: Option<MessageModel>, is_focused: bool) -> Self {
        let message_content = Self::format_message_content(message);

        Self {
            message_content,
            scroll_offset: 0,
            cursor_line: 0,
            cursor_col: 0,
            is_focused,
            visible_lines: 0,
        }
    }

    /// Format message content based on the message data type
    fn format_message_content(message: Option<MessageModel>) -> Vec<String> {
        match message {
            Some(data) => {
                match &data.body {
                    BodyData::ValidJson(json) => {
                        // If it's valid JSON, show it pretty-printed
                        match serde_json::to_string_pretty(json) {
                            Ok(json_str) => json_str.lines().map(String::from).collect(),
                            Err(e) => vec![format!("JSON formatting error: {}", e)],
                        }
                    }
                    BodyData::RawString(body_str) => {
                        // Show raw string with line breaks
                        body_str.lines().map(String::from).collect()
                    }
                }
            }
            None => vec!["No message selected".to_string()],
        }
    }

    // === Navigation Methods ===

    fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    fn scroll_down(&mut self, visible_lines: usize) {
        let max_scroll = if self.message_content.len() > visible_lines {
            self.message_content.len() - visible_lines
        } else {
            0
        };

        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
    }

    fn move_cursor_up(&mut self) {
        if self.cursor_line > 0 {
            // Move cursor up within visible area
            self.cursor_line -= 1;
        } else if self.scroll_offset > 0 {
            // At top of visible area, scroll up
            self.scroll_offset -= 1;
            // Keep cursor at the same relative position (top)
        }

        self.adjust_cursor_column();
    }

    fn move_cursor_down(&mut self, visible_lines: usize) {
        let current_absolute_line = self.cursor_line + self.scroll_offset;

        // Check if we can move down in the document
        if current_absolute_line < self.message_content.len().saturating_sub(1) {
            // If we're at the bottom of the visible area, scroll down
            if self.cursor_line >= visible_lines.saturating_sub(1) {
                self.scroll_offset += 1;
                // Keep cursor at the same relative position
            } else {
                // Move cursor down within visible area
                self.cursor_line += 1;
            }

            self.adjust_cursor_column();
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
    }

    fn move_cursor_right(&mut self) {
        if let Some(line) = self.get_current_line() {
            if self.cursor_col < line.len() {
                self.cursor_col += 1;
            }
        }
    }

    /// Adjust cursor column to ensure it's within the current line bounds
    fn adjust_cursor_column(&mut self) {
        if let Some(line) = self.get_current_line() {
            if self.cursor_col > line.len() {
                self.cursor_col = line.len();
            }
        }
    }

    /// Get the current line content at cursor position
    fn get_current_line(&self) -> Option<&String> {
        self.message_content
            .get(self.cursor_line + self.scroll_offset)
    }

    // === Rendering Methods ===

    /// Create the block widget with proper styling
    fn create_block(&self) -> Block {
        let border_color = if self.is_focused {
            ThemeManager::primary_accent() // Teal when focused
        } else {
            Color::White // White when not focused
        };

        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(" 📄 Message Details ")
            .title_alignment(Alignment::Center)
            .title_style(
                Style::default()
                    .fg(ThemeManager::title_accent()) // Use same pink as table headers
                    .add_modifier(Modifier::BOLD),
            )
    }

    /// Create line content with line numbers and cursor highlighting
    fn create_content_lines(&self, visible_lines: usize) -> Vec<Line> {
        let mut lines = Vec::new();
        let start_line = self.scroll_offset;
        let end_line = (start_line + visible_lines).min(self.message_content.len());

        for (display_line, line_idx) in (start_line..end_line).enumerate() {
            if let Some(content) = self.message_content.get(line_idx) {
                let line = self.create_single_line(content, line_idx, display_line);
                lines.push(line);
            }
        }

        lines
    }

    /// Create a single line with line number and optional cursor highlighting
    fn create_single_line<'a>(
        &self,
        content: &'a str,
        line_idx: usize,
        display_line: usize,
    ) -> Line<'a> {
        let line_number = line_idx + 1;
        let line_num_str = format!("{:4} ", line_number);

        let mut spans = vec![Span::styled(
            line_num_str,
            Style::default()
                .fg(ThemeManager::header_accent()) // Always yellow to match table headers
                .add_modifier(Modifier::ITALIC),
        )];

        // Add cursor highlighting if this is the cursor line and we're focused
        if display_line == self.cursor_line && self.is_focused {
            spans.extend(self.create_cursor_highlighted_spans(content));
        } else {
            // Normal line without cursor
            spans.push(Span::styled(
                content,
                Style::default().fg(ThemeManager::text_primary()),
            ));
        }

        Line::from(spans)
    }

    /// Create spans for a line with cursor highlighting
    fn create_cursor_highlighted_spans<'a>(&self, content: &'a str) -> Vec<Span<'a>> {
        let mut spans = Vec::new();

        // Split the content at cursor position
        let (before_cursor, at_and_after_cursor) =
            content.split_at(self.cursor_col.min(content.len()));

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

    /// Create the status bar widget
    fn create_status_bar(&self) -> Paragraph {
        let status_text = format!(
            "Ln {}, Col {} | Press <ESC> to quit",
            self.cursor_line + self.scroll_offset + 1,
            self.cursor_col + 1
        );

        Paragraph::new(status_text)
            .style(
                Style::default().fg(if self.is_focused {
                    ThemeManager::primary_accent() // Teal text when focused
                } else {
                    Color::White // White text when not focused
                }), // No background - clean and transparent
            )
            .alignment(Alignment::Center)
    }

    /// Calculate the area for the status bar overlay
    fn calculate_status_area(&self, area: Rect) -> Rect {
        Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: 1,
        }
    }

    // === Event Handling Methods ===

    /// Handle page navigation events
    fn handle_page_navigation(&mut self, is_up: bool) {
        if is_up {
            for _ in 0..10 {
                self.scroll_up();
            }
        } else {
            for _ in 0..10 {
                self.scroll_down(self.visible_lines);
            }
        }
    }
}

impl MockComponent for MessageDetails {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Calculate available area for content (excluding borders)
        let content_height = area.height.saturating_sub(2); // 2 for borders only
        let visible_lines = content_height as usize;

        // Store visible_lines for use in keyboard events
        self.visible_lines = visible_lines;

        // Create and render the main content
        let content_lines = self.create_content_lines(visible_lines);
        let block = self.create_block();
        let paragraph = Paragraph::new(content_lines)
            .block(block)
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);

        // Create and render the status bar overlay
        let status_bar = self.create_status_bar();
        let status_area = self.calculate_status_area(area);
        frame.render_widget(status_bar, status_area);
    }

    fn query(&self, _attr: Attribute) -> Option<AttrValue> {
        None
    }

    fn attr(&mut self, _attr: Attribute, _value: AttrValue) {
        // No attributes supported
    }

    fn state(&self) -> State {
        State::One(StateValue::Usize(self.cursor_line))
    }

    fn perform(&mut self, _cmd: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for MessageDetails {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Esc,
                modifiers: KeyModifiers::NONE,
            }) => {
                return Some(Msg::MessageActivity(MessageActivityMsg::CancelEditMessage));
            }

            Event::Keyboard(KeyEvent {
                code: Key::Up,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.move_cursor_up();
            }

            Event::Keyboard(KeyEvent {
                code: Key::Down,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.move_cursor_down(self.visible_lines);
            }

            Event::Keyboard(KeyEvent {
                code: Key::Left,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.move_cursor_left();
            }

            Event::Keyboard(KeyEvent {
                code: Key::Right,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.move_cursor_right();
            }

            Event::Keyboard(KeyEvent {
                code: Key::PageUp,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.handle_page_navigation(true);
            }

            Event::Keyboard(KeyEvent {
                code: Key::PageDown,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.handle_page_navigation(false);
            }

            _ => {}
        }

        Some(Msg::ForceRedraw)
    }
}
