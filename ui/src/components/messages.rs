use tui_realm_stdlib::Table;
use tuirealm::command::{Cmd, CmdResult, Direction};
use tuirealm::event::{Key, KeyEvent, KeyModifiers};
use tuirealm::props::{Alignment, BorderType, Borders, Color, TableBuilder, TextSpan};
use tuirealm::{Component, Event, MockComponent, NoUserEvent, StateValue};

use crate::models::messages::MessageModel;

use super::common::{MessageActivitMsg, Msg};

#[derive(MockComponent)]
pub struct Messages {
    component: Table,
}

const CMD_RESULT_MESSAGE_SELECTED: &str = "MessageSelected";

impl Messages {
    pub fn new(messages: &Vec<MessageModel>) -> Self {
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
                .table(Self::build_table_from_messages(messages))
        };

        Self { component }
    }

    fn build_table_from_messages(messages: &Vec<MessageModel>) -> Vec<Vec<TextSpan>> {
        let mut builder = TableBuilder::default();

        for msg in messages {
            builder
                .add_col(TextSpan::from(msg.sequence.to_string()))
                .add_col(TextSpan::from(msg.id.to_string()))
                .add_col(TextSpan::from(
                    msg.enqueued_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                ))
                .add_col(TextSpan::from(msg.delivery_count.to_string()))
                .add_row();
        }
        builder.build()
    }
}

impl Component<Msg, NoUserEvent> for Messages {
    #[allow(clippy::too_many_lines)]
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let cmd_result = match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Down,
                modifiers: KeyModifiers::NONE,
            }) => self.component.perform(Cmd::Move(Direction::Down)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('j'),
                modifiers: KeyModifiers::NONE,
            }) => self.component.perform(Cmd::Move(Direction::Down)),
            Event::Keyboard(KeyEvent {
                code: Key::Up,
                modifiers: KeyModifiers::NONE,
            }) => self.component.perform(Cmd::Move(Direction::Up)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('k'),
                modifiers: KeyModifiers::NONE,
            }) => self.component.perform(Cmd::Move(Direction::Up)),
            Event::Keyboard(KeyEvent {
                code: Key::PageDown,
                modifiers: KeyModifiers::NONE,
            }) => self.component.perform(Cmd::Scroll(Direction::Down)),
            Event::Keyboard(KeyEvent {
                code: Key::PageUp,
                modifiers: KeyModifiers::NONE,
            }) => self.perform(Cmd::Scroll(Direction::Up)),
            Event::Keyboard(KeyEvent {
                code: Key::Enter,
                modifiers: KeyModifiers::NONE,
            }) => CmdResult::Custom(CMD_RESULT_MESSAGE_SELECTED, self.state()),
            Event::Keyboard(KeyEvent {
                code: Key::Esc,
                modifiers: KeyModifiers::NONE,
            }) => return Some(Msg::AppClose),
            _ => CmdResult::None,
        };
        match cmd_result {
            CmdResult::Changed(state) => match state.unwrap_one() {
                StateValue::Usize(index) => Some(Msg::MessageActivity(
                    MessageActivitMsg::RefreshMessageDetails(index),
                )),
                _ => {
                    println!("Incorrect state in message table");
                    None
                }
            },
            CmdResult::Custom(CMD_RESULT_MESSAGE_SELECTED, state) => match state.unwrap_one() {
                StateValue::Usize(index) => {
                    Some(Msg::MessageActivity(MessageActivitMsg::EditMessage(index)))
                }
                _ => {
                    println!("Incorrect state in message table");
                    None
                }
            },
            CmdResult::None => None,
            _ => Some(Msg::ForceRedraw),
        }
    }
}
