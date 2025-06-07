mod error;
use app::model::Model;
use components::common::ComponentId;
use error::AppError;
use log::{debug, error, info};
use std::error::Error as StdError;
use tuirealm::application::PollStrategy;
use tuirealm::{AttrValue, Attribute, Update};

mod app;
mod components;
mod config;
mod logger;
mod theme;

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    // Initialize logger
    if let Err(e) = logger::setup_logger() {
        eprintln!("Failed to initialize logger: {}", e);
    }

    info!("Starting Quetty application");

    // Initialize global theme manager
    use crate::theme::ThemeManager;
    let theme_config = config::CONFIG.theme();
    if let Err(e) = ThemeManager::init_global(&theme_config) {
        log::error!("Failed to initialize theme manager: {}", e);
        return Err(Box::new(e) as Box<dyn StdError>);
    }

    // Setup model
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
                                &ComponentId::Label,
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
                debug!("Processing {} messages", messages.len());
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

    // Terminate terminal
    info!("Application shutdown initiated");
    debug!("Leaving alternate screen");
    let _ = model.terminal.leave_alternate_screen();
    let _ = model.terminal.disable_raw_mode();
    let _ = model.terminal.clear_screen();

    info!("Application terminated successfully");
    Ok(())
}
