use crate::components::common::{ConfigActivityMsg, Msg};
use crate::components::state::ComponentState;
use crate::error::AppResult;
use crate::theme::ThemeManager;
use tuirealm::command::{Cmd, CmdResult};
use tuirealm::event::{Key, KeyEvent, KeyModifiers};
use tuirealm::props::Alignment;
use tuirealm::ratatui::layout::{Constraint, Direction, Layout, Rect};
use tuirealm::ratatui::style::Modifier;
use tuirealm::ratatui::text::{Line, Span, Text};
use tuirealm::ratatui::widgets::{Block, Borders, Clear, Paragraph};
use tuirealm::{
    AttrValue, Attribute, Component, Event, Frame, MockComponent, NoUserEvent, State, StateValue,
};

const CMD_RESULT_SUBMIT: &str = "Submit";
const CMD_RESULT_CANCEL: &str = "Cancel";

pub struct PasswordPopup {
    password: String,
    error_message: Option<String>,
}

impl Default for PasswordPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl PasswordPopup {
    pub fn new() -> Self {
        Self {
            password: String::new(),
            error_message: None,
        }
    }

    pub fn with_error(error_message: String) -> Self {
        Self {
            password: String::new(),
            error_message: Some(error_message),
        }
    }

    fn get_password(&self) -> String {
        self.password.clone()
    }
}

impl ComponentState for PasswordPopup {
    fn mount(&mut self) -> AppResult<()> {
        log::debug!("Mounting PasswordPopup component");
        Ok(())
    }
}

impl MockComponent for PasswordPopup {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Clear the area
        frame.render_widget(Clear, area);

        // Use the provided area directly (it's already centered by PopupLayout in the view function)
        let popup_area = area;

        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),                                                // Title
                Constraint::Length(3), // Instructions
                Constraint::Length(3), // Password field
                Constraint::Length(if self.error_message.is_some() { 3 } else { 0 }), // Error message
                Constraint::Min(0),                                                   // Actions
            ])
            .split(popup_area);

        // Title
        let title = Paragraph::new("Enter Master Password")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(tuirealm::ratatui::widgets::BorderType::Rounded)
                    .border_style(
                        tuirealm::ratatui::style::Style::default()
                            .fg(ThemeManager::primary_accent()),
                    ),
            )
            .alignment(Alignment::Center)
            .style(
                tuirealm::ratatui::style::Style::default()
                    .fg(ThemeManager::message_delivery_count())
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(title, chunks[0]);

        // Instructions
        let instructions =
            Paragraph::new("Enter your master password to decrypt the connection string")
                .block(
                    Block::default()
                        .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                        .border_type(tuirealm::ratatui::widgets::BorderType::Rounded)
                        .border_style(
                            tuirealm::ratatui::style::Style::default()
                                .fg(ThemeManager::primary_accent()),
                        ),
                )
                .alignment(Alignment::Center)
                .style(tuirealm::ratatui::style::Style::default().fg(ThemeManager::text_primary()));
        frame.render_widget(instructions, chunks[1]);

        // Password field
        let password_display = "*".repeat(self.password.len().min(30));
        let password_text = if password_display.is_empty() {
            "<empty>".to_string()
        } else {
            format!("{password_display}_")
        };

        let password_field = Paragraph::new(password_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(tuirealm::ratatui::widgets::BorderType::Rounded)
                    .border_style(
                        tuirealm::ratatui::style::Style::default()
                            .fg(ThemeManager::primary_accent()),
                    )
                    .title("Master Password")
                    .title_style(
                        tuirealm::ratatui::style::Style::default().fg(ThemeManager::text_primary()),
                    ),
            )
            .alignment(Alignment::Center)
            .style(tuirealm::ratatui::style::Style::default().fg(ThemeManager::text_primary()));
        frame.render_widget(password_field, chunks[2]);

        // Error message (if any)
        if let Some(ref error) = self.error_message {
            let error_field = Paragraph::new(error.clone())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(tuirealm::ratatui::widgets::BorderType::Rounded)
                        .border_style(
                            tuirealm::ratatui::style::Style::default()
                                .fg(ThemeManager::status_error()),
                        )
                        .title("Error")
                        .title_style(
                            tuirealm::ratatui::style::Style::default()
                                .fg(ThemeManager::status_error()),
                        ),
                )
                .alignment(Alignment::Center)
                .style(tuirealm::ratatui::style::Style::default().fg(ThemeManager::status_error()));
            frame.render_widget(error_field, chunks[3]);
        }

        // Actions
        let actions_text = [
            ("[Enter]".to_string(), true),
            (" submit ".to_string(), false),
            ("[Esc]".to_string(), true),
            (" cancel ".to_string(), false),
            ("[C]".to_string(), true),
            (" open config".to_string(), false),
        ];

        let mut spans: Vec<Span> = Vec::new();
        for (i, (text, highlight)) in actions_text.iter().enumerate() {
            // Add separator before each pair (except the first one)
            if i > 0 && i % 2 == 0 {
                spans.push(Span::styled(
                    " │ ",
                    tuirealm::ratatui::style::Style::default().fg(ThemeManager::text_muted()),
                ));
            }

            if *highlight {
                spans.push(Span::styled(
                    text.clone(),
                    tuirealm::ratatui::style::Style::default().fg(ThemeManager::shortcut_key()),
                ));
            } else {
                spans.push(Span::styled(
                    text.clone(),
                    tuirealm::ratatui::style::Style::default()
                        .fg(ThemeManager::shortcut_description()),
                ));
            }
        }

        let actions_chunk_index = if self.error_message.is_some() { 4 } else { 3 };
        let actions = Paragraph::new(Text::from(Line::from(spans)))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(tuirealm::ratatui::widgets::BorderType::Rounded)
                    .border_style(
                        tuirealm::ratatui::style::Style::default()
                            .fg(ThemeManager::primary_accent()),
                    )
                    .title("Actions")
                    .title_style(
                        tuirealm::ratatui::style::Style::default().fg(ThemeManager::text_primary()),
                    ),
            )
            .alignment(Alignment::Center);

        frame.render_widget(actions, chunks[actions_chunk_index]);
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        match attr {
            Attribute::Content => Some(AttrValue::String(self.get_password())),
            _ => None,
        }
    }

    fn attr(&mut self, _attr: Attribute, _value: AttrValue) {}

    fn state(&self) -> State {
        State::None
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        match cmd {
            Cmd::Submit => {
                if self.password.trim().is_empty() {
                    CmdResult::None
                } else {
                    CmdResult::Submit(State::One(StateValue::String(
                        CMD_RESULT_SUBMIT.to_string(),
                    )))
                }
            }
            Cmd::Cancel => CmdResult::Submit(State::One(StateValue::String(
                CMD_RESULT_CANCEL.to_string(),
            ))),
            _ => CmdResult::None,
        }
    }
}

