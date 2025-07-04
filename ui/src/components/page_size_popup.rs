use crate::components::common::{Msg, PopupActivityMsg};
use crate::components::state::ComponentState;
use crate::config::limits::{MAX_PAGE_SIZE, MIN_PAGE_SIZE};
use crate::theme::ThemeManager;
use tuirealm::{
    Component, Event, MockComponent, NoUserEvent, State, StateValue,
    command::{Cmd, CmdResult},
    event::{Key, KeyEvent, KeyModifiers},
    ratatui::{
        Frame,
        layout::{Alignment, Rect},
        style::{Modifier, Style},
        text::{Line, Span, Text},
        widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    },
};

/// A popup component for selecting page size with predefined options.
///
/// This component provides a user-friendly interface for selecting the number
/// of messages to display per page, with options from 100 to 1000 in 100-message intervals.
///
/// # Usage
///
/// ```rust
/// use quetty::components::page_size_popup::PageSizePopup;
///
/// let popup = PageSizePopup::new();
/// ```
///
/// # Events
///
/// - `KeyEvent::Enter` - Submits the selected page size
/// - `KeyEvent::Esc` - Cancels the selection
/// - Arrow keys - Navigate through options
/// - Number keys - Jump to specific option
///
/// # Messages
///
/// Emits `Msg::PopupActivity(PopupActivityMsg::PageSizeResult(size))` on successful selection.
pub struct PageSizePopup {
    options: Vec<u32>,
    selected_index: usize,
    is_mounted: bool,
}

impl PageSizePopup {
    /// Creates a new page size selection popup.
    ///
    /// # Returns
    ///
    /// A new `PageSizePopup` instance ready for mounting.
    pub fn new() -> Self {
        // Generate options from 100 to 1000 in 100-message intervals
        let options: Vec<u32> = (MIN_PAGE_SIZE..=MAX_PAGE_SIZE).step_by(100).collect();

        Self {
            options,
            selected_index: 0, // Default to first option (100)
            is_mounted: false,
        }
    }

    /// Gets the currently selected page size.
    ///
    /// # Returns
    ///
    /// The selected page size value.
    pub fn get_selected_size(&self) -> u32 {
        self.options[self.selected_index]
    }

    /// Renders the list of page size options.
    fn render_options(&self) -> Vec<Line> {
        self.options
            .iter()
            .enumerate()
            .map(|(index, &size)| {
                let is_selected = index == self.selected_index;
                let prefix = if is_selected { "● " } else { "○ " };
                let style = if is_selected {
                    Style::default()
                        .fg(ThemeManager::primary_accent())
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(ThemeManager::text_primary())
                };

                Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(format!("{size} messages per page"), style),
                ])
            })
            .collect()
    }
}

impl Default for PageSizePopup {
    fn default() -> Self {
        Self::new()
    }
}

