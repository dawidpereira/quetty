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

    // Base colors
    pub fn background(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.background)
    }

    pub fn surface(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.surface)
    }

    pub fn overlay(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.overlay)
    }

    // Text colors
    pub fn text_primary(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.text_primary)
    }

    pub fn text_secondary(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.text_secondary)
    }

    pub fn text_muted(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.text_muted)
    }

    // Queue-specific colors
    pub fn queue_name(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.queue_name)
    }

    pub fn queue_count(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.queue_count)
    }

    pub fn namespace_name(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.namespace_name)
    }

    // Message table colors
    pub fn message_row(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.message_row)
    }

    pub fn message_row_selected(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.message_row_selected)
    }

    pub fn message_id(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.message_id)
    }

    pub fn message_sequence(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.message_sequence)
    }

    pub fn message_timestamp(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.message_timestamp)
    }

    // Table structure colors
    pub fn table_border(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.table_border)
    }

    pub fn table_border_focused(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.table_border_focused)
    }

    pub fn table_header(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.table_header)
    }

    // Selection and highlighting
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

    pub fn highlight_symbol(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.highlight_symbol)
    }

    // Status colors
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

    // Popup colors
    pub fn popup_background(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.popup_background)
    }

    pub fn popup_border(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.popup_border)
    }

    pub fn popup_title(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.popup_title)
    }

    pub fn popup_text(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.popup_text)
    }

    // Bulk selection colors
    pub fn bulk_checkbox_checked(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.bulk_checkbox_checked)
    }

    pub fn bulk_checkbox_unchecked(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.bulk_checkbox_unchecked)
    }

    // DLQ colors
    pub fn dlq_indicator(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.dlq_indicator)
    }

    pub fn dlq_queue_name(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.dlq_queue_name)
    }

    // Navigation colors
    pub fn pagination_info(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.pagination_info)
    }

    pub fn navigation_hint(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.navigation_hint)
    }

    pub fn navigation_active(&self) -> Color {
        self.current_theme
            .colors
            .hex_to_color(&self.current_theme.colors.navigation_active)
    }

    // Help colors
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

    /// Get available themes
    pub fn discover_themes(&self) -> AppResult<Vec<(String, Vec<String>)>> {
        self.loader.discover_themes()
    }

    /// Check if themes directory exists
    pub fn themes_dir_exists(&self) -> bool {
        self.loader.themes_dir_exists()
    }
}
