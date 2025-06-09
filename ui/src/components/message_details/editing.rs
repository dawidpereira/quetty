use super::component::MessageDetails;
use copypasta::{ClipboardContext, ClipboardProvider};

impl MessageDetails {
    /// Toggle edit mode
    pub fn toggle_edit_mode(&mut self) {
        self.is_editing = !self.is_editing;
        if !self.is_editing {
            // Exiting edit mode, check if content changed
            self.is_dirty = self.message_content != self.original_content;
        }
    }

    /// Restore original content (for escape key)
    pub fn restore_original_content(&mut self) {
        self.message_content = self.original_content.clone();
        self.is_dirty = false;
        self.is_editing = false;
    }

    /// Insert character at cursor position
    pub fn insert_char(&mut self, ch: char) {
        if !self.is_editing {
            return;
        }

        // Ensure we have a line at cursor position
        while self.cursor_line + self.scroll_offset >= self.message_content.len() {
            self.message_content.push(String::new());
        }

        let line_idx = self.cursor_line + self.scroll_offset;
        if let Some(line) = self.message_content.get_mut(line_idx) {
            line.insert(self.cursor_col, ch);
            self.cursor_col += 1;
            self.is_dirty = true;
        }
    }

    /// Delete character at cursor position (backspace)
    pub fn delete_char_backward(&mut self) {
        if !self.is_editing || self.cursor_col == 0 {
            return;
        }

        let line_idx = self.cursor_line + self.scroll_offset;
        if let Some(line) = self.message_content.get_mut(line_idx) {
            if self.cursor_col > 0 && self.cursor_col <= line.len() {
                line.remove(self.cursor_col - 1);
                self.cursor_col -= 1;
                self.is_dirty = true;
            }
        }
    }

    /// Delete character at cursor position (delete key)
    pub fn delete_char_forward(&mut self) {
        if !self.is_editing {
            return;
        }

        let line_idx = self.cursor_line + self.scroll_offset;
        if let Some(line) = self.message_content.get_mut(line_idx) {
            if self.cursor_col < line.len() {
                line.remove(self.cursor_col);
                self.is_dirty = true;
            }
        }
    }

    /// Insert new line at cursor position
    pub fn insert_newline(&mut self) {
        if !self.is_editing {
            return;
        }

        let line_idx = self.cursor_line + self.scroll_offset;
        if let Some(line) = self.message_content.get_mut(line_idx) {
            let new_line = line.split_off(self.cursor_col);
            self.message_content.insert(line_idx + 1, new_line);
            self.cursor_line += 1;
            self.cursor_col = 0;
            self.is_dirty = true;
        }
    }

    /// Copy message content to clipboard
    pub fn copy_to_clipboard(&self) -> Result<(), String> {
        let mut ctx = ClipboardContext::new()
            .map_err(|e| format!("Failed to create clipboard context: {}", e))?;

        let content = self.get_edited_content();
        ctx.set_contents(content)
            .map_err(|e| format!("Failed to set clipboard contents: {}", e))?;

        Ok(())
    }
}
