use serde::Deserialize;

/// Service Bus configuration
#[derive(Debug, Deserialize, Default)]
pub struct ServicebusConfig {
    connection_string: Option<String>,
}

impl ServicebusConfig {
    /// Get the Service Bus connection string if available
    pub fn connection_string(&self) -> Option<&str> {
        self.connection_string.as_deref()
    }
}
