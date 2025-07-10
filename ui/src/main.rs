mod app;
mod components;
mod config;
mod error;
mod logger;
mod services;
mod theme;
mod utils;
mod validation;

use app::application_lifecycle::ApplicationLifecycle;
use std::error::Error as StdError;

/// Main entry point for the Quetty application.
///
/// This function initializes the terminal-based Azure Service Bus queue management application
/// by setting up logging, initializing the application lifecycle, configuring the terminal,
/// running the main application loop, and properly shutting down resources.
///
/// # Application Flow
///
/// 1. **Logger Setup** - Initializes the logging system for debugging and error tracking
/// 2. **Application Initialization** - Creates and configures the application model
/// 3. **Terminal Setup** - Configures the terminal for TUI display
/// 4. **Main Loop** - Runs the interactive application loop handling user input
/// 5. **Shutdown** - Properly cleans up resources and restores terminal state
///
/// # Errors
///
/// Returns an error if any critical initialization step fails, including:
/// - Application initialization failures
/// - Terminal setup failures
/// - Application loop execution errors
/// - Shutdown process errors
///
/// # Examples
///
/// ```no_run
/// // Run the application
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     main().await
/// }
/// ```
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
