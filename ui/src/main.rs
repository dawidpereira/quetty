mod app;
mod components;
mod config;
mod constants;
mod error;
mod logger;
mod services;
mod theme;
mod utils;
mod validation;

use app::application_lifecycle::ApplicationLifecycle;
use clap::{Arg, Command};
use config::{get_config_dir, is_config_initialized, wizard::SetupWizard};
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
    // Parse command line arguments
    let matches = Command::new("quetty")
        .version(env!("CARGO_PKG_VERSION"))
        .about("A powerful terminal-based Azure Service Bus queue manager")
        .arg(
            Arg::new("config")
                .long("config")
                .short('c')
                .value_name("FILE")
                .help("Use custom configuration file"),
        )
        .arg(
            Arg::new("setup")
                .long("setup")
                .action(clap::ArgAction::SetTrue)
                .help("Run interactive setup wizard to create configuration"),
        )
        .arg(
            Arg::new("config-dir")
                .long("config-dir")
                .action(clap::ArgAction::SetTrue)
                .help("Show configuration directory path and exit"),
        )
        .arg(
            Arg::new("profile")
                .long("profile")
                .short('p')
                .value_name("NAME")
                .help("Use specified profile (default: 'default')"),
        )
        .get_matches();

    // Handle --config-dir flag
    if matches.get_flag("config-dir") {
        match get_config_dir() {
            Ok(config_dir) => {
                println!("Configuration directory: {}", config_dir.display());
                println!("Initialized: {}", is_config_initialized());
                return Ok(());
            }
            Err(e) => {
                eprintln!("Error: Failed to determine config directory: {e}");
                std::process::exit(1);
            }
        }
    }

    // Get profile name (default to "default")
    let profile_name = matches
        .get_one::<String>("profile")
        .map(|s| s.as_str())
        .unwrap_or("default");

    // Handle --setup flag
    if matches.get_flag("setup") {
        match SetupWizard::run_for_profile(profile_name) {
            Ok(()) => {
                return Ok(());
            }
            Err(e) => {
                eprintln!("Error: Setup failed for profile '{profile_name}': {e}");
                std::process::exit(1);
            }
        }
    }

    // Get custom config path if provided
    let custom_config_path = matches.get_one::<String>("config").map(|s| s.as_str());

    // Initialize application and get configured model (this will set up the config)
    let mut model =
        ApplicationLifecycle::initialize_with_config_and_profile(custom_config_path, profile_name)
            .await?;

    // Initialize logger after config is loaded
    if let Err(e) = logger::setup_logger() {
        eprintln!("Failed to initialize logger: {e}");
    }

    // Setup terminal
    ApplicationLifecycle::setup_terminal(&mut model)?;

    // Run main application loop
    ApplicationLifecycle::run_application_loop(&mut model)?;

    // Shutdown application
    ApplicationLifecycle::shutdown_application(model)?;

    Ok(())
}
