use crate::components::base_popup::PopupBuilder;
use crate::components::common::{AuthActivityMsg, Msg};
use crate::components::state::ComponentState;
use tuirealm::{
    Component, Event, MockComponent, NoUserEvent,
    command::{Cmd, CmdResult},
    event::{Key, KeyEvent},
    ratatui::{Frame, layout::Rect},
};

#[derive(Debug, Clone, PartialEq)]
pub enum AuthPopupState {
    ShowingDeviceCode {
        user_code: String,
        verification_url: String,
        message: String,
        expires_at: Option<std::time::Instant>,
    },
    Authenticating,
    Success,
    Failed(String),
}

pub struct AuthPopup {
    state: AuthPopupState,
    is_mounted: bool,
}

impl AuthPopup {
    pub fn new(state: AuthPopupState) -> Self {
        Self {
            state,
            is_mounted: false,
        }
    }
}

impl MockComponent for AuthPopup {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        match &self.state {
            AuthPopupState::ShowingDeviceCode {
                user_code,
                verification_url,
                message,
                expires_at,
            } => {
                let mut builder = PopupBuilder::new("Azure AD Authentication");

                // Add the message with proper line breaks
                for line in message.lines() {
                    builder = builder.add_text(line.to_string());
                }

                builder = builder.add_empty_line();

                // Add user code with bold and colored styling
                builder = builder.add_line(vec![
                    tuirealm::ratatui::text::Span::raw("User Code: "),
                    tuirealm::ratatui::text::Span::styled(
                        user_code.clone(),
                        tuirealm::ratatui::style::Style::default()
                            .fg(crate::theme::ThemeManager::primary_accent())
                            .add_modifier(tuirealm::ratatui::style::Modifier::BOLD),
                    ),
                ]);

                builder = builder.add_empty_line();

                // Add verification URL with muted color
                builder = builder.add_line(vec![
                    tuirealm::ratatui::text::Span::raw("Verification URL: "),
                    tuirealm::ratatui::text::Span::styled(
                        verification_url.clone(),
                        tuirealm::ratatui::style::Style::default()
                            .fg(crate::theme::ThemeManager::text_muted())
                            .add_modifier(tuirealm::ratatui::style::Modifier::UNDERLINED),
                    ),
                ]);

                // Add countdown timer if available
                if let Some(expires_at) = expires_at {
                    let now = std::time::Instant::now();
                    if now < *expires_at {
                        let remaining = expires_at.duration_since(now);
                        let minutes = remaining.as_secs() / 60;
                        let seconds = remaining.as_secs() % 60;
                        builder = builder.add_empty_line();

                        // Color-code the timer based on remaining time
                        let timer_color = if remaining.as_secs() < 60 {
                            crate::theme::ThemeManager::status_error()
                        } else if remaining.as_secs() < 300 {
                            crate::theme::ThemeManager::status_warning()
                        } else {
                            crate::theme::ThemeManager::status_success()
                        };

                        builder = builder.add_line(vec![
                            tuirealm::ratatui::text::Span::raw("Time remaining: "),
                            tuirealm::ratatui::text::Span::styled(
                                format!("{:02}:{:02}", minutes, seconds),
                                tuirealm::ratatui::style::Style::default()
                                    .fg(timer_color)
                                    .add_modifier(tuirealm::ratatui::style::Modifier::BOLD),
                            ),
                        ]);
                    } else {
                        builder = builder
                            .add_empty_line()
                            .add_error_text("Authentication timeout - please restart");
                    }
                }

                builder
                    .add_empty_line()
                    .add_empty_line()
                    .add_line(vec![
                        tuirealm::ratatui::text::Span::styled(
                            "[Y]",
                            tuirealm::ratatui::style::Style::default()
                                .fg(crate::theme::ThemeManager::status_success())
                                .add_modifier(tuirealm::ratatui::style::Modifier::BOLD),
                        ),
                        tuirealm::ratatui::text::Span::raw(" Copy code   "),
                        tuirealm::ratatui::text::Span::styled(
                            "[O]",
                            tuirealm::ratatui::style::Style::default()
                                .fg(crate::theme::ThemeManager::primary_accent())
                                .add_modifier(tuirealm::ratatui::style::Modifier::BOLD),
                        ),
                        tuirealm::ratatui::text::Span::raw(" Open URL   "),
                        tuirealm::ratatui::text::Span::styled(
                            "[ESC]",
                            tuirealm::ratatui::style::Style::default()
                                .fg(crate::theme::ThemeManager::status_error())
                                .add_modifier(tuirealm::ratatui::style::Modifier::BOLD),
                        ),
                        tuirealm::ratatui::text::Span::raw(" Cancel"),
                    ])
                    .render(frame, area);
            }
            AuthPopupState::Authenticating => {
                PopupBuilder::new("🔐 Azure AD Authentication")
                    .add_text("Waiting for authentication...")
                    .add_empty_line()
                    .add_text("Please complete the authentication in your browser")
                    .render(frame, area);
            }
            AuthPopupState::Success => {
                PopupBuilder::success("✅ Authentication Successful")
                    .add_text("You have been successfully authenticated")
                    .add_empty_line()
                    .with_instructions("Press any key to continue")
                    .render(frame, area);
            }
            AuthPopupState::Failed(error) => {
                PopupBuilder::error("❌ Authentication Failed")
                    .add_text(error)
                    .add_empty_line()
                    .with_instructions("Press 'r' to retry | Press 'Esc' to cancel")
                    .render(frame, area);
            }
        }
    }

    fn query(&self, _attr: tuirealm::Attribute) -> Option<tuirealm::AttrValue> {
        None
    }

    fn attr(&mut self, _attr: tuirealm::Attribute, _value: tuirealm::AttrValue) {
        // No attributes supported
    }

    fn state(&self) -> tuirealm::State {
        tuirealm::State::None
    }

    fn perform(&mut self, _cmd: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for AuthPopup {
    fn on(&mut self, event: Event<NoUserEvent>) -> Option<Msg> {
        match event {
            Event::Keyboard(KeyEvent { code: key, .. }) => match &self.state {
                AuthPopupState::ShowingDeviceCode { .. } => match key {
                    Key::Char('y') => Some(Msg::AuthActivity(AuthActivityMsg::CopyDeviceCode)),
                    Key::Char('o') => Some(Msg::AuthActivity(AuthActivityMsg::OpenVerificationUrl)),
                    Key::Esc => Some(Msg::AuthActivity(AuthActivityMsg::CancelAuthentication)),
                    _ => None,
                },
                AuthPopupState::Failed(_) => match key {
                    Key::Char('r') => Some(Msg::AuthActivity(AuthActivityMsg::Login)),
                    Key::Esc => Some(Msg::AuthActivity(AuthActivityMsg::CancelAuthentication)),
                    _ => None,
                },
                AuthPopupState::Success => {
                    Some(Msg::AuthActivity(AuthActivityMsg::CancelAuthentication))
                }
                AuthPopupState::Authenticating => None,
            },
            _ => None,
        }
    }
}

impl ComponentState for AuthPopup {
    fn mount(&mut self) -> crate::error::AppResult<()> {
        self.is_mounted = true;
        Ok(())
    }
}

impl Default for AuthPopupState {
    fn default() -> Self {
        AuthPopupState::Authenticating
    }
}
