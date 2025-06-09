use super::component::MessageDetails;

impl MessageDetails {
    pub fn move_cursor_up(&mut self) {
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

    pub fn move_cursor_down(&mut self, visible_lines: usize) {
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

    pub fn move_cursor_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if let Some(line) = self.get_current_line() {
            if self.cursor_col < line.len() {
                self.cursor_col += 1;
            }
        }
    }

    /// Adjust cursor column to ensure it's within the current line bounds
    pub fn adjust_cursor_column(&mut self) {
        if let Some(line) = self.get_current_line() {
            if self.cursor_col > line.len() {
                self.cursor_col = line.len();
            }
        }
    }

    pub fn handle_page_navigation(&mut self, is_up: bool) {
        let page_size = self.visible_lines.saturating_sub(1);

        if is_up {
            // Page up
            if self.scroll_offset >= page_size {
                self.scroll_offset -= page_size;
            } else {
                self.scroll_offset = 0;
            }
        } else {
            // Page down
            let max_scroll = if self.message_content.len() > self.visible_lines {
                self.message_content.len() - self.visible_lines
            } else {
                0
            };

            if self.scroll_offset + page_size < max_scroll {
                self.scroll_offset += page_size;
            } else {
                self.scroll_offset = max_scroll;
            }
        }
    }
}
