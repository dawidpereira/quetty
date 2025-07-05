use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    #[serde(default = "default_method")]
    pub method: String,
}

fn default_method() -> String {
    "connection_string".to_string()
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            method: default_method(),
        }
    }
}

impl AuthConfig {
    pub fn method(&self) -> &str {
        &self.method
    }
}