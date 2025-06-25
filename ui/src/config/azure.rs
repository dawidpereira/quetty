use crate::error::{AppError, AppResult};
use serde::Deserialize;

/// Service Bus configuration
#[derive(Debug, Deserialize)]
pub struct ServicebusConfig {
    connection_string: Option<String>,
}

impl ServicebusConfig {
    /// Get the Service Bus connection string
    pub fn connection_string(&self) -> AppResult<&str> {
        self.connection_string
            .as_deref()
            .ok_or_else(|| AppError::Config("Missing service bus connection string".to_string()))
    }
}
