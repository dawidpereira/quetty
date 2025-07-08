use crate::components::common::{ConfigActivityMsg, ConfigUpdateData, Msg};
use crate::components::state::ComponentState;
use crate::config::{self, AppConfig};
use crate::error::AppResult;
use crate::theme::ThemeManager;
use crate::utils::auth::{AUTH_METHOD_CONNECTION_STRING, AUTH_METHOD_DEVICE_CODE};
use crate::utils::connection_string::ConnectionStringParser;
use tuirealm::command::{Cmd, CmdResult};
use tuirealm::event::{Key, KeyEvent, KeyModifiers};
use tuirealm::props::{Alignment, Style};
use tuirealm::ratatui::layout::{Constraint, Direction, Layout, Rect};
use tuirealm::ratatui::style::Modifier;
use tuirealm::ratatui::text::{Line, Span, Text};
use tuirealm::ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use tuirealm::{
    AttrValue, Attribute, Component, Event, Frame, MockComponent, NoUserEvent, State, StateValue,
};

const CMD_RESULT_SAVE: &str = "Save";
const CMD_RESULT_CANCEL: &str = "Cancel";

#[derive(Debug, Clone)]
pub struct ConfigFormData {
    pub auth_method: String,
    pub tenant_id: String,
    pub client_id: String,
    pub client_secret: String,
    pub subscription_id: String,
    pub resource_group: String,
    pub namespace: String,
    pub connection_string: String,
    pub queue_name: String,
}

impl Default for ConfigFormData {
    fn default() -> Self {
        Self {
            auth_method: AUTH_METHOD_CONNECTION_STRING.to_string(),
            tenant_id: String::new(),
            client_id: String::new(),
            client_secret: String::new(),
            subscription_id: String::new(),
            resource_group: String::new(),
            namespace: String::new(),
            connection_string: String::new(),
            queue_name: String::new(),
        }
    }
}

pub struct ConfigScreen {
    form_data: ConfigFormData,
    selected_field: usize,
    validation_errors: Vec<String>,
    editing_mode: bool,
    current_input: String,
}

impl Default for ConfigScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigScreen {
    pub fn new() -> Self {
        let config = config::get_config_or_panic();
        let form_data = Self::load_current_config(config);

        Self {
            form_data,
            selected_field: 0,
            validation_errors: Vec::new(),
            editing_mode: false,
            current_input: String::new(),
        }
    }

    fn load_current_config(config: &AppConfig) -> ConfigFormData {
        ConfigFormData {
            auth_method: config.azure_ad().auth_method.clone(),
            tenant_id: std::env::var("AZURE_AD__TENANT_ID").unwrap_or_default(),
            client_id: std::env::var("AZURE_AD__CLIENT_ID").unwrap_or_default(),
            client_secret: std::env::var("AZURE_AD__CLIENT_SECRET").unwrap_or_default(),
            subscription_id: std::env::var("AZURE_AD__SUBSCRIPTION_ID").unwrap_or_default(),
            resource_group: std::env::var("AZURE_AD__RESOURCE_GROUP").unwrap_or_default(),
            namespace: std::env::var("AZURE_AD__NAMESPACE").unwrap_or_default(),
            connection_string: std::env::var("SERVICEBUS__CONNECTION_STRING").unwrap_or_default(),
            queue_name: std::env::var("SERVICEBUS__QUEUE_NAME").unwrap_or_default(),
        }
    }

