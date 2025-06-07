use crate::error::{AppError, AppResult};
use crate::theme::{
    loader::ThemeLoader,
    types::{Theme, ThemeConfig},
};
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tuirealm::props::Color;

// Global theme manager instance
static GLOBAL_THEME_MANAGER: OnceCell<ThemeManager> = OnceCell::new();

pub struct ThemeManager {
    current_theme: Arc<Theme>,
    loader: ThemeLoader,
}

impl ThemeManager {
    /// Create a new theme manager and load the default theme
    pub fn new() -> AppResult<Self> {
        let loader = ThemeLoader::new();
        let theme = loader.load_default_theme()?;

        Ok(Self {
            current_theme: Arc::new(theme),
            loader,
        })
    }

    /// Initialize the global theme manager - call this once at app startup
    pub fn init_global(config: &ThemeConfig) -> AppResult<()> {
        let loader = ThemeLoader::new();
        let theme = loader.load_theme_from_config(config)?;

        let manager = Self {
            current_theme: Arc::new(theme),
            loader,
        };

        GLOBAL_THEME_MANAGER
            .set(manager)
            .map_err(|_| AppError::Config("Theme manager already initialized".to_string()))?;

        log::info!("Global theme manager initialized");
        Ok(())
    }

    /// Get the global theme manager instance
    pub fn global() -> &'static ThemeManager {
        GLOBAL_THEME_MANAGER
            .get()
            .expect("Theme manager not initialized. Call ThemeManager::init_global() first.")
    }

    /// Create a new theme manager with custom themes directory
    pub fn with_themes_dir<P: Into<std::path::PathBuf>>(themes_dir: P) -> AppResult<Self> {
        let loader = ThemeLoader::with_themes_dir(themes_dir);
        let theme = loader.load_default_theme()?;

        Ok(Self {
            current_theme: Arc::new(theme),
            loader,
        })
    }

    /// Load theme from config
    pub fn load_from_config(&mut self, config: &ThemeConfig) -> AppResult<()> {
        let theme = self.loader.load_theme_from_config(config)?;
        self.current_theme = Arc::new(theme);
        Ok(())
    }

    /// Get the current theme
    pub fn current_theme(&self) -> Arc<Theme> {
        self.current_theme.clone()
    }

    /// Get theme metadata
    pub fn theme_name(&self) -> &str {
        &self.current_theme.metadata.name
    }

    // === Core Text Colors ===
    pub fn text_primary(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.text_primary)
    }

    pub fn text_muted(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.text_muted)
    }

    // === Layout Colors ===
    pub fn surface(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.surface)
    }

    // === Accent Colors ===
    pub fn primary_accent(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.primary_accent)
    }

    pub fn title_accent(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.title_accent)
    }

    pub fn header_accent(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.header_accent)
    }

    // === Selection Colors ===
    pub fn selection_bg(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.selection_bg)
    }

    pub fn selection_fg(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.selection_fg)
    }

    // === Message Table Colors ===
    pub fn message_sequence(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.message_sequence)
    }

    pub fn message_id(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.message_id)
    }

    pub fn message_timestamp(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.message_timestamp)
    }

    pub fn message_delivery_count(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.message_delivery_count)
    }

    // === List Item Colors ===
    pub fn namespace_list_item(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.namespace_list_item)
    }

    // === Status Colors ===
    pub fn status_success(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.status_success)
    }

    pub fn status_warning(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.status_warning)
    }

    pub fn status_error(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.status_error)
    }

    pub fn status_info(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.status_info)
    }

    pub fn status_loading(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.status_loading)
    }

    // === Help System Colors ===
    pub fn shortcut_key(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.shortcut_key)
    }

    pub fn shortcut_description(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.shortcut_description)
    }

    pub fn help_section_title(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.help_section_title)
    }

    // === Popup System Colors (used by confirmation popup) ===
    pub fn popup_background(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.popup_background)
    }

    pub fn popup_text(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.popup_text)
    }

    /// Get available themes
    pub fn discover_themes(&self) -> AppResult<Vec<(String, Vec<String>)>> {
        self.loader.discover_themes()
    }

    /// Check if themes directory exists
    pub fn themes_dir_exists(&self) -> bool {
        self.loader.themes_dir_exists()
    }
}

