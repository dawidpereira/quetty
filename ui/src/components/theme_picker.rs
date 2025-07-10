use crate::components::base_popup::PopupBuilder;
use crate::components::common::{Msg, ThemeActivityMsg};
use crate::components::state::ComponentState;
use crate::error::AppResult;
use crate::theme::ThemeManager;
use crate::theme::types::ThemeCollectionWithMetadata;
use tuirealm::command::{Cmd, CmdResult};
use tuirealm::event::{Key, KeyEvent};
use tuirealm::props::{Alignment, Style, TextModifiers};
use tuirealm::ratatui::layout::Rect;
use tuirealm::ratatui::widgets::{List, ListItem};
use tuirealm::{
    AttrValue, Attribute, Component, Event, Frame, MockComponent, NoUserEvent, State, StateValue,
};

const CMD_RESULT_THEME_SELECTED: &str = "ThemeSelected";
const CMD_RESULT_CLOSE_PICKER: &str = "ClosePicker";

/// Interactive theme picker component for selecting application themes and flavors.
///
/// Provides a two-level selection interface where users first choose a theme family,
/// then select a specific flavor within that theme. Displays theme metadata including
/// icons and descriptions for better user experience.
///
/// # Navigation
///
/// - **Arrow Keys** - Navigate between themes and flavors
/// - **Enter** - Confirm selection and apply theme
/// - **Tab** - Switch between theme and flavor selection modes
/// - **Escape** - Cancel and close picker
///
/// # Examples
///
/// ```no_run
/// use ui::components::theme_picker::ThemePicker;
/// use ui::components::state::ComponentState;
///
/// let mut picker = ThemePicker::new();
/// picker.mount()?; // Loads available themes
///
/// // Component handles user input and theme selection
/// ```
pub struct ThemePicker {
    themes: ThemeCollectionWithMetadata, // (theme_name, [(flavor_name, theme_icon, flavor_icon)])
    theme_selected: usize,               // Selected theme index
    flavor_selected: usize,              // Selected flavor index for current theme
    mode: PickerMode,
}

#[derive(Debug, PartialEq)]
enum PickerMode {
    SelectingTheme,
    SelectingFlavor,
}

impl ThemePicker {
    pub fn new() -> Self {
        Self {
            themes: Vec::new(),
            theme_selected: 0,
            flavor_selected: 0,
            mode: PickerMode::SelectingTheme,
        }
    }

    pub fn load_themes(&mut self) {
        match ThemeManager::global_discover_themes_with_metadata() {
            Ok(themes) => {
                self.themes = themes;
                if !self.themes.is_empty() {
                    self.theme_selected = 0;
                    self.flavor_selected = 0;
                }
            }
            Err(e) => {
                log::error!("Failed to discover themes: {e}");
                // Fallback to default themes with generic icons
                self.themes = vec![(
                    "quetty".to_string(),
                    vec![
                        ("dark".to_string(), "🎨".to_string(), "🎭".to_string()),
                        ("light".to_string(), "🎨".to_string(), "🎭".to_string()),
                    ],
                )];
            }
        }
    }

    fn get_current_theme(&self) -> Option<&String> {
        self.themes.get(self.theme_selected).map(|(name, _)| name)
    }

    fn get_current_flavor(&self) -> Option<&String> {
        self.themes
            .get(self.theme_selected)
            .and_then(|(_, flavors)| flavors.get(self.flavor_selected).map(|(name, _, _)| name))
    }

    fn get_display_items(&self) -> Vec<String> {
        match self.mode {
            PickerMode::SelectingTheme => self
                .themes
                .iter()
                .map(|(name, flavors)| {
                    // Get theme icon from first flavor's metadata (they should all have the same theme icon)
                    let icon = flavors
                        .first()
                        .map(|(_, theme_icon, _)| theme_icon.as_str())
                        .unwrap_or("🎨");
                    format!("{icon} {name}")
                })
                .collect(),
            PickerMode::SelectingFlavor => {
                if let Some((theme_name, flavors)) = self.themes.get(self.theme_selected) {
                    flavors
                        .iter()
                        .map(|(flavor_name, _, flavor_icon)| {
                            format!("{flavor_icon} {flavor_name} ({theme_name})")
                        })
                        .collect()
                } else {
                    vec![]
                }
            }
        }
    }
}

