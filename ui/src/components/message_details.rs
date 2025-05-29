use server::model::BodyData;
use server::model::MessageModel;
use tui_realm_textarea::{
    TEXTAREA_CMD_NEWLINE, TEXTAREA_CMD_PASTE, TEXTAREA_CMD_REDO, TEXTAREA_CMD_UNDO, TextArea,
};
use tuirealm::{
    Component, MockComponent, NoUserEvent,
    command::{Cmd, CmdResult, Direction},
    event::{Event, Key, KeyEvent, KeyModifiers},
    props::{Alignment, BorderType, Borders, Color, Style, TextModifiers},
};
use tui_realm_stdlib::Paragraph;

use crate::components::common::{MessageActivityMsg, Msg};

#[derive(MockComponent)]
pub struct MessageDetails {
    component: TextArea<'static>,
}

const CMD_CANCEL_EDIT_MESSAGE: &str = "CancelEditMessage";

//TODO: Add search
impl MessageDetails {
    pub fn new(message: Option<MessageModel>) -> Self {
        let mut textarea = match message {
            Some(data) => {
                match &data.body {
                    BodyData::ValidJson(json) => {
                        // If it's valid JSON, show it pretty-printed
                        match serde_json::to_string_pretty(json) {
                            Ok(json_str) => {
                                let lines: Vec<String> =
                                    json_str.lines().map(String::from).collect();
                                TextArea::new(lines)
                            }
                            Err(e) => TextArea::new(vec![format!("JSON formatting error: {}", e)]),
                        }
                    }
                    BodyData::RawString(body_str) => {
                        // Show raw string with line breaks
                        let lines: Vec<String> = body_str.lines().map(String::from).collect();
                        TextArea::new(lines)
                    }
                }
            }
            None => TextArea::new(vec!["No message selected".to_string()]),
        };

        textarea = textarea
            .borders(
                Borders::default()
                    .color(Color::Green)
                    .modifiers(BorderType::Rounded),
            )
            .title(" Message details ", Alignment::Center)
            .cursor_style(Style::default().add_modifier(TextModifiers::REVERSED))
            .cursor_line_style(Style::default())
            .footer_bar("Press <ESC> to quit", Style::default())
            .line_number_style(
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(TextModifiers::ITALIC),
            )
            .max_histories(64)
            .scroll_step(4)
            .status_bar(
                "Ln {ROW}, Col {COL}",
                Style::default().add_modifier(TextModifiers::REVERSED),
            )
            .tab_length(4);

        Self {
            component: textarea,
        }
    }
}

impl Component<Msg, NoUserEvent> for MessageDetails {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let cmd_result = match ev {
            // Handle modifiers actions
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::CONTROL,
            }) => match c {
                'v' => self.component.perform(Cmd::Custom(TEXTAREA_CMD_PASTE)),
                'z' => self.component.perform(Cmd::Custom(TEXTAREA_CMD_UNDO)),
                'y' => self.component.perform(Cmd::Custom(TEXTAREA_CMD_REDO)),
                'h' => self.component.perform(Cmd::Move(Direction::Left)),
                'j' => self.component.perform(Cmd::Move(Direction::Down)),
                'k' => self.component.perform(Cmd::Move(Direction::Up)),
                'l' => self.component.perform(Cmd::Move(Direction::Right)),
                _ => CmdResult::None,
            },

            // Handle submit
            Event::Keyboard(KeyEvent {
                code: Key::Enter,
                modifiers: KeyModifiers::ALT,
            }) => {
                match self.component.perform(Cmd::Submit) {
                    CmdResult::Submit(state) => {
                        // Safely convert state to vec without unwrap
                        match state {
                            tuirealm::State::Vec(items) => {
                                // Safely convert each item to string without unwrap
                                let mut lines = Vec::new();
                                for item in items {
                                    match item {
                                        tuirealm::StateValue::String(s) => lines.push(s),
                                        _ => {
                                            // For non-string values, provide a fallback
                                            log::warn!(
                                                "Unexpected state value type in message details"
                                            );
                                            lines.push(String::from("[Non-string content]"));
                                        }
                                    }
                                }
                                return Some(Msg::Submit(lines));
                            }
                            _ => {
                                log::warn!("Unexpected state type in message details");
                                return None;
                            }
                        }
                    }
                    _ => return None,
                }
            }

            // Handle keys with no modifiers
            Event::Keyboard(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
            }) => match code {
                Key::PageDown => self.component.perform(Cmd::Scroll(Direction::Down)),
                Key::PageUp => self.component.perform(Cmd::Scroll(Direction::Up)),
                Key::Left => self.component.perform(Cmd::Move(Direction::Left)),
                Key::Down => self.component.perform(Cmd::Move(Direction::Down)),
                Key::Up => self.component.perform(Cmd::Move(Direction::Up)),
                Key::Right => self.component.perform(Cmd::Move(Direction::Right)),
                Key::Backspace => self.component.perform(Cmd::Delete),
                Key::Enter => self.component.perform(Cmd::Custom(TEXTAREA_CMD_NEWLINE)),
                Key::Tab => self.component.perform(Cmd::Type('\t')),
                Key::Esc => CmdResult::Custom(CMD_CANCEL_EDIT_MESSAGE, self.state()),

                // Handle typing
                Key::Char(ch) => self.component.perform(Cmd::Type(ch)),

                _ => CmdResult::None,
            },

            Event::Keyboard(KeyEvent { code, modifiers: _ }) => {
                if let Key::Char(ch) = code {
                    self.component.perform(Cmd::Type(ch))
                } else {
                    CmdResult::None
                }
            }

            _ => CmdResult::None,
        };

        match cmd_result {
            CmdResult::Custom(CMD_CANCEL_EDIT_MESSAGE, _) => {
                Some(Msg::MessageActivity(MessageActivityMsg::CancelEditMessage))
            }
            _ => Some(Msg::ForceRedraw),
        }
    }
}
