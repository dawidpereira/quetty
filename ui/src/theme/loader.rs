use crate::config::defaults::default_themes;
use crate::error::{AppError, AppResult};
use crate::theme::types::{Theme, ThemeConfig};
use crate::theme::validation::{FlavorNameValidator, ThemeNameValidator, ThemeValidator};
use crate::validation::Validator;
use std::{collections::HashMap, fs, path::PathBuf};

/// Theme loader responsible for loading themes from embedded assets and filesystem
pub struct ThemeLoader {
    profile_name: String,
    theme_name_validator: ThemeNameValidator,
    flavor_name_validator: FlavorNameValidator,
    theme_validator: ThemeValidator,
}

impl ThemeLoader {
    pub fn new() -> Self {
        Self::new_for_profile("default")
    }

    pub fn new_for_profile(profile_name: &str) -> Self {
        Self {
            profile_name: profile_name.to_string(),
            theme_name_validator: ThemeNameValidator,
            flavor_name_validator: FlavorNameValidator,
            theme_validator: ThemeValidator,
        }
    }

    /// Get profile-specific themes directory if it exists
    fn get_profile_themes_dir(&self) -> Option<PathBuf> {
        use crate::config::setup::get_config_dir;

        if let Ok(config_dir) = get_config_dir() {
            let profile_themes_dir = config_dir
                .join("profiles")
                .join(&self.profile_name)
                .join("themes");

            if profile_themes_dir.exists() && profile_themes_dir.is_dir() {
                log::info!(
                    "Found profile themes directory: {}",
                    profile_themes_dir.display()
                );
                return Some(profile_themes_dir);
            }
        }
        None
    }

    /// Get global themes directory if it exists
    fn get_global_themes_dir(&self) -> Option<PathBuf> {
        use crate::config::setup::get_themes_dir;

        if let Ok(global_themes_dir) = get_themes_dir() {
            if global_themes_dir.exists() && global_themes_dir.is_dir() {
                log::info!(
                    "Found global themes directory: {}",
                    global_themes_dir.display()
                );
                return Some(global_themes_dir);
            }
        }
        None
    }

