//! # Theme Manager
//!
//! Centralized theme management system for the Quetty application.
//! Provides thread-safe access to themes, color schemes, and runtime theme switching.
//!
//! ## Features
//!
//! - **Global Theme Management** - Singleton pattern for application-wide theme access
//! - **Runtime Theme Switching** - Change themes without restarting the application
//! - **Fallback Colors** - Graceful degradation when theme loading fails
//! - **Thread Safety** - Safe concurrent access from multiple UI components
//! - **Theme Discovery** - Automatic discovery of available themes and flavors
//!
//! ## Usage
//!
//! ```no_run
//! use ui::theme::{manager::ThemeManager, types::ThemeConfig};
//!
//! // Initialize at application startup
//! let config = ThemeConfig {
//!     theme_name: "catppuccin".to_string(),
//!     flavor_name: "mocha".to_string(),
//! };
//! ThemeManager::init_global(&config)?;
//!
//! // Use theme colors in components
//! let primary_color = ThemeManager::primary_accent();
//! let text_color = ThemeManager::text_primary();
//!
//! // Switch themes at runtime
//! {
//!     let mut manager = ThemeManager::global().lock().unwrap();
//!     manager.switch_theme("nightfox", "carbonfox")?;
//! }
//! ```

use crate::error::{AppError, AppResult};
use crate::theme::{
    loader::ThemeLoader,
    types::{Theme, ThemeCollectionWithMetadata, ThemeConfig},
};
use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex};

use tuirealm::props::Color;

/// Global theme manager instance - thread-safe singleton for application-wide theme access
static GLOBAL_THEME_MANAGER: OnceCell<Mutex<ThemeManager>> = OnceCell::new();

/// Fallback colors used when theme loading fails or theme manager is unavailable.
///
/// These colors provide a basic, readable color scheme that ensures the application
/// remains functional even when themes cannot be loaded.
mod fallback_colors {
    use tuirealm::props::Color;

    pub const TEXT_PRIMARY: Color = Color::White;
    pub const TEXT_MUTED: Color = Color::Gray;
    pub const SURFACE: Color = Color::Black;
    pub const PRIMARY_ACCENT: Color = Color::Cyan;
    pub const TITLE_ACCENT: Color = Color::LightCyan;
    pub const HEADER_ACCENT: Color = Color::Blue;
    pub const SELECTION_BG: Color = Color::DarkGray;
    pub const SELECTION_FG: Color = Color::White;
    pub const MESSAGE_SEQUENCE: Color = Color::Yellow;
    pub const MESSAGE_ID: Color = Color::LightBlue;
    pub const MESSAGE_TIMESTAMP: Color = Color::Green;
    pub const MESSAGE_DELIVERY_COUNT: Color = Color::Magenta;
    pub const MESSAGE_STATE_READY: Color = Color::Green;
    pub const MESSAGE_STATE_DEFERRED: Color = Color::Yellow;
    pub const MESSAGE_STATE_OUTCOME: Color = Color::Blue;
    pub const MESSAGE_STATE_FAILED: Color = Color::Red;
    pub const NAMESPACE_LIST_ITEM: Color = Color::White;
    pub const STATUS_SUCCESS: Color = Color::Green;
    pub const STATUS_WARNING: Color = Color::Yellow;
    pub const STATUS_ERROR: Color = Color::Red;
    pub const STATUS_INFO: Color = Color::Blue;
    pub const STATUS_LOADING: Color = Color::Cyan;
    pub const SHORTCUT_KEY: Color = Color::LightCyan;
    pub const SHORTCUT_DESCRIPTION: Color = Color::Gray;
    pub const HELP_SECTION_TITLE: Color = Color::LightBlue;
    pub const POPUP_TEXT: Color = Color::White;
}

/// Central theme management system for the application.
///
/// ThemeManager provides thread-safe access to theme data and supports runtime
/// theme switching. It maintains the current theme state and provides accessor
/// methods for all color values used throughout the application.
///
/// # Thread Safety
///
/// The ThemeManager is designed to be used as a global singleton wrapped in a Mutex.
/// All color accessor methods are thread-safe and include fallback mechanisms.
pub struct ThemeManager {
    current_theme: Arc<Theme>,
    loader: ThemeLoader,
}

impl ThemeManager {
    /// Initializes the global theme manager singleton.
    ///
    /// This method must be called once at application startup before any theme
    /// colors are accessed. It loads the initial theme from the provided configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Theme configuration specifying the initial theme and flavor
    ///
    /// # Returns
    ///
    /// `Ok(())` if initialization succeeds
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The theme manager is already initialized
    /// - The initial theme cannot be loaded
    /// - Theme files are invalid or missing
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ui::theme::{manager::ThemeManager, types::ThemeConfig};
    ///
    /// let config = ThemeConfig {
    ///     theme_name: "catppuccin".to_string(),
    ///     flavor_name: "mocha".to_string(),
    /// };
    /// ThemeManager::init_global(&config)?;
    /// ```
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

