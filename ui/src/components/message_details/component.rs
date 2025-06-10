use crate::components::common::Msg;
use crate::error::AppError;
use server::model::{BodyData, MessageModel};
use tuirealm::{
    AttrValue, Attribute, Component, Frame, MockComponent, NoUserEvent, State, StateValue,
    command::{Cmd, CmdResult},
    event::Event,
    ratatui::layout::Rect,
};

pub struct MessageDetails {
    pub message_content: Vec<String>,
    pub original_content: Vec<String>, // Store original content for restore on escape
    pub current_message: Option<MessageModel>, // Store current message for operations
    pub scroll_offset: usize,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub is_focused: bool,
    pub visible_lines: usize,
    pub is_editing: bool,            // Track if we're in edit mode
    pub is_dirty: bool,              // Track if content has been modified
    pub repeat_count: Option<usize>, // Track how many times message will be sent (for composition mode)
}

impl MessageDetails {
    pub fn new(message: Option<MessageModel>) -> Self {
        Self::new_with_focus(message, false)
    }

    pub fn new_with_focus(message: Option<MessageModel>, is_focused: bool) -> Self {
        let message_content = Self::format_message_content(&message);
        let original_content = message_content.clone();

        Self {
            message_content,
            original_content,
            current_message: message,
            scroll_offset: 0,
            cursor_line: 0,
            cursor_col: 0,
            is_focused,
            visible_lines: 0,
            is_editing: false,
            is_dirty: false,
            repeat_count: None,
        }
    }

    pub fn new_for_composition_with_repeat_count(
        message: Option<MessageModel>,
        is_focused: bool,
        repeat_count: usize,
    ) -> Self {
        // Start with empty content for composition
        let message_content = vec![String::new()];
        let original_content = message_content.clone();

        Self {
            message_content,
            original_content,
            current_message: message,
            scroll_offset: 0,
            cursor_line: 0,
            cursor_col: 0,
            is_focused,
            visible_lines: 0,
            is_editing: true, // Start in edit mode for composition
            is_dirty: false,
            repeat_count: Some(repeat_count),
        }
    }

    /// Format message content based on the message data type
    fn format_message_content(message: &Option<MessageModel>) -> Vec<String> {
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

    /// Get current edited content as string
    pub fn get_edited_content(&self) -> String {
        self.message_content.join("\n")
    }

    /// Validate message content before sending
    pub fn validate_message_content(&self, content: &str) -> Result<(), AppError> {
        use super::validation::CompleteMessageValidator;
        use crate::validation::Validator;

        let validator = CompleteMessageValidator::azure_default();
        validator.validate(content).map_err(Into::into)
    }
}

impl MockComponent for MessageDetails {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Delegate to rendering module
        super::rendering::render_message_details(self, frame, area);
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        match attr {
            Attribute::Custom("cursor_position") => Some(AttrValue::Number(
                (self.cursor_line + self.scroll_offset) as isize,
            )),
            Attribute::Custom("focus") => Some(AttrValue::Flag(self.is_focused)),
            Attribute::Custom("edit_mode") => Some(AttrValue::Flag(self.is_editing)),
            Attribute::Custom("is_dirty") => Some(AttrValue::Flag(self.is_dirty)),
            Attribute::Text => Some(AttrValue::String(self.get_edited_content())),
            _ => None,
        }
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        match attr {
            Attribute::Custom("cursor_position") => {
                if let AttrValue::Number(position) = value {
                    let target_position = position as usize;
                    let max_line = self.message_content.len().saturating_sub(1);
                    let bounded_position = target_position.min(max_line);

                    // Calculate which line should be the cursor line and scroll offset
                    let visible_lines = self.visible_lines;
                    if bounded_position < visible_lines {
                        // Target is within first page - no scrolling needed
                        self.cursor_line = bounded_position;
                        self.scroll_offset = 0;
                    } else {
                        // Target requires scrolling
                        self.scroll_offset =
                            bounded_position.saturating_sub(visible_lines.saturating_sub(1));
                        self.cursor_line = bounded_position - self.scroll_offset;
                    }

                    // Ensure cursor column is within line bounds
                    self.adjust_cursor_column();
                }
            }
            Attribute::Custom("focus") => {
                if let AttrValue::Flag(focused) = value {
                    self.is_focused = focused;
                }
            }
            Attribute::Custom("edit_mode") => {
                if let AttrValue::Flag(editing) = value {
                    self.is_editing = editing;
                }
            }
            Attribute::Text => {
                if let AttrValue::String(content) = value {
                    self.message_content = content.lines().map(String::from).collect();
                    if self.message_content.is_empty() {
                        self.message_content.push(String::new());
                    }
                    // Reset cursor to beginning when setting new content
                    self.cursor_line = 0;
                    self.cursor_col = 0;
                    self.scroll_offset = 0;
                    self.is_dirty = false;
                }
            }
            _ => {}
        }
    }

    fn state(&self) -> State {
        State::One(StateValue::Usize(self.cursor_line + self.scroll_offset))
    }

    fn perform(&mut self, _cmd: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for MessageDetails {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        // Delegate to event handling module
        super::event_handling::handle_event(self, ev)
    }
}