impl MockComponent for PageSizePopup {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Create the border block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(ThemeManager::primary_accent()))
            .title(" Page Size Selection ")
            .title_alignment(Alignment::Center);

        // Create content lines
        let mut lines = Vec::new();

        // Add empty line at the top for better spacing
        lines.push(Line::from(""));

        // Add description
        lines.push(Line::from(vec![Span::styled(
            "Select the number of messages to display per page:",
            Style::default().fg(ThemeManager::text_primary()),
        )]));

        lines.push(Line::from(""));

        // Add options
        for option_line in self.render_options() {
            lines.push(option_line);
        }

        lines.push(Line::from(""));

        // Add instructions
        lines.push(Line::from(vec![Span::styled(
            "Use ↑/↓ or j/k to navigate, Enter to select, Esc to cancel",
            Style::default().fg(ThemeManager::text_muted()),
        )]));

        // Create the text widget
        let text = Text::from(lines);
        let paragraph = Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }

    fn query(&self, _attr: tuirealm::Attribute) -> Option<tuirealm::AttrValue> {
        None
    }

    fn attr(&mut self, _attr: tuirealm::Attribute, _value: tuirealm::AttrValue) {
        // No attributes to set
    }

    fn state(&self) -> State {
        State::One(StateValue::Usize(self.selected_index))
    }

    fn perform(&mut self, _cmd: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for PageSizePopup {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Enter,
                modifiers: KeyModifiers::NONE,
            }) => {
                // Submit the selected page size
                Some(Msg::PopupActivity(PopupActivityMsg::PageSizeResult(
                    self.get_selected_size() as usize,
                )))
            }
            Event::Keyboard(KeyEvent {
                code: Key::Esc,
                modifiers: KeyModifiers::NONE,
            }) => {
                // Cancel the selection
                Some(Msg::PopupActivity(PopupActivityMsg::ClosePageSize))
            }
            Event::Keyboard(KeyEvent {
                code: Key::Up,
                modifiers: KeyModifiers::NONE,
            }) => {
                // Move to previous option
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    Some(Msg::ForceRedraw)
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Down,
                modifiers: KeyModifiers::NONE,
            }) => {
                // Move to next option
                if self.selected_index < self.options.len() - 1 {
                    self.selected_index += 1;
                    Some(Msg::ForceRedraw)
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('k'),
                modifiers: KeyModifiers::NONE,
            }) => {
                // Move to previous option (vim-style)
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    Some(Msg::ForceRedraw)
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('j'),
                modifiers: KeyModifiers::NONE,
            }) => {
                // Move to next option (vim-style)
                if self.selected_index < self.options.len() - 1 {
                    self.selected_index += 1;
                    Some(Msg::ForceRedraw)
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) => {
                // Jump to specific option based on number key
                if let Some(digit) = c.to_digit(10) {
                    let target_size = digit * 100;
                    if let Some(index) = self.options.iter().position(|&size| size == target_size) {
                        self.selected_index = index;
                        Some(Msg::ForceRedraw)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl ComponentState for PageSizePopup {
    fn mount(&mut self) -> crate::error::AppResult<()> {
        log::debug!("Mounting PageSizePopup component");

        if self.is_mounted {
            log::warn!("PageSizePopup is already mounted");
            return Ok(());
        }

        self.is_mounted = true;
        log::debug!("PageSizePopup component mounted successfully");
        Ok(())
    }
}

impl Drop for PageSizePopup {
    fn drop(&mut self) {
        if self.is_mounted {
            log::debug!("PageSizePopup component dropped");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tuirealm::event::{Key, KeyEvent, KeyModifiers};

    #[test]
    fn test_page_size_popup_creation() {
        let popup = PageSizePopup::new();
        assert_eq!(popup.options.len(), 10); // 100, 200, ..., 1000
        assert_eq!(popup.options[0], 100);
        assert_eq!(popup.options[9], 1000);
        assert_eq!(popup.selected_index, 0);
    }

    #[test]
    fn test_get_selected_size() {
        let mut popup = PageSizePopup::new();
        assert_eq!(popup.get_selected_size(), 100);

        popup.selected_index = 4;
        assert_eq!(popup.get_selected_size(), 500);
    }

    #[test]
    fn test_options_generation() {
        let popup = PageSizePopup::new();
        let expected: Vec<u32> = (100..=1000).step_by(100).collect();
        assert_eq!(popup.options, expected);
    }

    #[test]
    fn test_navigation() {
        let mut popup = PageSizePopup::new();

        // Test initial state
        assert_eq!(popup.selected_index, 0);
        assert_eq!(popup.get_selected_size(), 100);

        // Test moving down
        popup.selected_index = 1;
        assert_eq!(popup.get_selected_size(), 200);

        // Test moving to middle
        popup.selected_index = 4;
        assert_eq!(popup.get_selected_size(), 500);

        // Test moving to end
        popup.selected_index = 9;
        assert_eq!(popup.get_selected_size(), 1000);
    }

    #[test]
    fn test_number_key_navigation() {
        let mut popup = PageSizePopup::new();

        // Test number key '3' should select 300
        if let Some(digit) = '3'.to_digit(10) {
            let target_size = digit * 100;
            if let Some(index) = popup.options.iter().position(|&size| size == target_size) {
                popup.selected_index = index;
            }
        }
        assert_eq!(popup.get_selected_size(), 300);

        // Test number key '7' should select 700
        if let Some(digit) = '7'.to_digit(10) {
            let target_size = digit * 100;
            if let Some(index) = popup.options.iter().position(|&size| size == target_size) {
                popup.selected_index = index;
            }
        }
        assert_eq!(popup.get_selected_size(), 700);
    }

    #[test]
    fn test_arrow_key_events() {
        let mut popup = PageSizePopup::new();
        assert_eq!(popup.selected_index, 0);

        // Test down arrow
        let down_event = Event::Keyboard(KeyEvent {
            code: Key::Down,
            modifiers: KeyModifiers::NONE,
        });
        let result = popup.on(down_event);
        assert_eq!(result, Some(Msg::ForceRedraw));
        assert_eq!(popup.selected_index, 1);

        // Test up arrow
        let up_event = Event::Keyboard(KeyEvent {
            code: Key::Up,
            modifiers: KeyModifiers::NONE,
        });
        let result = popup.on(up_event);
        assert_eq!(result, Some(Msg::ForceRedraw));
        assert_eq!(popup.selected_index, 0);

        // Test up arrow at top (should not change)
        let up_event = Event::Keyboard(KeyEvent {
            code: Key::Up,
            modifiers: KeyModifiers::NONE,
        });
        let result = popup.on(up_event);
        assert_eq!(result, None);
        assert_eq!(popup.selected_index, 0);

        // Test down arrow at bottom (should not change)
        popup.selected_index = popup.options.len() - 1;
        let down_event = Event::Keyboard(KeyEvent {
            code: Key::Down,
            modifiers: KeyModifiers::NONE,
        });
        let result = popup.on(down_event);
        assert_eq!(result, None);
        assert_eq!(popup.selected_index, popup.options.len() - 1);
    }

    #[test]
    fn test_jk_navigation() {
        let mut popup = PageSizePopup::new();
        assert_eq!(popup.selected_index, 0);

        // Test 'j' key (move down)
        let j_event = Event::Keyboard(KeyEvent {
            code: Key::Char('j'),
            modifiers: KeyModifiers::NONE,
        });
        let result = popup.on(j_event);
        assert_eq!(result, Some(Msg::ForceRedraw));
        assert_eq!(popup.selected_index, 1);

        // Test 'k' key (move up)
        let k_event = Event::Keyboard(KeyEvent {
            code: Key::Char('k'),
            modifiers: KeyModifiers::NONE,
        });
        let result = popup.on(k_event);
        assert_eq!(result, Some(Msg::ForceRedraw));
        assert_eq!(popup.selected_index, 0);

        // Test 'k' key at top (should not change)
        let k_event = Event::Keyboard(KeyEvent {
            code: Key::Char('k'),
            modifiers: KeyModifiers::NONE,
        });
        let result = popup.on(k_event);
        assert_eq!(result, None);
        assert_eq!(popup.selected_index, 0);

        // Test 'j' key at bottom (should not change)
        popup.selected_index = popup.options.len() - 1;
        let j_event = Event::Keyboard(KeyEvent {
            code: Key::Char('j'),
            modifiers: KeyModifiers::NONE,
        });
        let result = popup.on(j_event);
        assert_eq!(result, None);
        assert_eq!(popup.selected_index, popup.options.len() - 1);
    }

    #[test]
    fn test_enter_and_escape_events() {
        let mut popup = PageSizePopup::new();

        // Test Enter should return PageSizeResult
        let enter_event = Event::Keyboard(KeyEvent {
            code: Key::Enter,
            modifiers: KeyModifiers::NONE,
        });

        if let Some(msg) = popup.on(enter_event) {
            match msg {
                Msg::PopupActivity(PopupActivityMsg::PageSizeResult(size)) => {
                    assert_eq!(size, 100); // Default selection
                }
                _ => panic!("Enter should return PageSizeResult message"),
            }
        } else {
            panic!("Enter should return a message");
        }

        // Test Escape should return ClosePageSize
        let escape_event = Event::Keyboard(KeyEvent {
            code: Key::Esc,
            modifiers: KeyModifiers::NONE,
        });

        if let Some(msg) = popup.on(escape_event) {
            match msg {
                Msg::PopupActivity(PopupActivityMsg::ClosePageSize) => {
                    // Expected
                }
                _ => panic!("Escape should return ClosePageSize message"),
            }
        } else {
            panic!("Escape should return a message");
        }
    }
}