    fn get_auth_methods() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                AUTH_METHOD_CONNECTION_STRING,
                "Service Bus Connection String",
            ),
            (AUTH_METHOD_DEVICE_CODE, "Azure AD Device Code Flow"),
        ]
    }

    fn get_fields_for_auth_method(&self) -> Vec<(&'static str, &'static str, bool)> {
        match self.form_data.auth_method.as_str() {
            AUTH_METHOD_CONNECTION_STRING => vec![
                ("connection_string", "Connection String", true),
                ("queue_name", "Queue Name", false),
            ],
            AUTH_METHOD_DEVICE_CODE => vec![
                ("tenant_id", "Tenant ID", true),
                ("client_id", "Client ID", true),
                ("subscription_id", "Subscription ID", false),
                ("resource_group", "Resource Group", false),
                ("namespace", "Namespace", false),
            ],
            "managed_identity" => vec![
                ("subscription_id", "Subscription ID", false),
                ("resource_group", "Resource Group", false),
                ("namespace", "Namespace", false),
            ],
            _ => vec![],
        }
    }

    fn get_field_value(&self, field: &str) -> &str {
        match field {
            "auth_method" => &self.form_data.auth_method,
            "tenant_id" => &self.form_data.tenant_id,
            "client_id" => &self.form_data.client_id,
            "client_secret" => &self.form_data.client_secret,
            "subscription_id" => &self.form_data.subscription_id,
            "resource_group" => &self.form_data.resource_group,
            "namespace" => &self.form_data.namespace,
            "connection_string" => &self.form_data.connection_string,
            "queue_name" => &self.form_data.queue_name,
            _ => "",
        }
    }

    fn set_field_value(&mut self, field: &str, value: String) {
        match field {
            "auth_method" => self.form_data.auth_method = value,
            "tenant_id" => self.form_data.tenant_id = value,
            "client_id" => self.form_data.client_id = value,
            "client_secret" => self.form_data.client_secret = value,
            "subscription_id" => self.form_data.subscription_id = value,
            "resource_group" => self.form_data.resource_group = value,
            "namespace" => self.form_data.namespace = value,
            "connection_string" => self.form_data.connection_string = value,
            "queue_name" => self.form_data.queue_name = value,
            _ => {}
        }
    }

    fn validate_config(&self) -> Vec<String> {
        let mut errors = Vec::new();

        match self.form_data.auth_method.as_str() {
            AUTH_METHOD_CONNECTION_STRING => {
                if self.form_data.connection_string.trim().is_empty() {
                    errors.push(
                        "Connection string is required for connection_string auth method"
                            .to_string(),
                    );
                } else {
                    // Validate connection string format
                    if let Err(validation_error) =
                        ConnectionStringParser::validate_connection_string(
                            &self.form_data.connection_string,
                        )
                    {
                        errors.push(format!("Invalid connection string: {validation_error}"));
                    }
                }
            }
            AUTH_METHOD_DEVICE_CODE => {
                if self.form_data.tenant_id.trim().is_empty() {
                    errors.push("Tenant ID is required for device code flow".to_string());
                }
                if self.form_data.client_id.trim().is_empty() {
                    errors.push("Client ID is required for device code flow".to_string());
                }
            }
            "managed_identity" => {
                // No required fields for managed identity
            }
            _ => {
                errors.push("Invalid authentication method selected".to_string());
            }
        }

        errors
    }

    fn render_authentication_tab(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // All fields including auth method
                Constraint::Length(3), // Actions
            ])
            .split(area);

        // Create a combined list with auth method + fields
        let auth_methods = Self::get_auth_methods();
        let fields = self.get_fields_for_auth_method();
        let mut all_items = Vec::new();

        // Add auth method selection as first item
        all_items.push(("auth_method", "Authentication Method", true));

        // Add other fields
        all_items.extend(fields);

        let field_items: Vec<ListItem> = all_items
            .iter()
            .enumerate()
            .map(|(i, (field, label, required))| {
                let is_selected = i == self.selected_field;
                let req_indicator = if *required { "*" } else { " " };

                let display_value = if *field == "auth_method" {
                    // Show current auth method selection with indicator
                    let method_desc = auth_methods
                        .iter()
                        .find(|(method, _)| *method == self.form_data.auth_method)
                        .map(|(_, desc)| desc.to_string())
                        .unwrap_or_else(|| self.form_data.auth_method.clone());
                    format!("{method_desc} (Enter to cycle)")
                } else {
                    let value = self.get_field_value(field);

                    // Show editing state if this field is being edited
                    if self.editing_mode && is_selected {
                        if *field == "client_secret" || *field == "connection_string" {
                            format!("{}_", "*".repeat(self.current_input.len()))
                        } else {
                            format!("{}_", self.current_input)
                        }
                    } else if *field == "client_secret" || *field == "connection_string" {
                        if value.is_empty() {
                            "<empty>".to_string()
                        } else {
                            "*".repeat(value.len().min(20))
                        }
                    } else if value.is_empty() {
                        "<empty>".to_string()
                    } else {
                        value.to_string()
                    }
                };

                let (label_style, value_style) = if is_selected {
                    (
                        Style::default()
                            .fg(ThemeManager::primary_accent()) // Bright cyan for selected field name
                            .add_modifier(Modifier::BOLD),
                        Style::default().fg(ThemeManager::text_primary()), // Value stays white
                    )
                } else if *required && self.get_field_value(field).trim().is_empty() {
                    (
                        Style::default().fg(ThemeManager::status_error()), // Red for error field name
                        Style::default().fg(ThemeManager::text_primary()), // Value stays white even on error
                    )
                } else {
                    (
                        Style::default().fg(ThemeManager::message_id()), // Light blue for normal field names
                        Style::default().fg(ThemeManager::text_primary()), // Value always white
                    )
                };

                ListItem::new(Line::from(vec![
                    Span::styled(format!("{req_indicator}{label}: "), label_style),
                    Span::styled(display_value, value_style),
                ]))
            })
            .collect();

        let fields_list = List::new(field_items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(tuirealm::ratatui::widgets::BorderType::Rounded)
                .border_style(
                    tuirealm::ratatui::style::Style::default().fg(ThemeManager::primary_accent()),
                )
                .title("Configuration Manager")
                .title_style(
                    tuirealm::ratatui::style::Style::default().fg(ThemeManager::text_primary()),
                ),
        );

        f.render_widget(fields_list, chunks[0]);

        // Actions - styled exactly like help bar
        let actions_text = if self.editing_mode {
            vec![
                ("Type to edit".to_string(), false),
                (" ".to_string(), false),
                ("[Enter]".to_string(), true),
                (" confirm ".to_string(), false),
                ("[Esc]".to_string(), true),
                (" cancel editing".to_string(), false),
            ]
        } else {
            vec![
                ("[Enter]".to_string(), true),
                (" edit field/cycle auth ".to_string(), false),
                ("[↑↓]".to_string(), true),
                (" navigate ".to_string(), false),
                ("[Esc]".to_string(), true),
                (" cancel ".to_string(), false),
                ("[s]".to_string(), true),
                (" save ".to_string(), false),
                ("[Ctrl+S]".to_string(), true),
                (" save & proceed".to_string(), false),
            ]
        };

        let mut spans: Vec<Span> = Vec::new();

        // Add each shortcut pair with separators (exactly like help bar)
        for (i, (text, highlight)) in actions_text.iter().enumerate() {
            // Add separator before each pair (except the first one)
            if i > 0 && i % 2 == 0 {
                spans.push(Span::styled(
                    " │ ",
                    tuirealm::ratatui::style::Style::default().fg(ThemeManager::text_muted()),
                ));
            }

            // Add the shortcut text
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

        f.render_widget(actions, chunks[1]);
    }

    fn to_config_update_data(&self) -> ConfigUpdateData {
        ConfigUpdateData {
            auth_method: self.form_data.auth_method.clone(),
            tenant_id: if self.form_data.tenant_id.trim().is_empty() {
                None
            } else {
                Some(self.form_data.tenant_id.clone())
            },
            client_id: if self.form_data.client_id.trim().is_empty() {
                None
            } else {
                Some(self.form_data.client_id.clone())
            },
            client_secret: if self.form_data.client_secret.trim().is_empty() {
                None
            } else {
                Some(self.form_data.client_secret.clone())
            },
            subscription_id: if self.form_data.subscription_id.trim().is_empty() {
                None
            } else {
                Some(self.form_data.subscription_id.clone())
            },
            resource_group: if self.form_data.resource_group.trim().is_empty() {
                None
            } else {
                Some(self.form_data.resource_group.clone())
            },
            namespace: if self.form_data.namespace.trim().is_empty() {
                None
            } else {
                Some(self.form_data.namespace.clone())
            },
            connection_string: if self.form_data.connection_string.trim().is_empty() {
                None
            } else {
                Some(self.form_data.connection_string.clone())
            },
            queue_name: if self.form_data.queue_name.trim().is_empty() {
                None
            } else {
                Some(self.form_data.queue_name.clone())
            },
        }
    }
}

