//! Application lifecycle management
//!
//! This module handles the initialization, main loop, and shutdown of the application.
//! It extracts complex functionality from main.rs to improve maintainability.

use crate::app::model::Model;
use crate::components::common::{ComponentId, Msg};
use crate::config::{self, AppConfig, ConfigValidationError};
use crate::error::{AppError, ErrorReporter};
use crate::theme::{ThemeConfig, ThemeManager};

use log::{debug, error, info};
use std::error::Error as StdError;
use tuirealm::application::PollStrategy;
use tuirealm::terminal::CrosstermTerminalAdapter;
use tuirealm::{AttrValue, Attribute, Update};

/// Result of theme initialization attempt
#[derive(Debug)]
pub enum ThemeInitializationResult {
    /// Theme loaded successfully with no errors
    Success,
    /// User theme failed, but fallback to default succeeded. Contains error message to show user.
    FallbackSuccess { error_message: String },
    /// Both user theme and default theme failed. Application should exit.
    CriticalFailure { error_message: String },
}

/// Manages the display of configuration errors with user interaction
pub struct ConfigErrorDisplay {
    model: Model<CrosstermTerminalAdapter>,
}

impl ConfigErrorDisplay {
    /// Initialize the error display with the given validation errors
    pub async fn new(
        validation_errors: Vec<ConfigValidationError>,
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
    pub async fn show_and_wait_for_acknowledgment(&mut self) -> Result<(), Box<dyn StdError>> {
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
    pub fn shutdown(mut self) {
        info!("Terminating application due to configuration errors");
        self.model.shutdown();
        let _ = self.model.terminal.leave_alternate_screen();
        let _ = self.model.terminal.disable_raw_mode();
        let _ = self.model.terminal.clear_screen();
    }
}

/// Application initialization and lifecycle management
pub struct ApplicationLifecycle;

impl ApplicationLifecycle {
    /// Initialize the application and return the configured model
    pub async fn initialize() -> Result<Model<CrosstermTerminalAdapter>, Box<dyn StdError>> {
        info!("Starting Quetty application");

        let config = Self::load_configuration()?;
        let theme_init_result = Self::initialize_theme(&config.theme())?;
        Self::validate_configuration(&config).await?;

        info!("Configuration loaded and validated successfully");

        let mut model = Self::create_model().await?;
        Self::handle_theme_fallback(&mut model, theme_init_result)?;

        Ok(model)
    }

    /// Load and validate the application configuration
    fn load_configuration() -> Result<&'static AppConfig, Box<dyn StdError>> {
        match config::get_config() {
            config::ConfigLoadResult::Success(config) => Ok(config.as_ref()),
            config::ConfigLoadResult::LoadError(error) => {
                Self::report_critical_error(
                    AppError::Config(error.to_string()),
                    "ConfigurationLoader",
                    "load_config",
                    "Configuration loading failed. The application cannot start without a valid configuration.",
                );
                Err(error.to_string().into())
            }
            config::ConfigLoadResult::DeserializeError(error) => {
                Self::report_critical_error(
                    AppError::Config(error.to_string()),
                    "ConfigurationParser",
                    "parse_config",
                    "Configuration parsing failed. Please fix your configuration syntax and try again.",
                );
                Err(error.to_string().into())
            }
        }
    }

    /// Initialize the global theme manager
    fn initialize_theme(
        theme_config: &ThemeConfig,
    ) -> Result<ThemeInitializationResult, Box<dyn StdError>> {
        let result = Self::try_initialize_theme_manager(theme_config);

        // Handle critical theme failures immediately
        if let ThemeInitializationResult::CriticalFailure { error_message } = &result {
            Self::report_critical_error(
                AppError::Config(error_message.clone()),
                "ThemeManager",
                "initialize",
                "Application cannot start due to theme initialization failure. Please check your theme configuration.",
            );
            return Err(error_message.clone().into());
        }

        Ok(result)
    }

    /// Attempt to initialize theme manager with fallback
    fn try_initialize_theme_manager(theme_config: &ThemeConfig) -> ThemeInitializationResult {
        // Try to initialize with user's theme config first
        if let Err(e) = ThemeManager::init_global(theme_config) {
            error!("Failed to initialize theme manager with user config: {}", e);

            // Try to fallback to default theme
            let default_config = ThemeConfig::default();
            if let Err(default_e) = ThemeManager::init_global(&default_config) {
                error!(
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
                info!("Successfully fell back to default theme");
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

    /// Validate configuration after theme manager is initialized
    async fn validate_configuration(config: &AppConfig) -> Result<(), Box<dyn StdError>> {
        if let Err(validation_errors) = config.validate() {
            error!(
                "Configuration validation failed with {} errors",
                validation_errors.len()
            );
            Self::show_config_error_and_exit(validation_errors).await?;
            return Err("Configuration validation failed".into());
        }
        Ok(())
    }

    /// Create and initialize the application model
    async fn create_model() -> Result<Model<CrosstermTerminalAdapter>, Box<dyn StdError>> {
        match Model::new().await {
            Ok(model) => {
                info!("Model initialized successfully");
                Ok(model)
            }
            Err(e) => {
                Self::report_critical_error(
                    AppError::Component(e.to_string()),
                    "ApplicationModel",
                    "initialize",
                    "Failed to initialize application model. The application cannot start. Please check your configuration and try again.",
                );
                Err(e.into())
            }
        }
    }

    /// Handle theme fallback by showing error popup if needed
    fn handle_theme_fallback(
        model: &mut Model<CrosstermTerminalAdapter>,
        theme_init_result: ThemeInitializationResult,
    ) -> Result<(), Box<dyn StdError>> {
        if let ThemeInitializationResult::FallbackSuccess { error_message } = theme_init_result {
            if let Err(e) = model.mount_error_popup(&AppError::Config(error_message)) {
                model.error_reporter.report_config_error("theme", &e);
            }
        }
        Ok(())
    }

    /// Setup terminal for application use
    pub fn setup_terminal(
        model: &mut Model<CrosstermTerminalAdapter>,
    ) -> Result<(), Box<dyn StdError>> {
        debug!("Entering alternate screen");
        model
            .terminal
            .enter_alternate_screen()
            .map_err(|e| format!("Failed to enter alternate screen: {}", e))?;
        model
            .terminal
            .enable_raw_mode()
            .map_err(|e| format!("Failed to enable raw mode: {}", e))?;
        Ok(())
    }

    /// Run the main application loop
    pub fn run_application_loop(
        model: &mut Model<CrosstermTerminalAdapter>,
    ) -> Result<(), Box<dyn StdError>> {
        info!("Entering main application loop");

        while !model.state_manager.should_quit() {
            Self::process_single_iteration(model)?;
        }

        Ok(())
    }

    /// Process a single iteration of the main loop
    fn process_single_iteration(
        model: &mut Model<CrosstermTerminalAdapter>,
    ) -> Result<(), Box<dyn StdError>> {
        model.update_outside_msg();

        // Tick and handle messages
        match model.app.tick(PollStrategy::Once) {
            Err(err) => {
                Self::handle_tick_error(model, err)?;
            }
            Ok(messages) if !messages.is_empty() => {
                Self::process_messages(model, messages);
            }
            _ => {}
        }

        // Handle redraw if needed
        Self::handle_redraw(model)?;

        Ok(())
    }

    /// Handle tick errors by showing error popup
    fn handle_tick_error(
        model: &mut Model<CrosstermTerminalAdapter>,
        err: tuirealm::ApplicationError,
    ) -> Result<(), Box<dyn StdError>> {
        error!("Application tick error: {:?}", err);

        // Show error in popup
        if let Err(e) = model.mount_error_popup(&AppError::Component(format!(
            "Application error: {:?}",
            err
        ))) {
            error!("Failed to mount error popup: {}", e);
            // Fallback to simpler error handling
            if model
                .app
                .attr(
                    &ComponentId::TextLabel,
                    Attribute::Text,
                    AttrValue::String(format!("Application error: {:?}", err)),
                )
                .is_err()
            {
                return Err(format!("Failed to display error: {:?}", err).into());
            }
        }
        model.state_manager.set_redraw(true);
        Ok(())
    }

    /// Process all received messages
    fn process_messages(model: &mut Model<CrosstermTerminalAdapter>, messages: Vec<Msg>) {
        // Process all received messages and trigger redraw if any were handled
        model.state_manager.set_redraw(true);
        for msg in messages.into_iter() {
            let mut msg = Some(msg);
            while msg.is_some() {
                msg = model.update(msg);
            }
        }
    }

    /// Handle view redraw if needed
    fn handle_redraw(model: &mut Model<CrosstermTerminalAdapter>) -> Result<(), Box<dyn StdError>> {
        if model.state_manager.needs_redraw() {
            if let Err(e) = model.view() {
                error!("Error during view rendering: {}", e);
                // Show error in popup
                if let Err(popup_err) = model.mount_error_popup(&e) {
                    model
                        .error_reporter
                        .report_mount_error("ErrorPopup", "mount", popup_err);
                    // Since we can't show the error popup, report the original error through ErrorReporter
                    model
                        .error_reporter
                        .report_simple(e, "ViewRendering", "main_loop");
                }
            }
            model.state_manager.redraw_complete();
        }
        Ok(())
    }

    /// Properly shutdown the application
    pub fn shutdown_application(
        mut model: Model<CrosstermTerminalAdapter>,
    ) -> Result<(), Box<dyn StdError>> {
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

    /// Show configuration error and exit
    async fn show_config_error_and_exit(
        validation_errors: Vec<ConfigValidationError>,
    ) -> Result<(), Box<dyn StdError>> {
        let mut error_display = ConfigErrorDisplay::new(validation_errors).await?;
        error_display.show_and_wait_for_acknowledgment().await?;
        error_display.shutdown();
        Ok(())
    }

    /// Report critical error and prepare for application exit
    /// Uses ErrorReporter system for consistency with application error handling
    fn report_critical_error(
        error: AppError,
        component: &str,
        operation: &str,
        user_message: &str,
    ) {
        // Create a temporary ErrorReporter for critical initialization errors
        // This ensures consistency with the application's error handling patterns
        let (tx, _rx) = std::sync::mpsc::channel();
        let error_reporter = ErrorReporter::new(tx);

        // Use the specialized critical error reporting
        error_reporter.report_critical_and_exit(error, component, operation, user_message);

        // Also ensure the error is visible in case ErrorReporter fails
        eprintln!("Critical Error: {}", user_message);
    }
}