    pub fn load_theme(&self, theme_name: &str, flavor_name: &str) -> AppResult<Theme> {
        // Validate inputs
        self.theme_name_validator.validate(theme_name)?;
        self.flavor_name_validator.validate(flavor_name)?;

        // Try embedded themes FIRST (primary path for cargo install)
        let embedded_theme_key = format!("{theme_name}/{flavor_name}.toml");
        let embedded_themes = default_themes();

        let theme_content = if let Some(embedded_content) =
            embedded_themes.get(embedded_theme_key.as_str())
        {
            log::debug!("Using embedded theme: {embedded_theme_key}");
            embedded_content.to_string()
        } else {
            // Try profile-specific themes first, then global themes
            let profile_theme_result = self.try_load_from_profile_themes(theme_name, flavor_name);

            match profile_theme_result {
                Ok(content) => content,
                Err(_) => {
                    // Try global themes as fallback
                    match self.try_load_from_global_themes(theme_name, flavor_name) {
                        Ok(content) => content,
                        Err(_) => {
                            return Err(AppError::Config(format!(
                                "Theme '{}' not found in embedded themes, profile themes, or global themes.\n\
                                Available embedded themes: {}",
                                embedded_theme_key,
                                embedded_themes
                                    .keys()
                                    .cloned()
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )));
                        }
                    }
                }
            }
        };

        let mut theme: Theme = toml::from_str(&theme_content).map_err(|e| {
            AppError::Config(format!("Failed to parse theme '{embedded_theme_key}': {e}"))
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
        let mut themes = Vec::new();

        // Discover embedded themes first
        let embedded_themes = default_themes();
        let mut embedded_theme_map: HashMap<String, Vec<String>> = HashMap::new();

        for theme_key in embedded_themes.keys() {
            if let Some((theme_name, flavor_file)) = theme_key.split_once('/') {
                if let Some(flavor_name) = flavor_file.strip_suffix(".toml") {
                    embedded_theme_map
                        .entry(theme_name.to_string())
                        .or_default()
                        .push(flavor_name.to_string());
                }
            }
        }

        // Add embedded themes to the result
        for (theme_name, mut flavors) in embedded_theme_map {
            flavors.sort();
            themes.push((theme_name, flavors));
        }

        // Discover profile themes if directory exists
        if let Some(profile_themes_dir) = self.get_profile_themes_dir() {
            self.discover_themes_from_directory(&profile_themes_dir, &mut themes, "profile")?;
        }

        // Discover global themes if directory exists
        if let Some(global_themes_dir) = self.get_global_themes_dir() {
            self.discover_themes_from_directory(&global_themes_dir, &mut themes, "global")?;
        }

        themes.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(themes)
    }

    /// Try to load theme from profile-specific themes directory
    fn try_load_from_profile_themes(
        &self,
        theme_name: &str,
        flavor_name: &str,
    ) -> AppResult<String> {
        if let Some(profile_themes_dir) = self.get_profile_themes_dir() {
            let theme_path = profile_themes_dir
                .join(theme_name)
                .join(format!("{flavor_name}.toml"));

            match fs::read_to_string(&theme_path) {
                Ok(content) => {
                    log::info!(
                        "Loaded theme from profile directory: {}",
                        theme_path.display()
                    );
                    return Ok(content);
                }
                Err(e) => {
                    log::debug!(
                        "Failed to load theme from profile directory {}: {}",
                        theme_path.display(),
                        e
                    );
                }
            }
        }

        Err(AppError::Config(
            "Theme not found in profile directory".to_string(),
        ))
    }

    /// Try to load theme from global themes directory
    fn try_load_from_global_themes(
        &self,
        theme_name: &str,
        flavor_name: &str,
    ) -> AppResult<String> {
        if let Some(global_themes_dir) = self.get_global_themes_dir() {
            let theme_path = global_themes_dir
                .join(theme_name)
                .join(format!("{flavor_name}.toml"));

            match fs::read_to_string(&theme_path) {
                Ok(content) => {
                    log::info!(
                        "Loaded theme from global directory: {}",
                        theme_path.display()
                    );
                    return Ok(content);
                }
                Err(e) => {
                    log::debug!(
                        "Failed to load theme from global directory {}: {}",
                        theme_path.display(),
                        e
                    );
                }
            }
        }

        Err(AppError::Config(
            "Theme not found in global directory".to_string(),
        ))
    }

    /// Discover themes from a specific directory and merge them into the themes list
    fn discover_themes_from_directory(
        &self,
        themes_dir: &PathBuf,
        themes: &mut Vec<(String, Vec<String>)>,
        dir_type: &str,
    ) -> AppResult<()> {
        let entries = fs::read_dir(themes_dir).map_err(|e| {
            AppError::Config(format!(
                "Failed to read {} themes directory '{}': {}",
                dir_type,
                themes_dir.display(),
                e
            ))
        })?;

        for entry in entries {
            let entry = entry
                .map_err(|e| AppError::Config(format!("Failed to read directory entry: {e}")))?;

            let path = entry.path();
            if path.is_dir() {
                if let Some(theme_name) = path.file_name().and_then(|n| n.to_str()) {
                    // Validate theme name
                    if self.theme_name_validator.validate(theme_name).is_ok() {
                        let flavors = self.discover_flavors(&path)?;
                        if !flavors.is_empty() {
                            // Check if we already have this theme from embedded or other sources
                            if let Some(existing_theme) =
                                themes.iter_mut().find(|(name, _)| name == theme_name)
                            {
                                // Merge filesystem flavors with existing ones
                                for flavor in flavors {
                                    if !existing_theme.1.contains(&flavor) {
                                        existing_theme.1.push(flavor);
                                    }
                                }
                                existing_theme.1.sort();
                                log::debug!("Merged {dir_type} flavors for theme '{theme_name}'");
                            } else {
                                // Add new filesystem theme
                                themes.push((theme_name.to_string(), flavors));
                                log::debug!("Added new {dir_type} theme '{theme_name}'");
                            }
                        }
                    }
                }
            }
        }

        Ok(())
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
                .map_err(|e| AppError::Config(format!("Failed to read directory entry: {e}")))?;

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
            let error_msg = format!("{error}");
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
                log::info!("Successfully loaded default theme: {}", theme.metadata.name);
                assert_eq!(theme.metadata.theme_name, Some("quetty".to_string()));
                assert_eq!(theme.metadata.flavor_name, Some("dark".to_string()));
            }
            Err(e) => {
                log::error!("Failed to load default theme: {e}");
                // This will show us what the exact error is
                panic!("Default theme should be loadable: {e}");
            }
        }
    }
}