impl ComponentState for ConfigScreen {
    fn mount(&mut self) -> AppResult<()> {
        log::debug!("Mounting ConfigScreen component");
        Ok(())
    }
}

impl MockComponent for ConfigScreen {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Clear the area
        frame.render_widget(Clear, area);

        // Create popup area (centered, 80% width, 80% height)
        let popup_width = area.width * 80 / 100;
        let popup_height = area.height * 80 / 100;
        let popup_x = (area.width - popup_width) / 2;
        let popup_y = (area.height - popup_height) / 2;
        let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Content
            ])
            .split(popup_area);

        // Title
        let title = Paragraph::new("Quetty Configuration")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(tuirealm::ratatui::widgets::BorderType::Rounded)
                    .border_style(
                        tuirealm::ratatui::style::Style::default()
                            .fg(ThemeManager::primary_accent()),
                    ), // Same cyan as main config
            )
            .alignment(Alignment::Center)
            .style(
                tuirealm::ratatui::style::Style::default()
                    .fg(ThemeManager::message_delivery_count()) // Magenta - closest to #EA6F92
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(title, chunks[0]);

        // Just render authentication directly - no tabs needed
        self.render_authentication_tab(frame, chunks[1]);
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        match attr {
            Attribute::Content => Some(AttrValue::String("ConfigScreen".to_string())),
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
                self.validation_errors = self.validate_config();
                if self.validation_errors.is_empty() {
                    CmdResult::Submit(State::One(StateValue::String(CMD_RESULT_SAVE.to_string())))
                } else {
                    CmdResult::None
                }
            }
            Cmd::Cancel => CmdResult::Submit(State::One(StateValue::String(
                CMD_RESULT_CANCEL.to_string(),
            ))),
            Cmd::Move(tuirealm::command::Direction::Up) => {
                if self.selected_field > 0 {
                    self.selected_field -= 1;
                }
                CmdResult::Changed(self.state())
            }
            Cmd::Move(tuirealm::command::Direction::Down) => {
                // Count total items (auth method + fields)
                let fields = self.get_fields_for_auth_method();
                let total_items = 1 + fields.len(); // 1 for auth method + fields
                if self.selected_field < total_items.saturating_sub(1) {
                    self.selected_field += 1;
                }
                CmdResult::Changed(self.state())
            }
            Cmd::Move(tuirealm::command::Direction::Left) => {
                // No horizontal navigation needed anymore
                CmdResult::None
            }
            Cmd::Move(tuirealm::command::Direction::Right) => {
                // No horizontal navigation needed anymore
                CmdResult::None
            }
            _ => CmdResult::None,
        }
    }
}

