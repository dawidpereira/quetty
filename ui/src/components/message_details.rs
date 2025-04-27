use tui_realm_textarea::{TextArea, TEXTAREA_CMD_NEWLINE, TEXTAREA_CMD_PASTE, TEXTAREA_CMD_REDO, TEXTAREA_CMD_UNDO};
use tuirealm::{
    command::{Cmd, CmdResult, Direction},
    event::{Event, Key, KeyEvent, KeyModifiers},
    props::{Alignment, BorderType, Borders, Color, Style, TextModifiers},
    Component, MockComponent, NoUserEvent,
};

use super::common::Msg;

#[derive(MockComponent)]
pub struct MessageDetails {
    component: TextArea<'static>,
}

impl MessageDetails {
    pub fn new() -> Self {
        // JSON message as &'static str (string literal)
        let json_body = r#"{
  "event": "OrderPlaced",
  "timestamp": "2025-04-26T14:30:00+02:00",
  "user": {
    "id": "user_12345",
    "name": "John Doe",
    "email": "john.doe@example.com"
  },
  "order": {
    "order_id": "order_98765",
    "items": [
      {
        "product_id": "prod_001",
        "name": "Wireless Mouse",
        "quantity": 2,
        "price": 79.99
      },
      {
        "product_id": "prod_002",
        "name": "Mechanical Keyboard",
        "quantity": 1,
        "price": 349.50
      }
    ],
    "total": 509.48,
    "currency": "PLN",
    "status": "placed"
  }
}"#;

        let lines: Vec<String> = json_body.lines().map(|l| l.to_string()).collect();
        let mut textarea = TextArea::new(lines);

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

        Self { component: textarea }
    }
}

impl Component<Msg, NoUserEvent> for MessageDetails {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let _ = match ev {

            // Handle modifiers actions
            Event::Keyboard(KeyEvent { code: Key::Char(c), modifiers: KeyModifiers::CONTROL }) => match c {
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
                match self.component.perform(Cmd::Submit){
                    CmdResult::Submit(state) => {
                        let state = state.unwrap_vec();
                         let lines: Vec<String> = state.into_iter()
                            .map(|sv| sv.unwrap_string()) //TODO: Unwrap_string panics if not a String variant
                            .collect();

                        return Some(Msg::Submit(lines))
                    }
                    _ => return None
                }

            },

            // Handle keys with no modifiers
            Event::Keyboard(KeyEvent { code, modifiers: KeyModifiers::NONE }) => match code {
                Key::PageDown => self.component.perform(Cmd::Scroll(Direction::Down)),
                Key::PageUp => self.component.perform(Cmd::Scroll(Direction::Up)),
                Key::Left => self.component.perform(Cmd::Move(Direction::Left)),
                Key::Down => self.component.perform(Cmd::Move(Direction::Down)),
                Key::Up => self.component.perform(Cmd::Move(Direction::Up)),
                Key::Right => self.component.perform(Cmd::Move(Direction::Right)),
                Key::Backspace => self.component.perform(Cmd::Delete),
                Key::Enter => self.component.perform(Cmd::Custom(TEXTAREA_CMD_NEWLINE)),
                Key::Tab => self.component.perform(Cmd::Type('\t')),
                Key::Esc => return Some(Msg::AppClose),

                // Handle typing
                Key::Char(ch) => {
                    self.component.perform(Cmd::Type(ch));
                    return None;
                }

                _ => CmdResult::None,
            },

            _ => CmdResult::None,
        };

        Some(Msg::ForceRedraw)
    }
}

