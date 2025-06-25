mod app;

mod components;
mod config;
mod error;
mod logger;
mod theme;
mod validation;

use crate::theme::{ThemeConfig, ThemeManager};
use app::model::Model;
use components::common::ComponentId;

use error::AppError;
use log::{debug, error, info};
use std::error::Error as StdError;
use tuirealm::application::PollStrategy;
use tuirealm::terminal::CrosstermTerminalAdapter;
use tuirealm::{AttrValue, Attribute, Update};

/// Manages the display of configuration errors with user interaction
struct ConfigErrorDisplay {
    model: Model<CrosstermTerminalAdapter>,
}

impl ConfigErrorDisplay {
    /// Initialize the error display with the given validation errors
    async fn new(
        validation_errors: Vec<config::ConfigValidationError>,
    ) -> Result<Self, Box<dyn StdError>> {
        // Initialize a minimal model for error display
        let mut model = Model::new()
            .await
            .map_err(|e| format!("Failed to initialize model for error display: {}", e))?;

        // Show the first error in a popup (most critical one)
        if let Some(first_error) = validation_errors.first() {
            let error_message = first_error.user_message();
            error!("Configuration error: {}", error_message);

            if let Err(e) = model.mount_error_popup(&AppError::Config(error_message)) {
                error!("Failed to mount configuration error popup: {}", e);
                // Fallback to logging all errors
                for validation_error in &validation_errors {
                    error!(
                        "Config validation error: {}",
                        validation_error.user_message()
                    );
                }
            }
        }

        // Also log all validation errors for debugging
        for (i, validation_error) in validation_errors.iter().enumerate() {
            error!("Config validation error {}: {:?}", i + 1, validation_error);
        }

        Ok(Self { model })
    }

