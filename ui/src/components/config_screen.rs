use crate::components::common::{ConfigActivityMsg, ConfigUpdateData, Msg};
use crate::components::state::ComponentState;
use crate::config::{self, AppConfig};
use crate::constants::env_vars::*;
use crate::error::AppResult;
use crate::theme::ThemeManager;
use crate::utils::auth::{
    AUTH_METHOD_CLIENT_SECRET, AUTH_METHOD_CONNECTION_STRING, AUTH_METHOD_DEVICE_CODE,
};
use crate::utils::connection_string::ConnectionStringParser;
use tuirealm::command::{Cmd, CmdResult};
use tuirealm::event::KeyEvent;
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

// UI text constants
const PLACEHOLDER_ENCRYPTED_CONNECTION_STRING: &str = "<<encrypted-connection-string-present>>";
const PLACEHOLDER_ENCRYPTED_CLIENT_SECRET: &str = "<<encrypted-client-secret-present>>";
const UI_HINT_CYCLE: &str = " (Enter to cycle)";
const UI_CURSOR_INDICATOR: &str = "_";
const UI_EMPTY_FIELD: &str = "<empty>";
const UI_PASSWORD_MASK: &str = "*";
const UI_ENCRYPTED_DATA_MESSAGE: &str = "****** (encrypted data present - enter new to replace)";

