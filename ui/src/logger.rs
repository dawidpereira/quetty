use crate::config;
use fern::colors::{Color, ColoredLevelConfig};
use log::LevelFilter;
use std::fs::OpenOptions;

pub fn setup_logger() -> Result<(), log::SetLoggerError> {
    let config = config::get_config_or_panic();
    let log_level = match config.logging().level().to_lowercase().as_str() {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => LevelFilter::Info, // Default to Info for any other value
    };

    let colors = ColoredLevelConfig::new()
        .trace(Color::BrightBlack)
        .debug(Color::BrightBlue)
        .info(Color::Green)
        .warn(Color::Yellow)
        .error(Color::Red);

    // Base configuration for all outputs
    let base_config = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                colors.color(record.level()),
                record.target(),
                message
            ))
        })
        .level(log_level);

    // Always ensure we have at least one log output
    let log_file = config.logging().file();

    // Create log file if configured
    if let Some(file_path) = log_file {
        match OpenOptions::new().create(true).append(true).open(file_path) {
            Ok(file) => {
                // Only log to the file
                base_config.chain(file).apply()?;
                // Print initialization message (will show before TUI starts)
                println!("Logging to file: {}", file_path);
            }
            Err(e) => {
                eprintln!("Warning: Failed to open log file '{}': {}", file_path, e);
                eprintln!("Continuing without file logging.");
                // Apply base config without file output
                base_config.apply()?;
            }
        }
    } else {
        // If no file is configured, create a default log file in the current directory
        let default_log_path = "quetty.log";
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(default_log_path)
        {
            Ok(file) => {
                // Only log to the file
                base_config.chain(file).apply()?;
                // Print initialization message (will show before TUI starts)
                println!(
                    "No log file configured. Logging to default file: {}",
                    default_log_path
                );
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to open default log file '{}': {}",
                    default_log_path, e
                );
                eprintln!("Continuing without file logging.");
                // Apply base config without file output
                base_config.apply()?;
            }
        }
    }

    log::info!(
        "Logger initialized with level: {}",
        config.logging().level()
    );
    Ok(())
}
