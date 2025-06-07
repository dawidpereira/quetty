use crate::error::{AppError, AppResult};
use crate::theme::{
    loader::ThemeLoader,
    types::{Theme, ThemeCollectionWithMetadata, ThemeConfig},
};
use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex};
use tuirealm::props::Color;

// Global theme manager instance - now wrapped in Mutex for thread-safe updates
static GLOBAL_THEME_MANAGER: OnceCell<Mutex<ThemeManager>> = OnceCell::new();

pub struct ThemeManager {
    current_theme: Arc<Theme>,
    loader: ThemeLoader,
}

impl ThemeManager {
    /// Initialize the global theme manager - call this once at app startup
    pub fn init_global(config: &ThemeConfig) -> AppResult<()> {
        let loader = ThemeLoader::new();
        let theme = loader.load_theme_from_config(config)?;

        let manager = Self {
            current_theme: Arc::new(theme),
            loader,
        };

        GLOBAL_THEME_MANAGER
            .set(Mutex::new(manager))
            .map_err(|_| AppError::Config("Theme manager already initialized".to_string()))?;

        log::info!("Global theme manager initialized");
        Ok(())
    }

    /// Get the global theme manager instance
    pub fn global() -> &'static Mutex<ThemeManager> {
        GLOBAL_THEME_MANAGER
            .get()
            .expect("Theme manager not initialized. Call ThemeManager::init_global() first.")
    }

    /// Switch to a new theme by name and flavor
    pub fn switch_theme(&mut self, theme_name: &str, flavor_name: &str) -> AppResult<()> {
        let theme = self.loader.load_theme(theme_name, flavor_name)?;
        self.current_theme = Arc::new(theme);
        log::info!("Switched to theme: {} ({})", theme_name, flavor_name);
        Ok(())
    }

    /// Switch to a new theme using ThemeConfig
    pub fn switch_theme_from_config(&mut self, config: &ThemeConfig) -> AppResult<()> {
        self.switch_theme(&config.theme_name, &config.flavor_name)
    }

    // === Convenience methods for accessing theme colors ===
    // These are static methods that access the global theme manager

    // === Core Text Colors ===
    pub fn text_primary() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.text_primary)
    }

    pub fn text_muted() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.text_muted)
    }

    // === Layout Colors ===
    pub fn surface() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.surface)
    }

    // === Accent Colors ===
    pub fn primary_accent() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.primary_accent)
    }

    pub fn title_accent() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.title_accent)
    }

    pub fn header_accent() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.header_accent)
    }

    // === Selection Colors ===
    pub fn selection_bg() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.selection_bg)
    }

    pub fn selection_fg() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.selection_fg)
    }

    // === Message Table Colors ===
    pub fn message_sequence() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.message_sequence)
    }

    pub fn message_id() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.message_id)
    }

    pub fn message_timestamp() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.message_timestamp)
    }

    pub fn message_delivery_count() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.message_delivery_count)
    }

    // === Message State Group Colors ===
    pub fn message_state_ready() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.message_state_ready)
    }

    pub fn message_state_deferred() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.message_state_deferred)
    }

    pub fn message_state_outcome() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.message_state_outcome)
    }

    pub fn message_state_failed() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.message_state_failed)
    }

    // === List Item Colors ===
    pub fn namespace_list_item() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.namespace_list_item)
    }

    // === Status Colors ===
    pub fn status_success() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.status_success)
    }

    pub fn status_warning() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.status_warning)
    }

    pub fn status_error() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.status_error)
    }

    pub fn status_info() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.status_info)
    }

    pub fn status_loading() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.status_loading)
    }

    // === Help System Colors ===
    pub fn shortcut_key() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.shortcut_key)
    }

    pub fn shortcut_description() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.shortcut_description)
    }

    pub fn help_section_title() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.help_section_title)
    }

    // === Popup System Colors (used by confirmation popup) ===

    pub fn popup_text() -> Color {
        let manager = Self::global().lock().unwrap();
        manager
            .current_theme
            .colors
            .hex_to_color(&manager.current_theme.colors.popup_text)
    }

    /// Get available themes with metadata
    pub fn discover_themes_with_metadata(&self) -> AppResult<ThemeCollectionWithMetadata> {
        let themes = self.loader.discover_themes()?;
        let mut result = Vec::new();

        for (theme_name, flavors) in themes {
            let mut flavor_data = Vec::new();

            for flavor_name in flavors {
                match self.loader.load_theme(&theme_name, &flavor_name) {
                    Ok(theme) => {
                        let flavor_display = theme
                            .metadata
                            .flavor_name
                            .as_ref()
                            .unwrap_or(&flavor_name)
                            .clone();
                        let theme_icon = theme
                            .metadata
                            .theme_icon
                            .clone()
                            .unwrap_or_else(|| self.get_default_theme_icon(&theme_name));
                        let flavor_icon = theme
                            .metadata
                            .flavor_icon
                            .clone()
                            .unwrap_or_else(|| self.get_default_flavor_icon(&flavor_name));

                        flavor_data.push((flavor_display, theme_icon, flavor_icon));
                    }
                    Err(e) => {
                        log::warn!("Failed to load theme {}:{}: {}", theme_name, flavor_name, e);
                        // Use fallback values
                        let theme_icon = self.get_default_theme_icon(&theme_name);
                        let flavor_icon = self.get_default_flavor_icon(&flavor_name);
                        flavor_data.push((flavor_name, theme_icon, flavor_icon));
                    }
                }
            }

            if !flavor_data.is_empty() {
                result.push((theme_name, flavor_data));
            }
        }

        Ok(result)
    }

    /// Get default icon for themes (single fallback)
    fn get_default_theme_icon(&self, _theme_name: &str) -> String {
        "🎨".to_string()
    }

    /// Get default icon for flavors (single fallback)
    fn get_default_flavor_icon(&self, _flavor_name: &str) -> String {
        "🎭".to_string()
    }

    /// Static method to get available themes with metadata from global manager
    pub fn global_discover_themes_with_metadata() -> AppResult<ThemeCollectionWithMetadata> {
        let manager = Self::global().lock().unwrap();
        manager.discover_themes_with_metadata()
    }
}
