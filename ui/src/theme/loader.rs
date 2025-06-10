use crate::error::{AppError, AppResult};
use crate::theme::types::{Theme, ThemeConfig};
use crate::theme::validation::{
    FlavorNameValidator, ThemeNameValidator, ThemePathValidator, ThemeValidator,
};
use crate::validation::Validator;
use std::{fs, path::PathBuf};

/// Theme loader responsible for loading themes from the filesystem
pub struct ThemeLoader {
    themes_dir: PathBuf,
    theme_name_validator: ThemeNameValidator,
    flavor_name_validator: FlavorNameValidator,
    path_validator: ThemePathValidator,
    theme_validator: ThemeValidator,
}

impl ThemeLoader {
    pub fn new() -> Self {
        // Try to find themes directory in several possible locations
        let themes_dir = Self::find_themes_directory();

        Self {
            themes_dir,
            theme_name_validator: ThemeNameValidator,
            flavor_name_validator: FlavorNameValidator,
            path_validator: ThemePathValidator,
            theme_validator: ThemeValidator,
        }
    }

    fn find_themes_directory() -> PathBuf {
        // Try different possible locations for the themes directory
        let possible_paths = vec![
            PathBuf::from("themes"),      // Current directory
            PathBuf::from("ui/themes"),   // From project root
            PathBuf::from("../themes"),   // From target directory
            PathBuf::from("./ui/themes"), // From project root with ./
        ];

        for path in possible_paths {
            if path.exists() && path.is_dir() {
                log::info!("Found themes directory at: {}", path.display());
                return path;
            }
        }

        // Default fallback - this will likely fail but at least give a clear error
        log::warn!(
            "Could not find themes directory in any expected location, using default 'themes'"
        );
        PathBuf::from("themes")
    }

    pub fn load_theme(&self, theme_name: &str, flavor_name: &str) -> AppResult<Theme> {
        // Validate inputs
        self.theme_name_validator.validate(theme_name)?;
        self.flavor_name_validator.validate(flavor_name)?;

        let theme_path = self
            .themes_dir
            .join(theme_name)
            .join(format!("{}.toml", flavor_name));

        // Validate path
        self.path_validator.validate(&theme_path)?;

        // Load and parse the theme file
        let theme_content = fs::read_to_string(&theme_path).map_err(|e| {
            AppError::Config(format!(
                "Failed to read theme file '{}': {}",
                theme_path.display(),
                e
            ))
        })?;

        let mut theme: Theme = toml::from_str(&theme_content).map_err(|e| {
            AppError::Config(format!(
                "Failed to parse theme file '{}': {}",
                theme_path.display(),
                e
            ))
        })?;

        // Set metadata if not present
        if theme.metadata.name.is_empty() {
            theme.metadata.name = theme_name.to_string();
        }
        if theme.metadata.flavor_name.is_none() {
            theme.metadata.flavor_name = Some(flavor_name.to_string());
        }

        // Validate the loaded theme
        self.theme_validator.validate(&theme)?;

        Ok(theme)
    }

    pub fn load_theme_from_config(&self, config: &ThemeConfig) -> AppResult<Theme> {
        self.load_theme(&config.theme_name, &config.flavor_name)
    }

    pub fn discover_themes(&self) -> AppResult<Vec<(String, Vec<String>)>> {
        if !self.themes_dir.exists() {
            return Ok(vec![]);
        }

        let mut themes = Vec::new();
        let entries = fs::read_dir(&self.themes_dir).map_err(|e| {
            AppError::Config(format!(
                "Failed to read themes directory '{}': {}",
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
                    // Validate theme name
                    if self.theme_name_validator.validate(theme_name).is_ok() {
                        let flavors = self.discover_flavors(&path)?;
                        if !flavors.is_empty() {
                            themes.push((theme_name.to_string(), flavors));
                        }
                    }
                }
            }
        }

        Ok(themes)
    }

    fn discover_flavors(&self, theme_dir: &PathBuf) -> AppResult<Vec<String>> {
        let mut flavors = Vec::new();
        let entries = fs::read_dir(theme_dir).map_err(|e| {
            AppError::Config(format!(
                "Failed to read theme directory '{}': {}",
                theme_dir.display(),
                e
            ))
        })?;

        for entry in entries {
            let entry = entry
                .map_err(|e| AppError::Config(format!("Failed to read directory entry: {}", e)))?;

            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("toml") {
                if let Some(flavor_name) = path.file_stem().and_then(|n| n.to_str()) {
                    // Validate flavor name
                    if self.flavor_name_validator.validate(flavor_name).is_ok() {
                        flavors.push(flavor_name.to_string());
                    }
                }
            }
        }

        flavors.sort();
        Ok(flavors)
    }
}

impl Default for ThemeLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_error_simulation() {
        let loader = ThemeLoader::new();

        // Test loading a non-existent theme - this should fail gracefully
        let result = loader.load_theme("nonexistent_theme", "nonexistent_flavor");
        assert!(result.is_err());

        // Verify the error contains useful information
        if let Err(error) = result {
            let error_msg = format!("{}", error);
            assert!(error_msg.contains("nonexistent_theme") || error_msg.contains("Invalid"));
        }
    }

    #[test]
    fn test_load_default_theme() {
        let loader = ThemeLoader::new();
        let default_config = ThemeConfig::default();

        // Test loading the default theme
        let result = loader.load_theme_from_config(&default_config);

        match result {
            Ok(theme) => {
                println!("Successfully loaded default theme: {}", theme.metadata.name);
                assert_eq!(theme.metadata.theme_name, Some("quetty".to_string()));
                assert_eq!(theme.metadata.flavor_name, Some("dark".to_string()));
            }
            Err(e) => {
                println!("Failed to load default theme: {}", e);
                // This will show us what the exact error is
                panic!("Default theme should be loadable: {}", e);
            }
        }
    }
}