impl Component<Msg, NoUserEvent> for PasswordPopup {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        log::debug!("PasswordPopup received event: {ev:?}");
        match ev {
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => {
                Some(Msg::ConfigActivity(ConfigActivityMsg::Cancel))
            }
            Event::Keyboard(KeyEvent {
                code: Key::Enter, ..
            }) => {
                if self.password.trim().is_empty() {
                    None // Don't submit empty password
                } else {
                    // Create config update data with just the master password
                    // If there's pending config data from the config screen, preserve those values
                    let config_data = crate::components::common::ConfigUpdateData {
                        auth_method: crate::utils::auth::AUTH_METHOD_CONNECTION_STRING.to_string(),
                        tenant_id: None,
                        client_id: None,
                        client_secret: None,
                        subscription_id: None,
                        resource_group: None,
                        namespace: None,
                        connection_string: None, // Don't update connection string
                        master_password: Some(self.password.clone()),
                        queue_name: None, // Will be updated in the message handler to preserve from config screen
                    };
                    Some(Msg::ConfigActivity(ConfigActivityMsg::ConfirmAndProceed(
                        config_data,
                    )))
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('c'),
                modifiers: KeyModifiers::NONE,
                ..
            })
            | Event::Keyboard(KeyEvent {
                code: Key::Char('C'),
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                // Open full config screen
                Some(Msg::ToggleConfigScreen)
            }
            Event::Keyboard(KeyEvent {
                code: Key::Backspace,
                ..
            }) => {
                self.password.pop();
                Some(Msg::ForceRedraw)
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c), ..
            }) => {
                if self.password.len() < 512 {
                    self.password.push(c);
                    Some(Msg::ForceRedraw)
                } else {
                    Some(Msg::ForceRedraw)
                }
            }
            _ => None,
        }
    }
}
