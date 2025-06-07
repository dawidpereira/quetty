use serde::{Deserialize, Serialize};
use tuirealm::props::Color;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeMetadata {
    pub name: String,
    pub description: String,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    // Base colors
    pub background: String,
    pub surface: String,
    pub overlay: String,

    // Text colors
    pub text_primary: String,
    pub text_secondary: String,
    pub text_muted: String,
    pub text_disabled: String,

    // Queue-specific colors
    pub queue_name: String,
    pub queue_count: String,
    pub namespace_name: String,

    // Message table colors
    pub message_row: String,
    pub message_row_selected: String,
    pub message_row_alternate: String,
    pub message_id: String,
    pub message_sequence: String,
    pub message_timestamp: String,
    pub message_delivery_count: String,

    // Table structure colors
    pub table_border: String,
    pub table_border_focused: String,
    pub table_header: String,
    pub table_header_text: String,

    // Selection and highlighting
    pub selection_bg: String,
    pub selection_fg: String,
    pub highlight_symbol: String,
    pub focus_indicator: String,

    // Interactive element colors
    pub button_primary: String,
    pub button_secondary: String,
    pub button_danger: String,
    pub button_text: String,

    // Status and feedback colors
    pub status_success: String,
    pub status_warning: String,
    pub status_error: String,
    pub status_info: String,
    pub status_loading: String,

    // Popup colors
    pub popup_background: String,
    pub popup_border: String,
    pub popup_title: String,
    pub popup_text: String,

    // Bulk selection colors
    pub bulk_checkbox_checked: String,
    pub bulk_checkbox_unchecked: String,
    pub bulk_selection_count: String,

    // Dead letter queue colors
    pub dlq_indicator: String,
    pub dlq_queue_name: String,

    // Navigation and pagination colors
    pub pagination_info: String,
    pub navigation_hint: String,
    pub navigation_active: String,
    pub navigation_inactive: String,

    // Help and shortcuts colors
    pub shortcut_key: String,
    pub shortcut_description: String,
    pub help_section_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub metadata: ThemeMetadata,
    pub colors: ThemeColors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub theme_name: String,
    pub flavor_name: String,
}

impl ThemeColors {
    /// Convert a hex color string to tuirealm Color
    pub fn hex_to_color(&self, hex: &str) -> Color {
        if hex.is_empty() || hex == "reset" {
            return Color::Reset;
        }

        // Handle standard color names
        match hex.to_lowercase().as_str() {
            "black" => Color::Black,
            "red" => Color::Red,
            "green" => Color::Green,
            "yellow" => Color::Yellow,
            "blue" => Color::Blue,
            "magenta" => Color::Magenta,
            "cyan" => Color::Cyan,
            "white" => Color::White,
            "gray" | "grey" => Color::Gray,
            "darkgray" | "darkgrey" => Color::DarkGray,
            "lightred" => Color::LightRed,
            "lightgreen" => Color::LightGreen,
            "lightyellow" => Color::LightYellow,
            "lightblue" => Color::LightBlue,
            "lightmagenta" => Color::LightMagenta,
            "lightcyan" => Color::LightCyan,
            "reset" => Color::Reset,
            _ => {
                // Try to parse as hex color
                if let Ok(rgb) = self.parse_hex_color(hex) {
                    Color::Rgb(rgb.0, rgb.1, rgb.2)
                } else {
                    Color::Reset
                }
            }
        }
    }

    fn parse_hex_color(&self, hex: &str) -> Result<(u8, u8, u8), &'static str> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return Err("Invalid hex color format");
        }

        let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "Invalid red component")?;
        let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "Invalid green component")?;
        let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "Invalid blue component")?;

        Ok((r, g, b))
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            theme_name: "default".to_string(),
            flavor_name: "dark".to_string(),
        }
    }
}
