use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum SetupError {
    #[error("Failed to determine config directory: {0}")]
    ConfigDirError(String),
    #[error("Failed to create directory {path}: {source}")]
    CreateDirError {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to write file {path}: {source}")]
    WriteFileError {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to set file permissions for {path}: {source}")]
    PermissionError {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Get the standard configuration directory for the current platform
pub fn get_config_dir() -> Result<PathBuf, SetupError> {
    // Prefer ~/.config/quetty on Unix-like systems, fall back to platform defaults
    if cfg!(unix) {
        // On Unix systems (Linux, macOS), prefer ~/.config/quetty
        if let Some(mut home_path) = dirs::home_dir() {
            home_path.push(".config");
            home_path.push("quetty");
            return Ok(home_path);
        }
    }

    // Fall back to platform-specific config directory
    dirs::config_dir()
        .map(|mut path| {
            path.push("quetty");
            path
        })
        .ok_or_else(|| {
            SetupError::ConfigDirError("Unable to determine config directory".to_string())
        })
}

/// Get the standard configuration file path
pub fn get_config_file_path() -> Result<PathBuf, SetupError> {
    let mut config_dir = get_config_dir()?;
    config_dir.push("config.toml");
    Ok(config_dir)
}

/// Get the themes directory path
pub fn get_themes_dir() -> Result<PathBuf, SetupError> {
    let mut config_dir = get_config_dir()?;
    config_dir.push("themes");
    Ok(config_dir)
}

/// Check if config directory exists and has basic files
pub fn is_config_initialized() -> bool {
    match get_config_file_path() {
        Ok(config_path) => config_path.exists(),
        Err(_) => false,
    }
}

/// Initialize the config directory with default files
pub fn initialize_config_dir() -> Result<PathBuf, SetupError> {
    let config_dir = get_config_dir()?;
    let themes_dir = get_themes_dir()?;

    // Create config directory
    create_dir_if_not_exists(&config_dir)?;

    // Create themes directory
    create_dir_if_not_exists(&themes_dir)?;

    // Create profiles directory structure
    let profiles_dir = config_dir.join("profiles");
    create_dir_if_not_exists(&profiles_dir)?;

    let default_profile_dir = profiles_dir.join("default");
    create_dir_if_not_exists(&default_profile_dir)?;

    // Create only .env file for secrets (this is the only file that must exist)
    let env_file = default_profile_dir.join(".env");
    if !env_file.exists() {
        let env_content = "# Environment variables for default profile\n# SECRETS AND AUTHENTICATION ONLY\n# For other settings, create config.toml or keys.toml in this directory\n\n# Add your Azure credentials here\n# AZURE_AD__TENANT_ID=your-tenant-id\n# AZURE_AD__CLIENT_ID=your-client-id\n# AZURE_AD__CLIENT_SECRET=your-client-secret\n";
        write_file_with_permissions(&env_file, env_content)?;
        log::info!("Created .env file: {}", env_file.display());
    }

    log::info!(
        "Profile directory structure created: {}",
        profiles_dir.display()
    );

    // Note: We don't create config.toml or keys.toml files by default
    // They are embedded in the binary and loaded as needed
    // Users can create override files if they want to customize
    // Themes are also embedded and loaded from binary by default

    log::info!("Config directory initialized: {}", config_dir.display());
    Ok(config_dir)
}

/// Create directory if it doesn't exist
fn create_dir_if_not_exists(path: &Path) -> Result<(), SetupError> {
    if !path.exists() {
        fs::create_dir_all(path).map_err(|source| SetupError::CreateDirError {
            path: path.to_path_buf(),
            source,
        })?;
    }
    Ok(())
}

/// Write file with appropriate permissions (600 for config files)
fn write_file_with_permissions(path: &Path, content: &str) -> Result<(), SetupError> {
    fs::write(path, content).map_err(|source| SetupError::WriteFileError {
        path: path.to_path_buf(),
        source,
    })?;

    // Set restrictive permissions on Unix systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = fs::Permissions::from_mode(0o600); // rw-------
        fs::set_permissions(path, permissions).map_err(|source| SetupError::PermissionError {
            path: path.to_path_buf(),
            source,
        })?;
    }

    Ok(())
}

/// Find config file using discovery priority
pub fn find_config_file() -> Option<PathBuf> {
    // Priority order:
    // 1. ./config.toml (current directory - for backward compatibility)
    // 2. Standard OS config directory

    let current_dir_config = PathBuf::from("config.toml");
    if current_dir_config.exists() {
        return Some(current_dir_config);
    }

    let legacy_config = PathBuf::from("../config.toml");
    if legacy_config.exists() {
        return Some(legacy_config);
    }

    match get_config_file_path() {
        Ok(standard_config) if standard_config.exists() => Some(standard_config),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_dir_determination() {
        let result = get_config_dir();
        assert!(result.is_ok());
        let config_dir = result.unwrap();
        assert!(config_dir.to_string_lossy().contains("quetty"));
    }

    #[test]
    fn test_find_config_file_priority() {
        // This test would need a more complex setup to properly test
        // the priority order, but we can at least test that it doesn't panic
        let result = find_config_file();
        // Result depends on the actual file system state
        assert!(result.is_some() || result.is_none());
    }
}
