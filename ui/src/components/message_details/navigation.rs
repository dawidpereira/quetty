use super::component::MessageDetails;

impl MessageDetails {
    /// Move cursor up one line
    pub fn move_cursor_up(&mut self) {
        if self.cursor_line > 0 {
            self.cursor_line -= 1;
            // Adjust column if current line is shorter
            self.adjust_cursor_column();
        } else if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
            self.adjust_cursor_column();
        }
    }

    /// Move cursor down one line
    pub fn move_cursor_down(&mut self, visible_lines: usize) {
        let total_lines = self.message_content.len().max(1);
        let current_line = self.cursor_line + self.scroll_offset;

        if current_line + 1 < total_lines {
            if self.cursor_line + 1 < visible_lines {
                self.cursor_line += 1;
            } else {
                self.scroll_offset += 1;
            }
            self.adjust_cursor_column();
        }
    }

    /// Move cursor left one character
    pub fn move_cursor_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
    }

    /// Move cursor right one character
    pub fn move_cursor_right(&mut self) {
        let current_line_idx = self.cursor_line + self.scroll_offset;
        if let Some(line) = self.message_content.get(current_line_idx) {
            if self.cursor_col < line.len() {
                self.cursor_col += 1;
            }
        }
    }

    /// Move cursor to start of current line
    pub fn move_cursor_to_line_start(&mut self) {
        self.cursor_col = 0;
    }

    /// Move cursor to end of current line
    pub fn move_cursor_to_line_end(&mut self) {
        let current_line_idx = self.cursor_line + self.scroll_offset;
        if let Some(line) = self.message_content.get(current_line_idx) {
            self.cursor_col = line.len();
        }
    }

    /// Move cursor to top of document
    pub fn move_cursor_to_top(&mut self) {
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
    }

    /// Move cursor to bottom of document
    pub fn move_cursor_to_bottom(&mut self) {
        if !self.message_content.is_empty() {
            let last_line_idx = self.message_content.len() - 1;
            self.scroll_offset = last_line_idx;
            self.cursor_line = 0;
            self.cursor_col = 0;
        }
    }

    /// Handle page navigation (page up/down)
    pub fn handle_page_navigation(&mut self, page_up: bool) {
        let page_size = self.visible_lines.max(1);

        if page_up {
            // Page up
            if self.scroll_offset >= page_size {
                self.scroll_offset -= page_size;
            } else {
                self.scroll_offset = 0;
                self.cursor_line = 0;
            }
        } else {
            // Page down
            let total_lines = self.message_content.len().max(1);
            let max_scroll = total_lines.saturating_sub(page_size);

            if self.scroll_offset + page_size <= max_scroll {
                self.scroll_offset += page_size;
            } else {
                self.scroll_offset = max_scroll;
            }
        }

        self.adjust_cursor_column();
    }

    /// Adjust cursor column if it's beyond the end of the current line
    pub fn adjust_cursor_column(&mut self) {
        let current_line_idx = self.cursor_line + self.scroll_offset;
        if let Some(line) = self.message_content.get(current_line_idx) {
            if self.cursor_col > line.len() {
                self.cursor_col = line.len();
            }
        }
    }
}
