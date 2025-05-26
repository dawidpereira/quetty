use server::model::MessageModel;
use tui_realm_stdlib::Table;
use tuirealm::command::{Cmd, CmdResult, Direction};
use tuirealm::event::{Key, KeyEvent, KeyModifiers};
use tuirealm::props::{Alignment, BorderType, Borders, Color, TableBuilder, TextSpan};
use tuirealm::terminal::TerminalAdapter;
use tuirealm::{Component, Event, MockComponent, NoUserEvent, StateValue};

use super::common::{LoadingActivityMsg, MessageActivityMsg, Msg, QueueActivityMsg, QueueType};
use crate::error::{AppError, AppResult};

use crate::app::model::Model;
use crate::config;

#[derive(Debug, Clone)]
pub struct PaginationInfo {
    pub current_page: usize,
    pub total_pages_loaded: usize,
    pub total_messages_loaded: usize,
    pub current_page_size: usize,
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub queue_name: Option<String>,
    pub queue_type: QueueType,
}

#[derive(MockComponent)]
pub struct Messages {
    component: Table,
}

const CMD_RESULT_MESSAGE_SELECTED: &str = "MessageSelected";
const CMD_RESULT_MESSAGE_PREVIEW: &str = "MessagePreview";
const CMD_RESULT_QUEUE_UNSELECTED: &str = "QueueUnSelected";

impl Messages {
    pub fn new(messages: Option<&Vec<MessageModel>>) -> Self {
        Self::new_with_pagination(messages, None)
    }

    pub fn new_with_pagination(
        messages: Option<&Vec<MessageModel>>,
        pagination_info: Option<PaginationInfo>,
    ) -> Self {
        let title = if let Some(info) = pagination_info {
            let queue_display = Self::format_queue_display(&info);
            if info.total_messages_loaded == 0 {
                format!(" {} - No messages available ", queue_display)
            } else {
                format!(
                    " {} - Page {}/{} • {} total • {} on page {} ",
                    queue_display,
                    info.current_page + 1, // Display as 1-based
                    info.total_pages_loaded.max(1),
                    info.total_messages_loaded,
                    info.current_page_size,
                    Self::format_navigation_hints(&info)
                )
            }
        } else {
            " Messages ".to_string()
        };

        let component = {
            Table::default()
                .borders(
                    Borders::default()
                        .modifiers(BorderType::Rounded)
                        .color(Color::Green),
                )
                .background(Color::Reset)
                .foreground(Color::Green)
                .title(&title, Alignment::Center)
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

    fn format_queue_display(info: &PaginationInfo) -> String {
        let queue_name = info.queue_name.as_deref().unwrap_or("Unknown Queue");
        match info.queue_type {
            QueueType::Main => format!("Messages ({}) [Main - d→DLQ]", queue_name),
            QueueType::DeadLetter => format!("Dead Letter Queue ({}) [DLQ - d→Main]", queue_name),
        }
    }

    fn format_navigation_hints(info: &PaginationInfo) -> String {
        let mut hints = Vec::new();

        if info.has_previous_page {
            hints.push("◀[p]");
        }
        if info.has_next_page {
            hints.push("[n]▶");
        }

        if hints.is_empty() {
            "• End of pages".to_string()
        } else {
            format!("• {}", hints.join(" "))
        }
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
            Event::Keyboard(KeyEvent {
                code: Key::Char('n'),
                modifiers: KeyModifiers::NONE,
            }) => return Some(Msg::MessageActivity(MessageActivityMsg::NextPage)),
            Event::Keyboard(KeyEvent {
                code: Key::Char(']'),
                modifiers: KeyModifiers::NONE,
            }) => return Some(Msg::MessageActivity(MessageActivityMsg::NextPage)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('p'),
                modifiers: KeyModifiers::NONE,
            }) => return Some(Msg::MessageActivity(MessageActivityMsg::PreviousPage)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('['),
                modifiers: KeyModifiers::NONE,
            }) => return Some(Msg::MessageActivity(MessageActivityMsg::PreviousPage)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('d'),
                modifiers: KeyModifiers::NONE,
            }) => return Some(Msg::QueueActivity(QueueActivityMsg::ToggleDeadLetterQueue)),
            Event::Keyboard(KeyEvent {
                code: Key::Char('d'),
                modifiers: KeyModifiers::CONTROL,
            }) => {
                let index = match self.state() {
                    tuirealm::State::One(StateValue::Usize(index)) => index,
                    _ => 0,
                };
                return Some(Msg::PopupActivity(
                    super::common::PopupActivityMsg::ShowConfirmation {
                        title: "Send Message to Dead Letter Queue".to_string(),
                        message: "Are you sure you want to send this message to the dead letter queue?\nThis action cannot be undone.".to_string(),
                        on_confirm: Box::new(Msg::MessageActivity(
                            super::common::MessageActivityMsg::SendMessageToDLQ(index)
                        )),
                    }
                ));
            }
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
        log::debug!("Loading messages");
        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        // Show loading indicator
        if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Start(
            "Loading messages...".to_string(),
        ))) {
            log::error!("Failed to send loading start message: {}", e);
        }

        let consumer = self.queue_state.consumer.clone().ok_or_else(|| {
            log::error!("No consumer available");
            AppError::State("No consumer available".to_string())
        })?;

        let tx_to_main_err = tx_to_main.clone();
        taskpool.execute(async move {
            let result = async {
                log::debug!("Acquiring consumer lock");
                let mut consumer = consumer.lock().await;
                log::debug!("Peeking messages");

                let messages = consumer
                    .peek_messages(config::CONFIG.max_messages(), None)
                    .await
                    .map_err(|e| {
                        log::error!("Failed to peek messages: {}", e);
                        AppError::ServiceBus(e.to_string())
                    })?;

                log::info!("Loaded {} messages", messages.len());

                // Stop loading indicator
                if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Stop)) {
                    log::error!("Failed to send loading stop message: {}", e);
                }

                // Send initial messages as new messages loaded
                if !messages.is_empty() {
                    tx_to_main
                        .send(Msg::MessageActivity(MessageActivityMsg::NewMessagesLoaded(
                            messages,
                        )))
                        .map_err(|e| {
                            log::error!("Failed to send new messages loaded message: {}", e);
                            AppError::Component(e.to_string())
                        })?;
                } else {
                    // No messages, but still need to update the view
                    tx_to_main
                        .send(Msg::MessageActivity(MessageActivityMsg::MessagesLoaded(
                            messages,
                        )))
                        .map_err(|e| {
                            log::error!("Failed to send messages loaded message: {}", e);
                            AppError::Component(e.to_string())
                        })?;
                }

                Ok::<(), AppError>(())
            }
            .await;
            if let Err(e) = result {
                log::error!("Error in message loading task: {}", e);

                // Stop loading indicator even if there was an error
                if let Err(err) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Stop)) {
                    log::error!("Failed to send loading stop message: {}", err);
                }

                // Send error message
                let _ = tx_to_main_err.send(Msg::Error(e));
            }
        });

        Ok(())
    }
}