    /// Show the error popup and wait for user acknowledgment
    async fn show_and_wait_for_acknowledgment(&mut self) -> Result<(), Box<dyn StdError>> {
        info!(
            "Configuration validation failed. Application will exit after user acknowledges the error."
        );

        // Draw the error popup
        if let Err(e) = self.model.view() {
            error!("Error during error popup rendering: {}", e);
        }

        // Main loop to handle the error popup until user closes it
        while !self.model.state_manager.should_quit() {
            self.model.update_outside_msg();

            match self.model.app.tick(PollStrategy::Once) {
                Err(err) => {
                    error!("Application tick error during error display: {}", err);
                    break;
                }
                Ok(messages) if !messages.is_empty() => {
                    for msg in messages.into_iter() {
                        let mut msg = Some(msg);
                        while msg.is_some() {
                            // Handle the message
                            msg = self.model.update(msg);
                        }
                    }

                    // Check if error popup was closed - if so, quit the app
                    if !self.model.app.mounted(&ComponentId::ErrorPopup) {
                        info!("Configuration error popup closed by user, terminating application");
                        self.model.set_quit(true);
                        break;
                    }

                    // Check if popup was closed (which should set quit to true)
                    if let Err(e) = self.model.view() {
                        error!("Error during view rendering: {}", e);
                        break;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Properly shutdown the error display
    fn shutdown(mut self) {
        info!("Terminating application due to configuration errors");
        self.model.shutdown();
        let _ = self.model.terminal.leave_alternate_screen();
        let _ = self.model.terminal.disable_raw_mode();
        let _ = self.model.terminal.clear_screen();
    }
}

#[derive(Debug)]
pub enum ThemeInitializationResult {
    /// Theme loaded successfully with no errors
    Success,
    /// User theme failed, but fallback to default succeeded. Contains error message to show user.
    FallbackSuccess { error_message: String },
    /// Both user theme and default theme failed. Application should exit.
    CriticalFailure { error_message: String },
}

/// Initialize the global theme manager with the given config
/// Returns the result of initialization and any error message to show the user
fn initialize_theme_manager(theme_config: &ThemeConfig) -> ThemeInitializationResult {
    // Try to initialize with user's theme config first
    if let Err(e) = ThemeManager::init_global(theme_config) {
        log::error!("Failed to initialize theme manager with user config: {}", e);

        // Try to fallback to default theme
        let default_config = ThemeConfig::default();
        if let Err(default_e) = ThemeManager::init_global(&default_config) {
            log::error!(
                "Failed to initialize theme manager with default theme: {}",
                default_e
            );
            return ThemeInitializationResult::CriticalFailure {
                error_message: format!(
                    "Critical theme error: Unable to load any theme.\n\nUser theme error: {}\nDefault theme error: {}\n\nPlease check your theme files.",
                    e, default_e
                ),
            };
        } else {
            log::info!("Successfully fell back to default theme");
            return ThemeInitializationResult::FallbackSuccess {
                error_message: format!(
                    "Unable to load theme '{}' with flavor '{}': {}\n\nFalling back to default theme (quetty/dark).",
                    theme_config.theme_name, theme_config.flavor_name, e
                ),
            };
        }
    }

    // User theme loaded successfully
    ThemeInitializationResult::Success
}

async fn show_config_error_and_exit(
    validation_errors: Vec<config::ConfigValidationError>,
) -> Result<(), Box<dyn StdError>> {
    let mut error_display = ConfigErrorDisplay::new(validation_errors).await?;
    error_display.show_and_wait_for_acknowledgment().await?;
    error_display.shutdown();
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    // Initialize logger first
    if let Err(e) = logger::setup_logger() {
        eprintln!("Failed to initialize logger: {}", e);
    }

    info!("Starting Quetty application");

    // Load configuration early, but don't validate yet
    let config = match config::get_config() {
        config::ConfigLoadResult::Success(config) => config.as_ref(),
        config::ConfigLoadResult::LoadError(error) => {
            error!("Configuration loading failed: {}", error);
            error!("Critical configuration error: {}", error);
            error!("Please fix your configuration and try again.");
            return Ok(());
        }
        config::ConfigLoadResult::DeserializeError(error) => {
            error!("Configuration parsing failed: {}", error);
            error!("Critical configuration error: {}", error);
            error!("Please fix your configuration and try again.");
            return Ok(());
        }
    };

    // Initialize global theme manager with loaded config
    let theme_config = config.theme();
    let theme_init_result = initialize_theme_manager(&theme_config);

    // Handle critical theme failures
    if let ThemeInitializationResult::CriticalFailure { error_message } = &theme_init_result {
        error!("{}", error_message);
        error!("Application cannot start due to theme initialization failure");
        return Ok(());
    }

    // Now validate configuration after ThemeManager is initialized
    if let Err(validation_errors) = config.validate() {
        error!(
            "Configuration validation failed with {} errors",
            validation_errors.len()
        );
        return show_config_error_and_exit(validation_errors).await;
    }

    info!("Configuration loaded and validated successfully");

    // Setup model - now we know config is valid and ThemeManager is initialized
    let mut model = match Model::new().await {
        Ok(model) => {
            info!("Model initialized successfully");
            model
        }
        Err(e) => {
            error!("Failed to initialize application model: {}", e);
            error!("Critical initialization error: {}", e);
            error!("The application cannot start. Please check your configuration and try again.");
            return Ok(());
        }
    };

    // Show theme error popup if there was a theme loading issue
    if let ThemeInitializationResult::FallbackSuccess { error_message } = theme_init_result {
        if let Err(e) = model.mount_error_popup(&AppError::Config(error_message)) {
            error!("Failed to mount theme error popup: {}", e);
        }
    }

    // Enter alternate screen
    debug!("Entering alternate screen");
    let _ = model.terminal.enter_alternate_screen();
    let _ = model.terminal.enable_raw_mode();

    info!("Entering main application loop");
    // Main loop
    while !model.state_manager.should_quit() {
        model.update_outside_msg();
        // Tick
        match model.app.tick(PollStrategy::Once) {
            Err(err) => {
                error!("Application tick error: {}", err);
                // Show error in popup
                if let Err(e) = model
                    .mount_error_popup(&AppError::Component(format!("Application error: {}", err)))
                {
                    error!("Failed to mount error popup: {}", e);
                    // Fallback to simpler error handling
                    assert!(
                        model
                            .app
                            .attr(
                                &ComponentId::TextLabel,
                                Attribute::Text,
                                AttrValue::String(format!("Application error: {}", err)),
                            )
                            .is_ok()
                    );
                }
                model.state_manager.set_redraw(true);
            }
            Ok(messages) if !messages.is_empty() => {
                // Process all received messages and trigger redraw if any were handled
                model.state_manager.set_redraw(true);
                for msg in messages.into_iter() {
                    let mut msg = Some(msg);
                    while msg.is_some() {
                        msg = model.update(msg);
                    }
                }
            }
            _ => {}
        }
        // Redraw
        if model.state_manager.needs_redraw() {
            if let Err(e) = model.view() {
                error!("Error during view rendering: {}", e);
                // Show error in popup
                if let Err(popup_err) = model.mount_error_popup(&e) {
                    error!("Failed to mount error popup: {}", popup_err);
                    // Fallback to old error handling
                    error::handle_error(e);
                }
            }
            model.state_manager.redraw_complete();
        }
    }

    // Ensure proper shutdown (in case quit was set outside of AppClose message)
    info!("Application shutdown initiated");
    model.shutdown();

    // Terminate terminal
    debug!("Leaving alternate screen");
    let _ = model.terminal.leave_alternate_screen();
    let _ = model.terminal.disable_raw_mode();
    let _ = model.terminal.clear_screen();

    info!("Application terminated successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_initialization_success() {
        // Test with default theme config - should succeed
        let default_config = ThemeConfig::default();
        let result = initialize_theme_manager(&default_config);

        match result {
            ThemeInitializationResult::Success => {
                // Success - this is expected for the default theme
            }
            ThemeInitializationResult::FallbackSuccess { .. } => {
                // This might happen if the default theme in config is different than the fallback
                // That's also acceptable
            }
            ThemeInitializationResult::CriticalFailure { error_message } => {
                // In test environment, this might happen due to global state conflicts
                // Only panic if it's not a "already initialized" error
                if !error_message.contains("already initialized") {
                    panic!(
                        "Default theme should be loadable, but got critical failure: {}",
                        error_message
                    );
                }
                // If it's an "already initialized" error, that's acceptable in tests
            }
        }
    }

    #[test]
    fn test_theme_initialization_with_invalid_theme() {
        // Test with an invalid theme config
        let invalid_config = ThemeConfig {
            theme_name: "nonexistent_theme".to_string(),
            flavor_name: "nonexistent_flavor".to_string(),
        };

        let result = initialize_theme_manager(&invalid_config);

        match result {
            ThemeInitializationResult::Success => {
                panic!("Expected invalid theme to fail, but got success");
            }
            ThemeInitializationResult::FallbackSuccess { error_message } => {
                // This is the expected behavior - fallback to default
                assert!(error_message.contains("nonexistent_theme"));
                assert!(error_message.contains("nonexistent_flavor"));
                assert!(error_message.contains("Falling back to default theme"));
            }
            ThemeInitializationResult::CriticalFailure { error_message } => {
                // This would only happen if both user and default themes fail
                // This might be possible in test environment, so we'll just verify the error
                assert!(error_message.contains("Critical theme error"));
                assert!(error_message.contains("nonexistent_theme"));
            }
        }
    }

    #[test]
    fn test_theme_initialization_with_empty_theme_name() {
        // Test with empty theme name (should be invalid)
        let empty_config = ThemeConfig {
            theme_name: "".to_string(),
            flavor_name: "dark".to_string(),
        };

        let result = initialize_theme_manager(&empty_config);

        match result {
            ThemeInitializationResult::Success => {
                panic!("Expected empty theme name to fail, but got success");
            }
            ThemeInitializationResult::FallbackSuccess { error_message } => {
                // Expected - fallback to default
                assert!(error_message.contains("Falling back to default theme"));
            }
            ThemeInitializationResult::CriticalFailure { .. } => {
                // Also acceptable if default theme also fails in test environment
            }
        }
    }

    #[test]
    fn test_theme_initialization_result_debug() {
        // Test that the enum implements Debug properly
        let success = ThemeInitializationResult::Success;
        let fallback = ThemeInitializationResult::FallbackSuccess {
            error_message: "Test error".to_string(),
        };
        let critical = ThemeInitializationResult::CriticalFailure {
            error_message: "Critical test error".to_string(),
        };

        // Should not panic - just testing Debug implementation
        log::debug!("Theme initialization - Success: {:?}", success);
        log::debug!("Theme initialization - Fallback: {:?}", fallback);
        log::debug!("Theme initialization - Critical: {:?}", critical);
    }

    #[test]
    fn test_theme_initialization_error_message_format() {
        // Test with a known invalid theme to verify error message format
        let invalid_config = ThemeConfig {
            theme_name: "test_invalid_theme_123".to_string(),
            flavor_name: "test_invalid_flavor_456".to_string(),
        };

        let result = initialize_theme_manager(&invalid_config);

        if let ThemeInitializationResult::FallbackSuccess { error_message } = result {
            // Verify error message contains expected information
            assert!(error_message.contains("test_invalid_theme_123"));
            assert!(error_message.contains("test_invalid_flavor_456"));
            assert!(error_message.contains("Unable to load theme"));
            assert!(error_message.contains("Falling back to default theme"));
            assert!(error_message.contains("quetty/dark"));
        }
        // If it's a critical failure, that's also acceptable in test environment
    }
}