// UI size constants
const PASSWORD_DISPLAY_LENGTH: usize = 20;
const MAX_INPUT_LENGTH: usize = 512;

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
    pub master_password: String,
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
            master_password: String::new(),
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
    /// Helper to convert string fields to Option, returning None for empty/whitespace-only strings
    fn string_to_option(value: &str) -> Option<String> {
        if value.trim().is_empty() {
            None
        } else {
            Some(value.to_string())
        }
    }

    /// Create a new configuration screen with current configuration loaded
    ///
    /// Initializes the configuration form with values from the current app configuration
    /// and environment variables. Handles encrypted values by showing placeholders.
    /// Sets up the UI state for configuration editing and validation.
    ///
    /// # Returns
    /// A new ConfigScreen instance ready for user interaction
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
        // Check if we have existing encrypted connection string
        let connection_string = if config.servicebus().has_connection_string() {
            // Show placeholder indicating encrypted data exists - user only needs to enter password
            PLACEHOLDER_ENCRYPTED_CONNECTION_STRING.to_string()
        } else {
            // No encrypted connection string - user needs to enter new one
            String::new()
        };

        ConfigFormData {
            auth_method: config.azure_ad().auth_method.clone(),
            tenant_id: std::env::var(AZURE_AD_TENANT_ID).unwrap_or_default(),
            client_id: std::env::var(AZURE_AD_CLIENT_ID).unwrap_or_default(),
            client_secret: if std::env::var(AZURE_AD_ENCRYPTED_CLIENT_SECRET).is_ok() {
                PLACEHOLDER_ENCRYPTED_CLIENT_SECRET.to_string()
            } else {
                std::env::var(AZURE_AD_CLIENT_SECRET).unwrap_or_default()
            },
            subscription_id: std::env::var(AZURE_AD_SUBSCRIPTION_ID).unwrap_or_default(),
            resource_group: std::env::var(AZURE_AD_RESOURCE_GROUP).unwrap_or_default(),
            namespace: std::env::var(AZURE_AD_NAMESPACE).unwrap_or_default(),
            connection_string,
            master_password: String::new(), // Never pre-populate password for security
            queue_name: std::env::var(SERVICEBUS_QUEUE_NAME).unwrap_or_default(),
        }
    }

    /// Check if we're in "unlock" mode (encrypted data exists, just need password)
    /// vs "setup" mode (need to configure everything from scratch)
    fn is_unlock_mode(&self) -> bool {
        (self.form_data.auth_method == "connection_string"
            && self
                .form_data
                .connection_string
                .contains(PLACEHOLDER_ENCRYPTED_CONNECTION_STRING))
            || (self.form_data.auth_method == "client_secret"
                && self
                    .form_data
                    .client_secret
                    .contains(PLACEHOLDER_ENCRYPTED_CLIENT_SECRET))
    }

    fn get_auth_methods() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                AUTH_METHOD_CONNECTION_STRING,
                "Service Bus Connection String",
            ),
            (AUTH_METHOD_DEVICE_CODE, "Azure AD Device Code Flow"),
            (AUTH_METHOD_CLIENT_SECRET, "Azure AD Client Secret Flow"),
        ]
    }

    fn get_fields_for_auth_method(&self) -> Vec<(&'static str, &'static str, bool)> {
        match self.form_data.auth_method.as_str() {
            AUTH_METHOD_CONNECTION_STRING => {
                let connection_string_required = !self.is_unlock_mode();
                vec![
                    (
                        "connection_string",
                        "Connection String",
                        connection_string_required,
                    ),
                    ("master_password", "Master Password", true),
                    ("queue_name", "Queue Name", false),
                ]
            }
            AUTH_METHOD_DEVICE_CODE => vec![
                ("tenant_id", "Tenant ID", true),
                ("client_id", "Client ID", true),
                ("subscription_id", "Subscription ID", false),
                ("resource_group", "Resource Group", false),
                ("namespace", "Namespace", false),
            ],
            AUTH_METHOD_CLIENT_SECRET => vec![
                ("tenant_id", "Tenant ID", true),
                ("client_id", "Client ID", true),
                ("client_secret", "Client Secret", true),
                ("master_password", "Master Password", true),
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
            "master_password" => &self.form_data.master_password,
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
            "master_password" => self.form_data.master_password = value,
            "queue_name" => self.form_data.queue_name = value,
            _ => {}
        }
    }

    fn validate_config(&self) -> Vec<String> {
        self.validate_config_internal(false)
    }

    fn validate_config_for_save_only(&self) -> Vec<String> {
        self.validate_config_internal(true)
    }

    fn validate_config_internal(&self, require_password_for_encrypted: bool) -> Vec<String> {
        let mut errors = Vec::new();

        match self.form_data.auth_method.as_str() {
            AUTH_METHOD_CONNECTION_STRING => {
                let config = crate::config::get_config_or_panic();
                let has_encrypted_connection = config.servicebus().has_connection_string();
                let connection_string = self.form_data.connection_string.trim();

                log::debug!(
                    "Config validation: has_encrypted_connection={}, connection_string='{}', master_password_len={}, require_password_for_encrypted={}",
                    has_encrypted_connection,
                    connection_string,
                    self.form_data.master_password.len(),
                    require_password_for_encrypted
                );

                if has_encrypted_connection
                    && (connection_string.is_empty()
                        || connection_string.contains("<<encrypted-connection-string-present>>"))
                {
                    // We have an existing encrypted connection string and user didn't provide a new one
                    // For save-only mode, require password. For save-and-proceed mode, allow password popup
                    if require_password_for_encrypted
                        && self.form_data.master_password.trim().is_empty()
                    {
                        errors.push(
                            "Master password is required to access existing connection string"
                                .to_string(),
                        );
                    }
                } else if connection_string.is_empty() {
                    // No connection string provided and no existing one
                    errors.push(
                        "Connection string is required for connection_string auth method"
                            .to_string(),
                    );
                } else {
                    // New connection string provided - validate it
                    if let Err(validation_error) =
                        ConnectionStringParser::validate_connection_string(connection_string)
                    {
                        errors.push(format!("Invalid connection string: {validation_error}"));
                    }

                    if self.form_data.master_password.trim().is_empty() {
                        errors.push(
                            "Master password is required for connection string encryption"
                                .to_string(),
                        );
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
            AUTH_METHOD_CLIENT_SECRET => {
                if self.form_data.tenant_id.trim().is_empty() {
                    errors.push("Tenant ID is required for client secret flow".to_string());
                }
                if self.form_data.client_id.trim().is_empty() {
                    errors.push("Client ID is required for client secret flow".to_string());
                }
                if self.form_data.client_secret.trim().is_empty() {
                    errors.push("Client Secret is required for client secret flow".to_string());
                } else if !self
                    .form_data
                    .client_secret
                    .contains(PLACEHOLDER_ENCRYPTED_CLIENT_SECRET)
                {
                    // New client secret provided - require master password for encryption
                    if self.form_data.master_password.trim().is_empty() {
                        errors.push(
                            "Master password is required to encrypt the client secret".to_string(),
                        );
                    }
                }
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
                    format!("{method_desc}{UI_HINT_CYCLE}")
                } else {
                    let value = self.get_field_value(field);

                    // Show editing state if this field is being edited
                    if self.editing_mode && is_selected {
                        if *field == "client_secret"
                            || *field == "connection_string"
                            || *field == "master_password"
                        {
                            format!(
                                "{}{UI_CURSOR_INDICATOR}",
                                UI_PASSWORD_MASK.repeat(self.current_input.len())
                            )
                        } else {
                            format!("{}{UI_CURSOR_INDICATOR}", self.current_input)
                        }
                    } else if *field == "client_secret" || *field == "master_password" {
                        if value.is_empty() {
                            UI_EMPTY_FIELD.to_string()
                        } else {
                            UI_PASSWORD_MASK.repeat(value.len().min(PASSWORD_DISPLAY_LENGTH))
                        }
                    } else if *field == "connection_string" {
                        if value.contains(PLACEHOLDER_ENCRYPTED_CONNECTION_STRING) {
                            UI_ENCRYPTED_DATA_MESSAGE.to_string()
                        } else if value.is_empty() {
                            UI_EMPTY_FIELD.to_string()
                        } else {
                            UI_PASSWORD_MASK.repeat(value.len().min(PASSWORD_DISPLAY_LENGTH))
                        }
                    } else if value.is_empty() {
                        UI_EMPTY_FIELD.to_string()
                    } else {
                        value.to_string()
                    }
                };

                let (label_style, value_style) = if is_selected {
                    let value_color = if *field == "auth_method" {
                        // Color-code auth method values when selected
                        match self.form_data.auth_method.as_str() {
                            "connection_string" => ThemeManager::status_success(), // Green for connection string
                            "device_code" => ThemeManager::status_warning(), // Yellow for device code
                            "client_secret" => ThemeManager::message_delivery_count(), // Magenta for client secret
                            _ => ThemeManager::text_primary(), // White for others
                        }
                    } else {
                        ThemeManager::text_primary() // White for other fields
                    };

                    (
                        Style::default()
                            .fg(ThemeManager::primary_accent()) // Bright cyan for selected field name
                            .add_modifier(Modifier::BOLD),
                        Style::default().fg(value_color),
                    )
                } else if *required && self.get_field_value(field).trim().is_empty() {
                    (
                        Style::default().fg(ThemeManager::status_error()), // Red for error field name
                        Style::default().fg(ThemeManager::text_primary()), // Value stays white even on error
                    )
                } else {
                    let value_color = if *field == "auth_method" {
                        // Color-code auth method values when not selected
                        match self.form_data.auth_method.as_str() {
                            "connection_string" => ThemeManager::status_success(), // Green for connection string
                            "device_code" => ThemeManager::status_warning(), // Yellow for device code
                            "client_secret" => ThemeManager::message_delivery_count(), // Magenta for client secret
                            _ => ThemeManager::text_primary(), // White for others
                        }
                    } else {
                        ThemeManager::text_primary() // White for other fields
                    };

                    (
                        Style::default().fg(ThemeManager::message_id()), // Light blue for normal field names
                        Style::default().fg(value_color),
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
                (" save for next startup ".to_string(), false),
                ("[Ctrl+S]".to_string(), true),
                (" save & login".to_string(), false),
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
            tenant_id: Self::string_to_option(&self.form_data.tenant_id),
            client_id: Self::string_to_option(&self.form_data.client_id),
            client_secret: Self::string_to_option(&self.form_data.client_secret),
            subscription_id: Self::string_to_option(&self.form_data.subscription_id),
            resource_group: Self::string_to_option(&self.form_data.resource_group),
            namespace: Self::string_to_option(&self.form_data.namespace),
            connection_string: Self::string_to_option(&self.form_data.connection_string),
            master_password: Self::string_to_option(&self.form_data.master_password),
            queue_name: Self::string_to_option(&self.form_data.queue_name),
        }
    }

    /// Handle keyboard events in a focused manner
    fn handle_keyboard_event(&mut self, key_event: KeyEvent) -> Option<Msg> {
        use tuirealm::event::Key;
        use tuirealm::event::KeyModifiers;

        match key_event {
            KeyEvent { code: Key::Esc, .. } => self.handle_escape_key(),
            KeyEvent {
                code: Key::Char('s'),
                modifiers: KeyModifiers::NONE,
                ..
            } => self.handle_save_key(),
            KeyEvent {
                code: Key::Char('s'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => self.handle_save_and_proceed_key(),
            KeyEvent { code: Key::Tab, .. } => None, // Tab no longer needed
            KeyEvent { code: Key::Up, .. } => {
                self.handle_navigation_key(tuirealm::command::Direction::Up)
            }
            KeyEvent {
                code: Key::Down, ..
            } => self.handle_navigation_key(tuirealm::command::Direction::Down),
            KeyEvent {
                code: Key::Left, ..
            } => self.handle_navigation_key(tuirealm::command::Direction::Left),
            KeyEvent {
                code: Key::Right, ..
            } => self.handle_navigation_key(tuirealm::command::Direction::Right),
            KeyEvent {
                code: Key::Enter, ..
            } => self.handle_enter_key(),
            KeyEvent {
                code: Key::Backspace,
                ..
            } => self.handle_backspace_key(),
            KeyEvent {
                code: Key::Char('C'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => self.handle_reload_config_key(),
            KeyEvent {
                code: Key::Char(c), ..
            } => self.handle_character_input(c),
            _ => self.handle_other_keyboard_events(),
        }
    }

    /// Handle Escape key press
    fn handle_escape_key(&mut self) -> Option<Msg> {
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

    /// Handle save key ('s') press
    fn handle_save_key(&mut self) -> Option<Msg> {
        if !self.editing_mode {
            log::debug!("S pressed - validating config for save without login");
            self.validation_errors = self.validate_config_for_save_only();
            if self.validation_errors.is_empty() {
                log::debug!("Validation passed - saving config for next startup");
                Some(Msg::ConfigActivity(ConfigActivityMsg::Save(
                    self.to_config_update_data(),
                )))
            } else {
                log::debug!("Validation failed - not saving");
                None
            }
        } else {
            // In editing mode, treat 's' as character input
            self.handle_character_input('s')
        }
    }

    /// Handle save and proceed key (Ctrl+S) press
    fn handle_save_and_proceed_key(&mut self) -> Option<Msg> {
        log::debug!("Ctrl+S pressed - validating config for save and proceed");
        self.validation_errors = self.validate_config();
        log::debug!("Validation errors: {:?}", self.validation_errors);
        if self.validation_errors.is_empty() {
            log::debug!("Validation passed - saving config and triggering login");
            Some(Msg::ConfigActivity(ConfigActivityMsg::ConfirmAndProceed(
                self.to_config_update_data(),
            )))
        } else {
            log::debug!("Validation failed - not proceeding");
            None
        }
    }

    /// Handle navigation key press
    fn handle_navigation_key(&mut self, direction: tuirealm::command::Direction) -> Option<Msg> {
        if self.editing_mode {
            None // Ignore navigation while editing
        } else {
            let result = self.perform(Cmd::Move(direction));
            if matches!(result, CmdResult::Changed(_)) {
                Some(Msg::ForceRedraw)
            } else {
                None
            }
        }
    }

    /// Handle Enter key press
    fn handle_enter_key(&mut self) -> Option<Msg> {
        if self.editing_mode {
            self.handle_enter_confirm_editing()
        } else {
            self.handle_enter_start_editing()
        }
    }

    /// Handle Enter key when in editing mode (confirm editing)
    fn handle_enter_confirm_editing(&mut self) -> Option<Msg> {
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
    }

    /// Handle Enter key when not in editing mode (start editing or toggle auth method)
    fn handle_enter_start_editing(&mut self) -> Option<Msg> {
        let mut all_items = vec![("auth_method", "Authentication Method", true)];
        let fields = self.get_fields_for_auth_method();
        all_items.extend(fields);

        if let Some((field_name, _, _)) = all_items.get(self.selected_field) {
            log::debug!("Enter pressed on field: {field_name}");

            if *field_name == "auth_method" {
                self.handle_auth_method_toggle()
            } else {
                self.start_field_editing(field_name)
            }
        } else {
            None
        }
    }

    /// Handle auth method toggle
    fn handle_auth_method_toggle(&mut self) -> Option<Msg> {
        let auth_methods = Self::get_auth_methods();
        let current_idx = auth_methods
            .iter()
            .position(|(method, _)| *method == self.form_data.auth_method)
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % auth_methods.len();
        let new_auth_method = auth_methods[next_idx].0.to_string();

        log::debug!(
            "Auth method toggled from '{}' to '{}' - display only (not applied yet)",
            self.form_data.auth_method,
            new_auth_method
        );

        self.form_data.auth_method = new_auth_method;
        Some(Msg::ForceRedraw)
    }

    /// Start editing a field
    fn start_field_editing(&mut self, field_name: &str) -> Option<Msg> {
        self.editing_mode = true;
        // For sensitive fields, always start with empty input for security
        self.current_input = if field_name == "master_password" || field_name == "client_secret" {
            String::new()
        } else {
            self.get_field_value(field_name).to_string()
        };
        // Update global editing state to disable global key handling
        Some(Msg::SetEditingMode(true))
    }

    /// Handle Backspace key press
    fn handle_backspace_key(&mut self) -> Option<Msg> {
        if self.editing_mode {
            self.current_input.pop();
            Some(Msg::ForceRedraw)
        } else {
            None
        }
    }

    /// Handle config reload key (Ctrl+C) press
    fn handle_reload_config_key(&mut self) -> Option<Msg> {
        if !self.editing_mode {
            // Reload config data from current state
            log::debug!("ConfigScreen: Reloading configuration data");
            let config = crate::config::get_config_or_panic();
            self.form_data = Self::load_current_config(config);
            Some(Msg::ForceRedraw)
        } else {
            // In editing mode, treat 'C' as character input
            self.handle_character_input('C')
        }
    }

    /// Handle character input
    fn handle_character_input(&mut self, c: char) -> Option<Msg> {
        if self.editing_mode {
            log::debug!(
                "ConfigScreen: Adding character '{c}' to input (current length: {})",
                self.current_input.len()
            );
            // Add character to current input (limit length for practical reasons)
            if self.current_input.len() < MAX_INPUT_LENGTH {
                self.current_input.push(c);
                log::debug!("ConfigScreen: Current input now: '{}'", self.current_input);
                Some(Msg::ForceRedraw)
            } else {
                Some(Msg::ForceRedraw) // Still consume the event and redraw
            }
        } else {
            // When not editing, don't consume the event so global keys work
            None
        }
    }

    /// Handle other keyboard events
    fn handle_other_keyboard_events(&mut self) -> Option<Msg> {
        // When in editing mode, consume other keyboard events to prevent global key handling
        if self.editing_mode {
            Some(Msg::ForceRedraw)
        } else {
            None
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
            Event::Keyboard(key_event) => self.handle_keyboard_event(key_event),
            _ => None,
        }
    }
}