impl MockComponent for ThemePicker {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let display_items = self.get_display_items();

        let items: Vec<ListItem> = display_items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let mut list_item = ListItem::new(item.clone());
                let selected_index = match self.mode {
                    PickerMode::SelectingTheme => self.theme_selected,
                    PickerMode::SelectingFlavor => self.flavor_selected,
                };

                if i == selected_index {
                    list_item = list_item.style(
                        Style::default()
                            .fg(ThemeManager::namespace_list_item())
                            .bg(ThemeManager::surface())
                            .add_modifier(TextModifiers::BOLD),
                    );
                } else {
                    list_item = list_item.style(Style::default().fg(ThemeManager::text_primary()));
                }
                list_item
            })
            .collect();

        let title = match self.mode {
            PickerMode::SelectingTheme => "  🎨 Select Theme  ".to_string(),
            PickerMode::SelectingFlavor => {
                if let Some(theme_name) = self.get_current_theme() {
                    format!("  🎭 Select {theme_name} Flavor  ")
                } else {
                    "  🎭 Select Flavor  ".to_string()
                }
            }
        };

        let instructions = match self.mode {
            PickerMode::SelectingTheme => "↑/↓/j/k: Navigate, Enter: Select Theme, Esc: Close",
            PickerMode::SelectingFlavor => "↑/↓/j/k: Navigate, Enter: Apply, Backspace/Esc: Back",
        };

        // Use PopupBuilder for consistent styling
        let popup_block = PopupBuilder::new("Theme Picker").create_block_with_title(title);

        let list = List::new(items)
            .block(popup_block)
            .highlight_style(
                Style::default()
                    .fg(ThemeManager::selection_fg())
                    .bg(ThemeManager::selection_bg())
                    .add_modifier(TextModifiers::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_widget(list, area);

        // Render instructions at the bottom
        let instruction_area = Rect {
            x: area.x,
            y: area.y + area.height - 1,
            width: area.width,
            height: 1,
        };

        if instruction_area.y < area.y + area.height {
            let instruction_widget = tuirealm::ratatui::widgets::Paragraph::new(instructions)
                .style(Style::default().fg(ThemeManager::text_muted()))
                .alignment(Alignment::Center);
            frame.render_widget(instruction_widget, instruction_area);
        }
    }

    fn query(&self, _attr: Attribute) -> Option<AttrValue> {
        None
    }

    fn attr(&mut self, _attr: Attribute, _value: AttrValue) {}

    fn state(&self) -> State {
        if let (Some(theme), Some(flavor)) = (self.get_current_theme(), self.get_current_flavor()) {
            State::One(StateValue::String(format!("{theme}:{flavor}")))
        } else {
            State::None
        }
    }

    fn perform(&mut self, _cmd: Cmd) -> CmdResult {
        CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for ThemePicker {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let cmd_result = match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            }) => {
                match self.mode {
                    PickerMode::SelectingTheme => {
                        if self.theme_selected + 1 < self.themes.len() {
                            self.theme_selected += 1;
                            self.flavor_selected = 0; // Reset flavor selection
                        }
                    }
                    PickerMode::SelectingFlavor => {
                        if let Some((_, flavors)) = self.themes.get(self.theme_selected) {
                            if self.flavor_selected + 1 < flavors.len() {
                                self.flavor_selected += 1;
                            }
                        }
                    }
                }
                CmdResult::Changed(State::One(StateValue::Usize(match self.mode {
                    PickerMode::SelectingTheme => self.theme_selected,
                    PickerMode::SelectingFlavor => self.flavor_selected,
                })))
            }
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => {
                match self.mode {
                    PickerMode::SelectingTheme => {
                        if self.theme_selected > 0 {
                            self.theme_selected -= 1;
                            self.flavor_selected = 0; // Reset flavor selection
                        }
                    }
                    PickerMode::SelectingFlavor => {
                        if self.flavor_selected > 0 {
                            self.flavor_selected -= 1;
                        }
                    }
                }
                CmdResult::Changed(State::One(StateValue::Usize(match self.mode {
                    PickerMode::SelectingTheme => self.theme_selected,
                    PickerMode::SelectingFlavor => self.flavor_selected,
                })))
            }
            Event::Keyboard(KeyEvent {
                code: Key::Enter, ..
            }) => {
                match self.mode {
                    PickerMode::SelectingTheme => {
                        // Move to flavor selection
                        self.mode = PickerMode::SelectingFlavor;
                        self.flavor_selected = 0;
                        CmdResult::Changed(State::One(StateValue::String(
                            "flavor_mode".to_string(),
                        )))
                    }
                    PickerMode::SelectingFlavor => {
                        // Apply the selected theme
                        if let (Some(theme), Some(flavor)) =
                            (self.get_current_theme(), self.get_current_flavor())
                        {
                            CmdResult::Custom(
                                CMD_RESULT_THEME_SELECTED,
                                State::One(StateValue::String(format!("{theme}:{flavor}"))),
                            )
                        } else {
                            CmdResult::None
                        }
                    }
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Backspace,
                ..
            }) => {
                if self.mode == PickerMode::SelectingFlavor {
                    self.mode = PickerMode::SelectingTheme;
                    CmdResult::Changed(State::One(StateValue::String("theme_mode".to_string())))
                } else {
                    CmdResult::None
                }
            }
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => {
                match self.mode {
                    PickerMode::SelectingFlavor => {
                        // Go back to theme selection mode
                        self.mode = PickerMode::SelectingTheme;
                        CmdResult::Changed(State::One(StateValue::String("theme_mode".to_string())))
                    }
                    PickerMode::SelectingTheme => {
                        // Close the picker
                        CmdResult::Custom(CMD_RESULT_CLOSE_PICKER, State::None)
                    }
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c), ..
            }) => {
                let keys = crate::config::get_config_or_panic().keys();
                if c == keys.up() {
                    match self.mode {
                        PickerMode::SelectingTheme => {
                            if self.theme_selected > 0 {
                                self.theme_selected -= 1;
                                self.flavor_selected = 0; // Reset flavor selection
                            }
                        }
                        PickerMode::SelectingFlavor => {
                            if self.flavor_selected > 0 {
                                self.flavor_selected -= 1;
                            }
                        }
                    }
                    CmdResult::Changed(State::One(StateValue::Usize(match self.mode {
                        PickerMode::SelectingTheme => self.theme_selected,
                        PickerMode::SelectingFlavor => self.flavor_selected,
                    })))
                } else if c == keys.down() {
                    match self.mode {
                        PickerMode::SelectingTheme => {
                            if self.theme_selected + 1 < self.themes.len() {
                                self.theme_selected += 1;
                                self.flavor_selected = 0; // Reset flavor selection
                            }
                        }
                        PickerMode::SelectingFlavor => {
                            if let Some((_, flavors)) = self.themes.get(self.theme_selected) {
                                if self.flavor_selected + 1 < flavors.len() {
                                    self.flavor_selected += 1;
                                }
                            }
                        }
                    }
                    CmdResult::Changed(State::One(StateValue::Usize(match self.mode {
                        PickerMode::SelectingTheme => self.theme_selected,
                        PickerMode::SelectingFlavor => self.flavor_selected,
                    })))
                } else {
                    CmdResult::None
                }
            }
            _ => CmdResult::None,
        };

        match cmd_result {
            CmdResult::Custom(CMD_RESULT_THEME_SELECTED, state) => {
                if let State::One(StateValue::String(theme_flavor)) = state {
                    let parts: Vec<&str> = theme_flavor.split(':').collect();
                    if parts.len() == 2 {
                        Some(Msg::ThemeActivity(ThemeActivityMsg::ThemeSelected(
                            parts[0].to_string(),
                            parts[1].to_string(),
                        )))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            CmdResult::Custom(CMD_RESULT_CLOSE_PICKER, _) => {
                Some(Msg::ThemeActivity(ThemeActivityMsg::ThemePickerClosed))
            }
            _ => Some(Msg::ForceRedraw),
        }
    }
}

impl ComponentState for ThemePicker {
    fn mount(&mut self) -> AppResult<()> {
        // Load themes during component mounting
        self.load_themes();
        Ok(())
    }
}

impl Default for ThemePicker {
    fn default() -> Self {
        Self::new()
    }
}