    /// Gets a reference to the global theme manager instance.
    ///
    /// # Returns
    ///
    /// A reference to the Mutex-wrapped ThemeManager
    ///
    /// # Panics
    ///
    /// Panics if the theme manager has not been initialized with [`init_global`]
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ui::theme::manager::ThemeManager;
    ///
    /// let manager = ThemeManager::global();
    /// let mut theme_manager = manager.lock().unwrap();
    /// theme_manager.switch_theme("nightfox", "carbonfox")?;
    /// ```
    pub fn global() -> &'static Mutex<ThemeManager> {
        GLOBAL_THEME_MANAGER
            .get()
            .expect("Theme manager not initialized. Call ThemeManager::init_global() first.")
    }

    /// Safe helper function to access the theme manager with fallback on lock contention.
    ///
    /// This method provides non-blocking access to the theme manager. If the lock
    /// cannot be acquired immediately or the manager is not initialized, it returns
    /// the provided fallback value.
    ///
    /// # Type Parameters
    ///
    /// * `F` - Function that operates on the theme
    /// * `R` - Return type
    ///
    /// # Arguments
    ///
    /// * `f` - Function to execute with the current theme
    /// * `fallback` - Value to return if theme access fails
    fn with_theme_manager<F, R>(f: F, fallback: R) -> R
    where
        F: FnOnce(&Arc<Theme>) -> R,
    {
        match GLOBAL_THEME_MANAGER.get() {
            Some(manager_mutex) => {
                // Try to acquire lock with timeout
                match manager_mutex.try_lock() {
                    Ok(manager) => f(&manager.current_theme),
                    Err(_) => {
                        log::warn!("Theme manager lock contention, using fallback");
                        fallback
                    }
                }
            }
            None => {
                log::warn!("Theme manager not initialized, using fallback");
                fallback
            }
        }
    }

    /// Gets a color from the current theme with automatic fallback.
    ///
    /// This method safely extracts a color from the theme using the provided
    /// getter function. If theme access fails, it returns the fallback color.
    ///
    /// # Type Parameters
    ///
    /// * `F` - Function that extracts a color from the theme
    ///
    /// # Arguments
    ///
    /// * `color_getter` - Function to extract the desired color
    /// * `fallback` - Color to use if theme access fails
    fn get_theme_color<F>(color_getter: F, fallback: Color) -> Color
    where
        F: FnOnce(&Theme) -> Color,
    {
        Self::with_theme_manager(|theme| color_getter(theme), fallback)
    }
}

// Macro to generate theme accessor methods with proper error handling and fallbacks
macro_rules! theme_accessor {
    ($method:ident, $field:ident, $fallback:expr) => {
        impl ThemeManager {
            pub fn $method() -> Color {
                Self::get_theme_color(
                    |theme| theme.colors.hex_to_color(&theme.colors.$field),
                    $fallback,
                )
            }
        }
    };
}

// Generate all theme accessor methods
theme_accessor!(text_primary, text_primary, fallback_colors::TEXT_PRIMARY);
theme_accessor!(text_muted, text_muted, fallback_colors::TEXT_MUTED);
theme_accessor!(surface, surface, fallback_colors::SURFACE);
theme_accessor!(
    primary_accent,
    primary_accent,
    fallback_colors::PRIMARY_ACCENT
);
theme_accessor!(title_accent, title_accent, fallback_colors::TITLE_ACCENT);
theme_accessor!(header_accent, header_accent, fallback_colors::HEADER_ACCENT);
theme_accessor!(selection_bg, selection_bg, fallback_colors::SELECTION_BG);
theme_accessor!(selection_fg, selection_fg, fallback_colors::SELECTION_FG);
theme_accessor!(
    message_sequence,
    message_sequence,
    fallback_colors::MESSAGE_SEQUENCE
);
theme_accessor!(message_id, message_id, fallback_colors::MESSAGE_ID);
theme_accessor!(
    message_timestamp,
    message_timestamp,
    fallback_colors::MESSAGE_TIMESTAMP
);
theme_accessor!(
    message_delivery_count,
    message_delivery_count,
    fallback_colors::MESSAGE_DELIVERY_COUNT
);
theme_accessor!(
    message_state_ready,
    message_state_ready,
    fallback_colors::MESSAGE_STATE_READY
);
theme_accessor!(
    message_state_deferred,
    message_state_deferred,
    fallback_colors::MESSAGE_STATE_DEFERRED
);
theme_accessor!(
    message_state_outcome,
    message_state_outcome,
    fallback_colors::MESSAGE_STATE_OUTCOME
);
theme_accessor!(
    message_state_failed,
    message_state_failed,
    fallback_colors::MESSAGE_STATE_FAILED
);
theme_accessor!(
    namespace_list_item,
    namespace_list_item,
    fallback_colors::NAMESPACE_LIST_ITEM
);
theme_accessor!(
    status_success,
    status_success,
    fallback_colors::STATUS_SUCCESS
);
theme_accessor!(
    status_warning,
    status_warning,
    fallback_colors::STATUS_WARNING
);
theme_accessor!(status_error, status_error, fallback_colors::STATUS_ERROR);
theme_accessor!(status_info, status_info, fallback_colors::STATUS_INFO);
theme_accessor!(
    status_loading,
    status_loading,
    fallback_colors::STATUS_LOADING
);
theme_accessor!(shortcut_key, shortcut_key, fallback_colors::SHORTCUT_KEY);
theme_accessor!(
    shortcut_description,
    shortcut_description,
    fallback_colors::SHORTCUT_DESCRIPTION
);
theme_accessor!(
    help_section_title,
    help_section_title,
    fallback_colors::HELP_SECTION_TITLE
);
theme_accessor!(popup_text, popup_text, fallback_colors::POPUP_TEXT);