impl Component<Msg, NoUserEvent> for ConfigScreen {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        log::debug!("ConfigScreen received event: {ev:?}");
        match ev {
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => {
                if self.editing_mode {
                    // Cancel editing
                    self.editing_mode = false;
                    self.current_input.clear();
                    // Update global editing state to re-enable global key handling
                    Some(Msg::SetEditingMode(false))
                } else {
                    Some(Msg::ConfigActivity(ConfigActivityMsg::Cancel))
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('s'),
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                if !self.editing_mode {
                    self.validation_errors = self.validate_config();
                    if self.validation_errors.is_empty() {
                        Some(Msg::ConfigActivity(ConfigActivityMsg::Save(
                            self.to_config_update_data(),
                        )))
                    } else {
                        None
                    }
                } else {
                    // In editing mode, let the general character handler process 's'
                    if self.current_input.len() < 512 {
                        self.current_input.push('s');
                        Some(Msg::ForceRedraw)
                    } else {
                        Some(Msg::ForceRedraw)
                    }
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('s'),
                modifiers: KeyModifiers::CONTROL,
                ..
            }) => {
                self.validation_errors = self.validate_config();
                if self.validation_errors.is_empty() {
                    Some(Msg::ConfigActivity(ConfigActivityMsg::ConfirmAndProceed(
                        self.to_config_update_data(),
                    )))
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent { code: Key::Tab, .. }) => {
                // Tab no longer needed - could use for something else or ignore
                None
            }
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => {
                if self.editing_mode {
                    None // Ignore navigation while editing
                } else {
                    let result = self.perform(Cmd::Move(tuirealm::command::Direction::Up));
                    if matches!(result, CmdResult::Changed(_)) {
                        Some(Msg::ForceRedraw)
                    } else {
                        None
                    }
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            }) => {
                if self.editing_mode {
                    None // Ignore navigation while editing
                } else {
                    let result = self.perform(Cmd::Move(tuirealm::command::Direction::Down));
                    if matches!(result, CmdResult::Changed(_)) {
                        Some(Msg::ForceRedraw)
                    } else {
                        None
                    }
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Left, ..
            }) => {
                if self.editing_mode {
                    None // Ignore navigation while editing
                } else {
                    let result = self.perform(Cmd::Move(tuirealm::command::Direction::Left));
                    if matches!(result, CmdResult::Changed(_)) {
                        Some(Msg::ForceRedraw)
                    } else {
                        None
                    }
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Right, ..
            }) => {
                if self.editing_mode {
                    None // Ignore navigation while editing
                } else {
                    let result = self.perform(Cmd::Move(tuirealm::command::Direction::Right));
                    if matches!(result, CmdResult::Changed(_)) {
                        Some(Msg::ForceRedraw)
                    } else {
                        None
                    }
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Enter, ..
            }) => {
                if self.editing_mode {
                    // Confirm current input
                    let mut all_items = vec![("auth_method", "Authentication Method", true)];
                    let fields = self.get_fields_for_auth_method();
                    all_items.extend(fields);

                    if let Some((field_name, _, _)) = all_items.get(self.selected_field) {
                        self.set_field_value(field_name, self.current_input.clone());
                    }

                    self.editing_mode = false;
                    self.current_input.clear();
                    // Update global editing state to re-enable global key handling
                    Some(Msg::SetEditingMode(false))
                } else {
                    // Get all items (auth method + fields)
                    let mut all_items = vec![("auth_method", "Authentication Method", true)];
                    let fields = self.get_fields_for_auth_method();
                    all_items.extend(fields);

                    if let Some((field_name, _, _)) = all_items.get(self.selected_field) {
                        log::debug!("Enter pressed on field: {field_name}");

                        if *field_name == "auth_method" {
                            // Cycle through auth methods
                            let auth_methods = Self::get_auth_methods();
                            let current_idx = auth_methods
                                .iter()
                                .position(|(method, _)| *method == self.form_data.auth_method)
                                .unwrap_or(0);
                            let next_idx = (current_idx + 1) % auth_methods.len();
                            self.form_data.auth_method = auth_methods[next_idx].0.to_string();

                            // Reset field selection when auth method changes
                            self.selected_field = 0;
                            Some(Msg::ForceRedraw)
                        } else {
                            // Start editing the field
                            self.editing_mode = true;
                            self.current_input = self.get_field_value(field_name).to_string();
                            // Update global editing state to disable global key handling
                            Some(Msg::SetEditingMode(true))
                        }
                    } else {
                        None
                    }
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Backspace,
                ..
            }) => {
                if self.editing_mode {
                    self.current_input.pop();
                    Some(Msg::ForceRedraw)
                } else {
                    None
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c), ..
            }) => {
                if self.editing_mode {
                    log::debug!(
                        "ConfigScreen: Adding character '{c}' to input (current length: {})",
                        self.current_input.len()
                    );
                    // Add character to current input (limit length for practical reasons)
                    if self.current_input.len() < 512 {
                        self.current_input.push(c);
                        log::debug!("ConfigScreen: Current input now: '{}'", self.current_input);
                        // Return ForceRedraw but only after adding the character
                        Some(Msg::ForceRedraw)
                    } else {
                        Some(Msg::ForceRedraw) // Still consume the event and redraw
                    }
                } else {
                    // When not editing, don't consume the event so global keys work
                    None
                }
            }
            Event::Keyboard(_) => {
                // When in editing mode, consume other keyboard events to prevent global key handling
                if self.editing_mode {
                    Some(Msg::ForceRedraw)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
