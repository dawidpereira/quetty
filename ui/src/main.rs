mod app;
mod components;
mod config;
mod error;
mod logger;
mod theme;
mod validation;

use app::application_lifecycle::ApplicationLifecycle;
use std::error::Error as StdError;

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    // Initialize logger first
    if let Err(e) = logger::setup_logger() {
        eprintln!("Failed to initialize logger: {e}");
    }

    // Initialize application and get configured model
    let mut model = ApplicationLifecycle::initialize().await?;

    // Setup terminal
    ApplicationLifecycle::setup_terminal(&mut model)?;

    // Run main application loop
    ApplicationLifecycle::run_application_loop(&mut model)?;

    // Shutdown application
    ApplicationLifecycle::shutdown_application(model)?;

    Ok(())
}
