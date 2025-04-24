use tuirealm::event::{Key, KeyEvent, KeyModifiers};
use tuirealm::props::{Alignment, BorderType, Borders, Color, TableBuilder, TextSpan};
use tuirealm::{Component, Event, MockComponent, NoUserEvent};
use tuirealm::command::{Cmd, CmdResult, Direction};
use tui_realm_stdlib::Table;

use super::common::Msg;

#[derive(MockComponent)]
pub struct Messages{
    component: Table
}

impl Messages {
    pub fn new() -> Self {
        let component = {
            Table::default()
                .borders(
                    Borders::default()
                        .modifiers(BorderType::Rounded)
                        .color(Color::Green),
                )
                .background(Color::Reset)
                .foreground(Color::Green)
                .title(" Messages ", Alignment::Center)
                .scroll(true)
                .highlighted_color(Color::Yellow)
                .highlighted_str(">")
                .rewind(false)
                .step(4)
                .row_height(1)
                .headers(&["Sequence", "Message ID", "Enqueued At ", "Delivery Count"])
                .column_spacing(2)
                .widths(&[10, 30, 25, 16])
                .table(
                    TableBuilder::default()
                        .add_col(TextSpan::from("1"))
                        .add_col(TextSpan::from("9d11fd83-b6d8-4c27-9cc1-ebb31d33bb97"))
                        .add_col(TextSpan::from("2025-04-24 14:00:00"))
                        .add_col(TextSpan::from("0"))
                        .add_row()
                        .add_col(TextSpan::from("2"))
                        .add_col(TextSpan::from("b5ba303c-b125-4191-923d-ef2b3b698a7c"))
                        .add_col(TextSpan::from("2025-04-24 14:05:00"))
                        .add_col(TextSpan::from("0"))
                        .add_row()
                        .add_col(TextSpan::from("3"))
                        .add_col(TextSpan::from("8ac51c0f-2d4e-492d-bc1f-b0e550273cc0"))
                        .add_col(TextSpan::from("2025-04-24 14:10:00"))
                        .add_col(TextSpan::from("0"))
                        .build(),
                )
        };

        Self { component }
    }
}

impl Component<Msg, NoUserEvent> for Messages {
    #[allow(clippy::too_many_lines)]
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let cmd_result = match ev {
            Event::Keyboard(KeyEvent {
                        code: Key::Down,
                        modifiers: KeyModifiers::NONE,
                    }) => self.perform(Cmd::Move(Direction::Down)),
            Event::Keyboard(KeyEvent {
                        code: Key::Char('j'),
                        modifiers: KeyModifiers::NONE,
                    }) => self.perform(Cmd::Move(Direction::Down)),
            Event::Keyboard(KeyEvent {
                        code: Key::Up,
                        modifiers: KeyModifiers::NONE,
                    }) => self.perform(Cmd::Move(Direction::Up)),
            Event::Keyboard(KeyEvent {
                        code: Key::Char('k'),
                        modifiers: KeyModifiers::NONE,
                    }) => self.perform(Cmd::Move(Direction::Up)),
            Event::Keyboard(KeyEvent {
                        code: Key::PageDown,
                        modifiers: KeyModifiers::NONE,
                    }) => self.perform(Cmd::Scroll(Direction::Down)),
            Event::Keyboard(KeyEvent {
                        code: Key::PageUp,
                        modifiers: KeyModifiers::NONE,
                    }) => self.perform(Cmd::Scroll(Direction::Up)),
            Event::Keyboard(KeyEvent {
                code: Key::Esc,
                modifiers: KeyModifiers::NONE,
            }) => return Some(Msg::AppClose),
            _ => CmdResult::None,
        };
        match cmd_result {
            CmdResult::None => None,
            _ => Some(Msg::ForceRedraw),
        }
    }
}

