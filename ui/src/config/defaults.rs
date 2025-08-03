use std::collections::HashMap;

/// Default base configuration file embedded in the binary
pub const DEFAULT_CONFIG: &str = include_str!("../../../config.default.toml");

/// Default key bindings configuration file embedded in the binary
pub const DEFAULT_KEYS: &str = include_str!("../../../keys.default.toml");

/// Default theme files embedded in the binary
pub fn default_themes() -> HashMap<&'static str, &'static str> {
    let mut themes = HashMap::new();
    themes.insert(
        "quetty/dark.toml",
        include_str!("../../themes/quetty/dark.toml"),
    );
    themes.insert(
        "quetty/light.toml",
        include_str!("../../themes/quetty/light.toml"),
    );

    // Add other theme directories
    themes.insert(
        "catppuccin/frappe.toml",
        include_str!("../../themes/catppuccin/frappe.toml"),
    );
    themes.insert(
        "catppuccin/latte.toml",
        include_str!("../../themes/catppuccin/latte.toml"),
    );
    themes.insert(
        "catppuccin/macchiato.toml",
        include_str!("../../themes/catppuccin/macchiato.toml"),
    );
    themes.insert(
        "catppuccin/mocha.toml",
        include_str!("../../themes/catppuccin/mocha.toml"),
    );

    themes.insert(
        "nightfox/carbonfox.toml",
        include_str!("../../themes/nightfox/carbonfox.toml"),
    );
    themes.insert(
        "nightfox/dawnfox.toml",
        include_str!("../../themes/nightfox/dawnfox.toml"),
    );
    themes.insert(
        "nightfox/duskfox.toml",
        include_str!("../../themes/nightfox/duskfox.toml"),
    );
    themes.insert(
        "nightfox/nightfox.toml",
        include_str!("../../themes/nightfox/nightfox.toml"),
    );
    themes.insert(
        "nightfox/nordfox.toml",
        include_str!("../../themes/nightfox/nordfox.toml"),
    );
    themes.insert(
        "nightfox/terafox.toml",
        include_str!("../../themes/nightfox/terafox.toml"),
    );

    themes
}

/// Get complete default configuration by merging base config and keys
pub fn get_complete_default_config() -> String {
    let complete_config = DEFAULT_CONFIG.to_string();

    // Remove the environment variable reference section from base config
    // and append the keys configuration
    if let Some(env_section_start) = complete_config.find("# =============================================================================\n# ENVIRONMENT VARIABLE REFERENCE") {
        let before_env = &complete_config[..env_section_start];
        let env_section = &complete_config[env_section_start..];

        // Extract the keys content (skip the header comments)
        let keys_content = DEFAULT_KEYS
            .lines()
            .skip_while(|line| line.starts_with('#') || line.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        format!("{before_env}\n{keys_content}\n\n{env_section}")
    } else {
        // Fallback: just append keys if we can't find the env section
        format!("{complete_config}\n\n{DEFAULT_KEYS}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_default_config_is_not_empty() {
        assert!(!DEFAULT_CONFIG.is_empty());
        assert!(DEFAULT_CONFIG.contains("[azure_ad]"));
    }

    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_default_keys_is_not_empty() {
        assert!(!DEFAULT_KEYS.is_empty());
        assert!(DEFAULT_KEYS.contains("[keys]"));
        assert!(DEFAULT_KEYS.contains("key_quit"));
    }

    #[test]
    fn test_complete_default_config() {
        let complete = get_complete_default_config();
        assert!(!complete.is_empty());
        assert!(complete.contains("[azure_ad]"));
        assert!(complete.contains("[keys]"));
        assert!(complete.contains("key_quit"));
    }

    #[test]
    fn test_default_themes_available() {
        let themes = default_themes();
        assert!(!themes.is_empty());
        assert!(themes.contains_key("quetty/dark.toml"));
        assert!(themes.contains_key("quetty/light.toml"));

        // Verify theme content is not empty
        for (name, content) in themes.iter() {
            assert!(!content.is_empty(), "Theme {name} should not be empty");
        }
    }
}
