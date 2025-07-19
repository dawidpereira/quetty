use crate::config;
use log::LevelFilter;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn setup_logger() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::get_config_or_panic();
    let log_level = match config.logging().level().to_lowercase().as_str() {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => LevelFilter::Info, // Default to Info for any other value
    };

    let log_file = config.logging().file();
    let default_log_path = get_default_log_path();
    let file_path = log_file.unwrap_or(&default_log_path);

    // Parse the file path to get directory and filename
    let path = Path::new(file_path);
    let directory = path.parent().unwrap_or(Path::new("."));
    let filename = path
        .file_stem()
        .and_then(|f| f.to_str())
        .unwrap_or("quetty");

    // Create directory if it doesn't exist
    if let Err(e) = fs::create_dir_all(directory) {
        eprintln!(
            "Warning: Failed to create log directory '{}': {}",
            directory.display(),
            e
        );
        return setup_fallback_logger(log_level);
    }

    // Clean up old log files if enabled
    if config.logging().cleanup_on_startup() {
        if let Err(e) =
            cleanup_old_log_files(directory, filename, config.logging().max_backup_files())
        {
            eprintln!("Warning: Failed to clean up old log files: {e}");
        }
    }

    // Create rolling file appender with size-based rotation
    let max_size_bytes = config.logging().max_file_size_mb() * 1024 * 1024;

    // For now, use daily rotation as tracing-appender doesn't have size-based rotation
    // We'll implement size-based rotation manually
    let file_appender = match create_size_aware_appender(directory, filename, max_size_bytes) {
        Ok(writer) => writer,
        Err(e) => {
            eprintln!("Warning: Failed to create rotating log file: {e}");
            return setup_fallback_logger(log_level);
        }
    };

    // Setup fern with the rotating file appender
    let base_config = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log_level)
        .chain(Box::new(file_appender) as Box<dyn Write + Send>);

    if let Err(e) = base_config.apply() {
        eprintln!("Warning: Failed to initialize logger: {e}");
        return setup_fallback_logger(log_level);
    }

    // Print initialization message (will show before TUI starts)
    if log_file.is_some() {
        println!("Logging to file: {file_path} (with rotation)");
    } else {
        println!(
            "No log file configured. Logging to default file: {default_log_path} (with rotation)"
        );
    }

    log::info!(
        "Logger initialized with level: {}, max_size: {}MB, max_backups: {}",
        config.logging().level(),
        config.logging().max_file_size_mb(),
        config.logging().max_backup_files()
    );

    Ok(())
}

/// Creates a size-aware file writer that handles rotation
fn create_size_aware_appender(
    directory: &Path,
    filename: &str,
    max_size_bytes: u64,
) -> Result<SizeAwareWriter, Box<dyn std::error::Error>> {
    let log_path = directory.join(format!("{filename}.log"));
    let writer = SizeAwareWriter::new(log_path, max_size_bytes)?;
    Ok(writer)
}

/// Cleanup old log files, keeping only the specified number of backups
fn cleanup_old_log_files(
    directory: &Path,
    filename: &str,
    max_backups: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let backup_pattern = format!("{filename}.log.");

    let mut log_files = Vec::new();

    // Collect all log files (current and backups)
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        if file_name_str.starts_with(&backup_pattern) {
            if let Ok(metadata) = entry.metadata() {
                log_files.push((
                    entry.path(),
                    metadata.modified().unwrap_or(std::time::UNIX_EPOCH),
                ));
            }
        }
    }

    // Sort by modification time (newest first)
    log_files.sort_by(|a, b| b.1.cmp(&a.1));

    // Remove excess backup files
    if log_files.len() > max_backups as usize {
        for (path, _) in log_files.iter().skip(max_backups as usize) {
            if let Err(e) = fs::remove_file(path) {
                eprintln!(
                    "Warning: Failed to remove old log file '{}': {}",
                    path.display(),
                    e
                );
            }
        }
    }

    Ok(())
}

/// Fallback logger setup if rotating logger fails
fn setup_fallback_logger(log_level: LevelFilter) -> Result<(), Box<dyn std::error::Error>> {
    use fern::colors::{Color, ColoredLevelConfig};

    let colors = ColoredLevelConfig::new()
        .trace(Color::BrightBlack)
        .debug(Color::BrightBlue)
        .info(Color::Green)
        .warn(Color::Yellow)
        .error(Color::Red);

    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                colors.color(record.level()),
                record.target(),
                message
            ))
        })
        .level(log_level)
        .chain(std::io::stderr())
        .apply()?;

    eprintln!("Fallback: Logging to stderr");
    Ok(())
}

/// A writer that handles size-based log rotation
struct SizeAwareWriter {
    current_file: Option<File>,
    log_path: PathBuf,
    max_size: u64,
    current_size: u64,
}

unsafe impl Send for SizeAwareWriter {}

impl SizeAwareWriter {
    fn new(log_path: PathBuf, max_size: u64) -> Result<Self, std::io::Error> {
        let current_size = if log_path.exists() {
            fs::metadata(&log_path)?.len()
        } else {
            0
        };

        let current_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        Ok(SizeAwareWriter {
            current_file: Some(current_file),
            log_path,
            max_size,
            current_size,
        })
    }

    fn rotate_if_needed(&mut self) -> Result<(), std::io::Error> {
        if self.current_size >= self.max_size {
            // Flush and close current file
            if let Some(ref mut file) = self.current_file {
                file.flush()?;
            }

            // Drop the file to close it
            self.current_file = None;

            // Rotate: move current log to backup
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            let backup_path = self.log_path.with_extension(format!("log.{timestamp}"));

            if let Err(e) = fs::rename(&self.log_path, &backup_path) {
                eprintln!("Warning: Failed to rotate log file: {e}");
            }

            // Create new log file
            let new_file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&self.log_path)?;

            self.current_file = Some(new_file);
            self.current_size = 0;
        }
        Ok(())
    }
}

impl Write for SizeAwareWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Some(ref mut file) = self.current_file {
            let bytes_written = file.write(buf)?;
            self.current_size += bytes_written as u64;

            // Check if we need to rotate after writing
            if let Err(e) = self.rotate_if_needed() {
                eprintln!("Warning: Failed to rotate log file: {e}");
            }

            Ok(bytes_written)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "Log file not available",
            ))
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(ref mut file) = self.current_file {
            file.flush()
        } else {
            Ok(())
        }
    }
}

/// Get the default log path based on context (development vs production)
fn get_default_log_path() -> String {
    // In debug builds (development), use local logs directory for easy access
    if cfg!(debug_assertions) {
        // Always use project root for development logs, regardless of current directory
        let project_root = std::env::current_dir()
            .ok()
            .and_then(|path| {
                // If we're in ui directory, go up one level
                if path.file_name().and_then(|n| n.to_str()) == Some("ui") {
                    path.parent().map(|p| p.to_path_buf())
                } else {
                    Some(path)
                }
            })
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        project_root
            .join("logs")
            .join("quetty.log")
            .to_string_lossy()
            .to_string()
    } else {
        // In release builds (production), use OS cache directory
        if let Some(cache_dir) = dirs::cache_dir() {
            let log_dir = cache_dir.join("quetty").join("logs");
            log_dir.join("quetty.log").to_string_lossy().to_string()
        } else {
            // Fallback if cache directory detection fails
            "logs/quetty.log".to_string()
        }
    }
}
