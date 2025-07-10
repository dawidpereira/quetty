//! # Theme System Module
//!
//! Comprehensive theming system for the Quetty terminal user interface providing
//! dynamic theme loading, management, and runtime switching capabilities. This module
//! enables rich visual customization with support for multiple theme families and flavors.
//!
//! ## Architecture
//!
//! The theme system is built around several key components:
//!
//! - **[`ThemeManager`]** - Global theme management and runtime switching
//! - **[`ThemeLoader`]** - Dynamic theme discovery and loading from filesystem
//! - **[`ThemeConfig`]** - Configuration structure for theme selection
//! - **Theme Validation** - Comprehensive validation for theme files and structures
//!
//! ## Supported Theme Families
//!
//! The system supports multiple popular terminal theme families:
//!
//! ### Catppuccin
//! - **Latte** - Light, warm theme for daytime use
//! - **Frappé** - Medium contrast theme with balanced colors
//! - **Macchiato** - Darker theme with soft contrast
//! - **Mocha** - Deep dark theme for nighttime coding
//!
//! ### Nightfox
//! - **Nightfox** - Balanced dark theme with blue accents
//! - **Dayfox** - Light theme with warm tones
//! - **Carbonfox** - High contrast dark theme
//! - **Duskfox** - Muted dark theme with purple accents
//!
//! ### Quetty (Default)
//! - **Dark** - Application default dark theme
//! - **Light** - Application default light theme
//!
//! ## Basic Usage
//!
//! ### Initialization and Basic Operations
//! ```no_run
//! use ui::theme::{ThemeManager, ThemeConfig};
//!
//! // Initialize at application startup
//! let config = ThemeConfig {
//!     theme_name: "catppuccin".to_string(),
//!     flavor_name: "mocha".to_string(),
//! };
//! ThemeManager::init_global(&config)?;
//!
//! // Access theme colors throughout the application
//! let primary_color = ThemeManager::primary_accent();
//! let text_color = ThemeManager::text_primary();
//! let surface_color = ThemeManager::surface();
//! ```
//!
//! ### Runtime Theme Switching
//! ```no_run
//! use ui::theme::ThemeManager;
//!
//! // Switch to a different theme at runtime
//! {
//!     let mut manager = ThemeManager::global().lock().unwrap();
//!     manager.switch_theme("nightfox", "carbonfox")?;
//! }
//!
//! // Colors automatically update for all components
//! let new_accent = ThemeManager::primary_accent();
//! ```
//!
//! ### Theme Discovery
//! ```no_run
//! use ui::theme::ThemeManager;
//!
//! // Discover available themes and flavors
//! let available_themes = ThemeManager::get_available_themes();
//! for (theme_name, flavors) in available_themes {
//!     println!("Theme: {}", theme_name);
//!     for (flavor_name, theme_icon, flavor_icon) in flavors {
//!         println!("  Flavor: {} {} {}", flavor_name, theme_icon, flavor_icon);
//!     }
//! }
//! ```
//!
//! ## Color Categories
//!
//! ### Core Colors
//! - **Text Colors** - Primary and muted text for readability
//! - **Surface Colors** - Background and container colors
//! - **Accent Colors** - Primary, title, and header accents
//!
//! ### Functional Colors
//! - **Selection Colors** - Highlighting and focus indicators
//! - **Status Colors** - Success, warning, error, info, and loading states
//! - **Message Colors** - Queue-specific colors for different message states
//!
//! ### Component-Specific Colors
//! - **Help System** - Shortcut keys and descriptions
//! - **Popups** - Modal backgrounds and text
//! - **Lists** - Namespace and item-specific styling
//!
//! ## Component Integration
//!
//! ### Using Colors in TUI Components
//! ```no_run
//! use ui::theme::ThemeManager;
//! use tuirealm::props::{PropPayload, PropValue};
//!
//! // Set component colors from theme
//! component.attr(
//!     PropName::ForegroundColor,
//!     PropPayload::One(PropValue::Color(ThemeManager::text_primary())),
//! );
//!
//! component.attr(
//!     PropName::BackgroundColor,
//!     PropPayload::One(PropValue::Color(ThemeManager::surface())),
//! );
//! ```
//!
//! ### Dynamic Color Updates
//! ```no_run
//! use ui::theme::ThemeManager;
//!
//! // Components can react to theme changes
//! pub fn update_colors(&mut self) {
//!     self.primary_color = ThemeManager::primary_accent();
//!     self.text_color = ThemeManager::text_primary();
//!     self.background_color = ThemeManager::surface();
//!     // Trigger component redraw...
//! }
//! ```
//!
//! ## Error Handling and Fallbacks
//!
//! The theme system provides graceful degradation:
//!
//! - **Missing Themes** - Falls back to default Quetty dark theme
//! - **Invalid Colors** - Uses fallback colors for invalid hex values
//! - **Loading Errors** - Continues with previously loaded theme or defaults
//! - **Thread Safety** - Handles concurrent access safely with mutex protection
//!
//! ## Theme File Structure
//!
//! Themes are loaded from `~/.config/quetty/themes/` directory:
//! ```
//! themes/
//! ├── catppuccin/
//! │   ├── latte.toml
//! │   ├── frappe.toml
//! │   ├── macchiato.toml
//! └── nightfox/
//!     ├── nightfox.toml
//!     ├── dayfox.toml
//!     └── carbonfox.toml
//! ```

pub mod loader;
pub mod manager;
pub mod types;
pub mod validation;

pub use manager::ThemeManager;
pub use types::ThemeConfig;
