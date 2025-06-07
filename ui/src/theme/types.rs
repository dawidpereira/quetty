use serde::{Deserialize, Serialize};
use tuirealm::props::Color;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeMetadata {
    pub name: String,
    pub description: String,
    pub author: String,
    // Optional theme-specific metadata
    pub theme_name: Option<String>,
    pub flavor_name: Option<String>,
    pub theme_icon: Option<String>,
    pub flavor_icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    // === Core Text Colors ===
    pub text_primary: String,
    pub text_muted: String,

    // === Layout Colors ===
    pub surface: String,

    // === Accent Colors ===
    pub primary_accent: String,
    pub title_accent: String,
    pub header_accent: String,

    // === Selection Colors ===
    pub selection_bg: String,
    pub selection_fg: String,

    // === Message Table Colors ===
    pub message_sequence: String,
    pub message_id: String,
    pub message_timestamp: String,
    pub message_delivery_count: String,
    pub queue_count: String,

    // === List Item Colors ===
    pub namespace_list_item: String,

    // === Status Colors ===
    pub status_success: String,
    pub status_warning: String,
    pub status_error: String,
    pub status_info: String,
    pub status_loading: String,

    // === Help System Colors ===
    pub shortcut_key: String,
    pub shortcut_description: String,
    pub help_section_title: String,

    // === Popup System Colors (used by confirmation popup) ===
    pub popup_background: String,
    pub popup_text: String,
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

/// Type alias for flavor metadata: (flavor_name, theme_icon, flavor_icon)
pub type FlavorMetadata = (String, String, String);

/// Type alias for theme collection with metadata: Vec<(theme_name, flavors)>
pub type ThemeCollectionWithMetadata = Vec<(String, Vec<FlavorMetadata>)>;

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
            theme_name: "quetty".to_string(),
            flavor_name: "dark".to_string(),
        }
    }
}
