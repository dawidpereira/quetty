use crate::config::AppConfig;

/// Authentication method constants
pub const AUTH_METHOD_CONNECTION_STRING: &str = "connection_string";
pub const AUTH_METHOD_DEVICE_CODE: &str = "device_code";

/// Utility functions for authentication method checking
pub struct AuthUtils;

impl AuthUtils {
    /// Check if the configuration is using connection string authentication
    pub fn is_connection_string_auth(config: &AppConfig) -> bool {
        config.azure_ad().auth_method == AUTH_METHOD_CONNECTION_STRING
    }

    /// Check if the configuration is using device code authentication
    pub fn is_device_code_auth(config: &AppConfig) -> bool {
        config.azure_ad().auth_method == AUTH_METHOD_DEVICE_CODE
    }

    /// Check if the authentication method requires Azure AD
    pub fn requires_azure_ad(config: &AppConfig) -> bool {
        !Self::is_connection_string_auth(config)
    }

    /// Check if the authentication method supports automatic discovery
    pub fn supports_discovery(config: &AppConfig) -> bool {
        Self::requires_azure_ad(config)
    }

    /// Get a human-readable description of the authentication method
    pub fn auth_method_description(config: &AppConfig) -> &'static str {
        match config.azure_ad().auth_method.as_str() {
            AUTH_METHOD_CONNECTION_STRING => "Connection String Authentication",
            AUTH_METHOD_DEVICE_CODE => "Azure AD Device Code Flow",
            _ => "Unknown Authentication Method",
        }
    }
}
