use crate::error::{AppError, AppResult};
use serde::Deserialize;

/// Service Bus configuration
#[derive(Debug, Deserialize, Default)]
pub struct ServicebusConfig {
    connection_string: Option<String>,
}

impl ServicebusConfig {
    /// Get the Service Bus connection string
    pub fn connection_string(&self) -> AppResult<&str> {
        if let Some(ref conn) = self.connection_string {
            return Ok(conn);
        }

        // No fallback anymore – the connection string must be provided via `config.toml`.
        Err(AppError::Config(
            "Missing service bus connection string. Please provide it in your config.toml under [servicebus].connection_string".to_string(),
        ))
    }
}
