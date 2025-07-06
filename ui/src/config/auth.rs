use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct AuthConfig {
    #[serde(default = "default_primary_method")]
    pub primary_method: String,
    #[serde(default = "default_fallback_enabled")]
    pub fallback_enabled: bool,
}

fn default_primary_method() -> String {
    "connection_string".to_string()
}

fn default_fallback_enabled() -> bool {
    true
}

impl AuthConfig {
    #[allow(dead_code)]
    pub fn primary_method(&self) -> &str {
        &self.primary_method
    }

    #[allow(dead_code)]
    pub fn fallback_enabled(&self) -> bool {
        self.fallback_enabled
    }
}
