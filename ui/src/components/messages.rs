use server::model::MessageModel;
use tui_realm_stdlib::Table;
use tuirealm::command::{Cmd, CmdResult, Direction};
use tuirealm::event::{Key, KeyEvent, KeyModifiers};
use tuirealm::props::{Alignment, BorderType, Borders, Color, TableBuilder, TextSpan};
use tuirealm::terminal::TerminalAdapter;
use tuirealm::{Component, Event, MockComponent, NoUserEvent, StateValue};

use super::common::{MessageActivityMsg, Msg, QueueActivityMsg};
use crate::error::{AppError, AppResult};

use crate::app::model::Model;
use crate::config;

#[derive(MockComponent)]
pub struct Messages {
    component: Table,
}

const CMD_RESULT_MESSAGE_SELECTED: &str = "MessageSelected";
const CMD_RESULT_MESSAGE_PREVIEW: &str = "MessagePreview";
const CMD_RESULT_QUEUE_UNSELECTED: &str = "QueueUnSelected";

impl Messages {
    pub fn new(messages: Option<&Vec<MessageModel>>) -> Self {
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

    fn build_table_from_messages(messages: Option<&Vec<MessageModel>>) -> Vec<Vec<TextSpan>> {
        if let Some(messages) = messages {
            let mut builder = TableBuilder::default();

            for msg in messages {
                builder
                    .add_col(TextSpan::from(msg.sequence.to_string()))
                    .add_col(TextSpan::from(msg.id.to_string()))
                    .add_col(TextSpan::from(msg.enqueued_at.to_string()))
                    .add_col(TextSpan::from(msg.delivery_count.to_string()))
                    .add_row();
            }
            return builder.build();
        }
        Vec::new()
    }
}

impl Component<Msg, NoUserEvent> for Messages {
    #[allow(clippy::too_many_lines)]
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let cmd_result = match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Down,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.component.perform(Cmd::Move(Direction::Down));
                CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, self.state())
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('j'),
                modifiers: KeyModifiers::NONE,
            }) => {
                self.component.perform(Cmd::Move(Direction::Down));
                CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, self.state())
            }
            Event::Keyboard(KeyEvent {
                code: Key::Up,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.component.perform(Cmd::Move(Direction::Up));
                CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, self.state())
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('k'),
                modifiers: KeyModifiers::NONE,
            }) => {
                self.component.perform(Cmd::Move(Direction::Up));
                CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, self.state())
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageDown,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.component.perform(Cmd::Scroll(Direction::Down));
                CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, self.state())
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageUp,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.component.perform(Cmd::Scroll(Direction::Up));
                CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, self.state())
            }
            Event::Keyboard(KeyEvent {
                code: Key::Enter,
                modifiers: KeyModifiers::NONE,
            }) => CmdResult::Custom(CMD_RESULT_MESSAGE_SELECTED, self.state()),
            Event::Keyboard(KeyEvent {
                code: Key::Esc,
                modifiers: KeyModifiers::NONE,
            }) => CmdResult::Custom(CMD_RESULT_QUEUE_UNSELECTED, self.state()),
            _ => CmdResult::None,
        };
        match cmd_result {
            CmdResult::Custom(CMD_RESULT_MESSAGE_SELECTED, state) => match state {
                tuirealm::State::One(StateValue::Usize(index)) => {
                    Some(Msg::MessageActivity(MessageActivityMsg::EditMessage(index)))
                }
                _ => None,
            },
            CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, state) => match state {
                tuirealm::State::One(StateValue::Usize(index)) => Some(Msg::MessageActivity(
                    MessageActivityMsg::PreviewMessageDetails(index),
                )),
                _ => None,
            },
            CmdResult::Custom(CMD_RESULT_QUEUE_UNSELECTED, _) => {
                Some(Msg::QueueActivity(QueueActivityMsg::QueueUnselected))
            }
            _ => Some(Msg::ForceRedraw),
        }
    }
}

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn load_messages(&mut self) -> AppResult<()> {
        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        let consumer = self
            .consumer
            .clone()
            .ok_or_else(|| AppError::State("No consumer available".to_string()))?;

        let tx_to_main_err = tx_to_main.clone();
        taskpool.execute(async move {
            let result = async {
                let mut consumer = consumer.lock().await;
                let messages = consumer
                    .peek_messages(config::CONFIG.max_messages(), None)
                    .await
                    .map_err(|e| AppError::ServiceBus(e.to_string()))?;

                tx_to_main
                    .send(Msg::MessageActivity(MessageActivityMsg::MessagesLoaded(
                        messages,
                    )))
                    .map_err(|e| AppError::Component(e.to_string()))?;

                Ok::<(), AppError>(())
            }
            .await;
            if let Err(e) = result {
                let _ = tx_to_main_err.send(Msg::Error(e));
            }
        });

        Ok(())
    }
}
