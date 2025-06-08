mod error;
use app::model::Model;
use components::common::ComponentId;
use config::CONFIG;
use error::AppError;
use log::{debug, error, info};
use std::error::Error as StdError;
use tuirealm::application::PollStrategy;
use tuirealm::terminal::CrosstermTerminalAdapter;
use tuirealm::{AttrValue, Attribute, Update};

mod app;
mod components;
mod config;
mod logger;
mod theme;

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
            .expect("Failed to initialize model for error display");

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
        while !self.model.quit {
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
                        self.model.quit = true;
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
    let config_loading_result = std::panic::catch_unwind(|| &*CONFIG);

    let config = match config_loading_result {
        Ok(config) => config,
        Err(_) => {
            let error_msg = "Failed to load configuration. Please check your config.toml file for syntax errors.";
            error!("{}", error_msg);
            eprintln!("Critical configuration error: {}", error_msg);
            eprintln!("Please fix your configuration and try again.");
            return Ok(());
        }
    };

    // Initialize global theme manager with loaded config
    use crate::theme::ThemeManager;
    let theme_config = config.theme();
    if let Err(e) = ThemeManager::init_global(&theme_config) {
        log::error!("Failed to initialize theme manager: {}", e);
        return Err(Box::new(e) as Box<dyn StdError>);
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
    let mut model = Model::new().await.expect("Failed to initialize model");
    info!("Model initialized successfully");

    // Enter alternate screen
    debug!("Entering alternate screen");
    let _ = model.terminal.enter_alternate_screen();
    let _ = model.terminal.enable_raw_mode();

    info!("Entering main application loop");
    // Main loop
    while !model.quit {
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
                model.redraw = true;
            }
            Ok(messages) if !messages.is_empty() => {
                // NOTE: redraw if at least one msg has been processed
                model.redraw = true;
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
        if model.redraw {
            if let Err(e) = model.view() {
                error!("Error during view rendering: {}", e);
                // Show error in popup
                if let Err(popup_err) = model.mount_error_popup(&e) {
                    error!("Failed to mount error popup: {}", popup_err);
                    // Fallback to old error handling
                    error::handle_error(e);
                }
            }
            model.redraw = false;
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