impl ThemeManager {
    /// Switches to a new theme by name and flavor.
    ///
    /// This method loads and activates a new theme at runtime. All subsequent
    /// color accessor calls will use the new theme colors.
    ///
    /// # Arguments
    ///
    /// * `theme_name` - Name of the theme (e.g., "catppuccin", "nightfox")
    /// * `flavor_name` - Name of the flavor (e.g., "mocha", "carbonfox")
    ///
    /// # Returns
    ///
    /// `Ok(())` if the theme switch succeeds
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The theme or flavor does not exist
    /// - Theme files are invalid or corrupted
    /// - File system access fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ui::theme::manager::ThemeManager;
    ///
    /// let mut manager = ThemeManager::global().lock().unwrap();
    /// manager.switch_theme("catppuccin", "latte")?;
    /// ```
    pub fn switch_theme(&mut self, theme_name: &str, flavor_name: &str) -> AppResult<()> {
        let theme = self.loader.load_theme(theme_name, flavor_name)?;
        self.current_theme = Arc::new(theme);
        log::info!("Switched to theme: {theme_name} ({flavor_name})");
        Ok(())
    }

    /// Switches to a new theme using a ThemeConfig struct.
    ///
    /// Convenience method for switching themes when you have a ThemeConfig.
    ///
    /// # Arguments
    ///
    /// * `config` - Theme configuration containing theme and flavor names
    ///
    /// # Returns
    ///
    /// `Ok(())` if the theme switch succeeds
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`switch_theme`]
    pub fn switch_theme_from_config(&mut self, config: &ThemeConfig) -> AppResult<()> {
        self.switch_theme(&config.theme_name, &config.flavor_name)
    }

    /// Discovers all available themes with their metadata and icons.
    ///
    /// This method scans the theme directories and loads metadata for all
    /// available themes and flavors. It includes display names, icons, and
    /// other theme information.
    ///
    /// # Returns
    ///
    /// [`ThemeCollectionWithMetadata`] containing all discovered themes with their metadata
    ///
    /// # Errors
    ///
    /// Returns an error if theme discovery or loading fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ui::theme::manager::ThemeManager;
    ///
    /// let manager = ThemeManager::global().lock().unwrap();
    /// let themes = manager.discover_themes_with_metadata()?;
    ///
    /// for (theme_name, flavors) in themes {
    ///     println!("Theme: {}", theme_name);
    ///     for (flavor_name, theme_icon, flavor_icon) in flavors {
    ///         println!("  {} {} {}", theme_icon, flavor_icon, flavor_name);
    ///     }
    /// }
    /// ```
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
                        log::warn!("Failed to load theme {theme_name}:{flavor_name}: {e}");
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

    /// Static method to discover themes using the global theme manager.
    ///
    /// Convenience method that accesses the global theme manager and discovers
    /// available themes. This method is thread-safe and includes lock contention handling.
    ///
    /// # Returns
    ///
    /// [`ThemeCollectionWithMetadata`] containing all discovered themes
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Theme manager is not initialized
    /// - Lock cannot be acquired
    /// - Theme discovery fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ui::theme::manager::ThemeManager;
    ///
    /// let themes = ThemeManager::global_discover_themes_with_metadata()?;
    /// ```
    pub fn global_discover_themes_with_metadata() -> AppResult<ThemeCollectionWithMetadata> {
        match GLOBAL_THEME_MANAGER.get() {
            Some(manager_mutex) => match manager_mutex.try_lock() {
                Ok(manager) => manager.discover_themes_with_metadata(),
                Err(_) => {
                    log::warn!("Theme manager lock contention during theme discovery");
                    Err(AppError::Config(
                        "Theme manager is busy, try again".to_string(),
                    ))
                }
            },
            None => {
                log::error!("Theme manager not initialized during theme discovery");
                Err(AppError::Config(
                    "Theme manager not initialized".to_string(),
                ))
            }
        }
    }
}
