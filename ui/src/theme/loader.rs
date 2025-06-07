use crate::error::{AppError, AppResult};
use crate::theme::types::{Theme, ThemeConfig};
use std::{fs, path::PathBuf};

pub struct ThemeLoader {
    themes_dir: PathBuf,
}

impl ThemeLoader {
    pub fn new() -> Self {
        // Default to themes directory in the UI project root
        let themes_dir = PathBuf::from("themes");
        Self { themes_dir }
    }

    pub fn with_themes_dir<P: Into<PathBuf>>(themes_dir: P) -> Self {
        Self {
            themes_dir: themes_dir.into(),
        }
    }

    /// Load a specific theme by name and flavor
    pub fn load_theme(&self, theme_name: &str, flavor_name: &str) -> AppResult<Theme> {
        let theme_path = self
            .themes_dir
            .join(theme_name)
            .join(format!("{}.toml", flavor_name));

        if !theme_path.exists() {
            return Err(AppError::Config(format!(
                "Theme file not found: {}",
                theme_path.display()
            )));
        }

        let content = fs::read_to_string(&theme_path).map_err(|e| {
            AppError::Config(format!(
                "Failed to read theme file {}: {}",
                theme_path.display(),
                e
            ))
        })?;

        let theme: Theme = toml::from_str(&content).map_err(|e| {
            AppError::Config(format!(
                "Failed to parse theme file {}: {}",
                theme_path.display(),
                e
            ))
        })?;

        log::info!(
            "Loaded theme: {} ({}) from {}",
            theme.metadata.name,
            flavor_name,
            theme_path.display()
        );

        Ok(theme)
    }

    /// Load theme from config or fall back to default
    pub fn load_theme_from_config(&self, config: &ThemeConfig) -> AppResult<Theme> {
        match self.load_theme(&config.theme_name, &config.flavor_name) {
            Ok(theme) => Ok(theme),
            Err(e) => {
                log::warn!(
                    "Failed to load configured theme {}:{}, falling back to default: {}",
                    config.theme_name,
                    config.flavor_name,
                    e
                );
                self.load_default_theme()
            }
        }
    }

    /// Load the default dark theme
    pub fn load_default_theme(&self) -> AppResult<Theme> {
        self.load_theme("default", "dark")
    }

    /// Discover all available themes and flavors
    pub fn discover_themes(&self) -> AppResult<Vec<(String, Vec<String>)>> {
        if !self.themes_dir.exists() {
            return Ok(Vec::new());
        }

        let mut themes = Vec::new();

        let entries = fs::read_dir(&self.themes_dir).map_err(|e| {
            AppError::Config(format!(
                "Failed to read themes directory {}: {}",
                self.themes_dir.display(),
                e
            ))
        })?;

        for entry in entries {
            let entry = entry
                .map_err(|e| AppError::Config(format!("Failed to read directory entry: {}", e)))?;

            let path = entry.path();
            if path.is_dir() {
                if let Some(theme_name) = path.file_name().and_then(|n| n.to_str()) {
                    let flavors = self.discover_flavors(&path)?;
                    if !flavors.is_empty() {
                        themes.push((theme_name.to_string(), flavors));
                    }
                }
            }
        }

        Ok(themes)
    }

    /// Discover all flavors for a specific theme
    fn discover_flavors(&self, theme_dir: &PathBuf) -> AppResult<Vec<String>> {
        let mut flavors = Vec::new();

        let entries = fs::read_dir(theme_dir).map_err(|e| {
            AppError::Config(format!(
                "Failed to read theme directory {}: {}",
                theme_dir.display(),
                e
            ))
        })?;

        for entry in entries {
            let entry = entry
                .map_err(|e| AppError::Config(format!("Failed to read directory entry: {}", e)))?;

            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("toml") {
                if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                    flavors.push(file_stem.to_string());
                }
            }
        }

        flavors.sort();
        Ok(flavors)
    }

    /// Check if themes directory exists
    pub fn themes_dir_exists(&self) -> bool {
        self.themes_dir.exists() && self.themes_dir.is_dir()
    }

    /// Get the themes directory path
    pub fn themes_dir(&self) -> &PathBuf {
        &self.themes_dir
    }
}
