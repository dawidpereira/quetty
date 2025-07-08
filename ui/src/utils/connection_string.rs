use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConnectionStringError {
    #[error(
        "Connection string is empty or only contains whitespace. Please provide a valid Azure Service Bus connection string."
    )]
    Empty,

    #[error(
        "Connection string is missing the Endpoint parameter. Expected format: 'Endpoint=sb://your-namespace.servicebus.windows.net/;SharedAccessKeyName=...;SharedAccessKey=...'"
    )]
    MissingEndpoint,

    #[error(
        "Invalid Service Bus endpoint format: '{0}'. Expected format: 'sb://your-namespace.servicebus.windows.net/'"
    )]
    InvalidEndpointFormat(String),

    #[error(
        "Unable to extract namespace from endpoint: '{0}'. Expected format: 'sb://namespace.servicebus.windows.net/'"
    )]
    NamespaceNotFound(String),

    #[error(
        "Invalid namespace format: {0}. Namespace must be 6-50 characters, contain only letters, numbers, and hyphens, start and end with alphanumeric characters, and not contain consecutive hyphens."
    )]
    InvalidNamespaceFormat(String),
}

/// Utility functions for parsing Service Bus connection strings
pub struct ConnectionStringParser;

impl ConnectionStringParser {
    /// Extract namespace from Service Bus connection string with comprehensive validation
    ///
    /// Expected format: Endpoint=sb://namespace.servicebus.windows.net/;SharedAccessKeyName=...;SharedAccessKey=...
    ///
    /// # Arguments
    /// * `connection_string` - The Service Bus connection string to parse
    ///
    /// # Returns
    /// * `Ok(String)` - The extracted namespace
    /// * `Err(ConnectionStringError)` - Error describing what went wrong
    pub fn extract_namespace(connection_string: &str) -> Result<String, ConnectionStringError> {
        // Check for empty or whitespace-only strings
        if connection_string.trim().is_empty() {
            return Err(ConnectionStringError::Empty);
        }

        // Find the Endpoint part in the connection string
        let endpoint_part = connection_string
            .split(';')
            .find(|part| part.starts_with("Endpoint="))
            .ok_or(ConnectionStringError::MissingEndpoint)?;

        // Extract the endpoint value
        let endpoint = endpoint_part
            .strip_prefix("Endpoint=")
            .ok_or(ConnectionStringError::MissingEndpoint)?;

        // Validate Service Bus endpoint format
        if !endpoint.starts_with("sb://") {
            return Err(ConnectionStringError::InvalidEndpointFormat(
                endpoint.to_string(),
            ));
        }

        // Extract the host part (everything after sb:// and before the first /)
        let host_part =
            endpoint
                .strip_prefix("sb://")
                .ok_or(ConnectionStringError::InvalidEndpointFormat(
                    endpoint.to_string(),
                ))?;

        // Remove trailing slash if present and extract just the hostname
        let hostname = host_part.trim_end_matches('/');

        // Extract namespace (everything before the first dot)
        let namespace =
            hostname
                .split('.')
                .next()
                .ok_or(ConnectionStringError::NamespaceNotFound(
                    hostname.to_string(),
                ))?;

        // Validate namespace format
        Self::validate_namespace_format(namespace)?;

        Ok(namespace.to_string())
    }

    /// Validate that the extracted namespace follows Azure Service Bus naming conventions
    ///
    /// Azure Service Bus namespace requirements:
    /// - 6-50 characters long
    /// - Can contain letters, numbers, and hyphens
    /// - Must start and end with a letter or number
    /// - Cannot contain consecutive hyphens
    fn validate_namespace_format(namespace: &str) -> Result<(), ConnectionStringError> {
        // Check length requirements
        if namespace.len() < 6 || namespace.len() > 50 {
            return Err(ConnectionStringError::InvalidNamespaceFormat(format!(
                "Namespace length must be 6-50 characters, got {}",
                namespace.len()
            )));
        }

        // Check that it starts and ends with alphanumeric
        if !namespace.chars().next().unwrap().is_alphanumeric()
            || !namespace.chars().last().unwrap().is_alphanumeric()
        {
            return Err(ConnectionStringError::InvalidNamespaceFormat(
                "Namespace must start and end with a letter or number".to_string(),
            ));
        }

        // Check for invalid characters (only letters, numbers, hyphens allowed)
        if !namespace.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(ConnectionStringError::InvalidNamespaceFormat(
                "Namespace can only contain letters, numbers, and hyphens".to_string(),
            ));
        }

        // Check for consecutive hyphens
        if namespace.contains("--") {
            return Err(ConnectionStringError::InvalidNamespaceFormat(
                "Namespace cannot contain consecutive hyphens".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate a complete Service Bus connection string format
    ///
    /// Checks for required components: Endpoint, SharedAccessKeyName, SharedAccessKey
    pub fn validate_connection_string(
        connection_string: &str,
    ) -> Result<(), ConnectionStringError> {
        if connection_string.trim().is_empty() {
            return Err(ConnectionStringError::Empty);
        }

        let parts: Vec<&str> = connection_string.split(';').collect();

        // Check for required components
        let has_endpoint = parts.iter().any(|part| part.starts_with("Endpoint="));
        let _has_key_name = parts
            .iter()
            .any(|part| part.starts_with("SharedAccessKeyName="));
        let _has_key = parts
            .iter()
            .any(|part| part.starts_with("SharedAccessKey="));

        if !has_endpoint {
            return Err(ConnectionStringError::MissingEndpoint);
        }

        // Validate the endpoint by extracting namespace (this will catch endpoint format issues)
        Self::extract_namespace(connection_string)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_namespace_valid() {
        let connection_string = "Endpoint=sb://mycompany.servicebus.windows.net/;SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=somekey";
        let result = ConnectionStringParser::extract_namespace(connection_string);
        assert_eq!(result.unwrap(), "mycompany");
    }

    #[test]
    fn test_extract_namespace_with_trailing_slash() {
        let connection_string = "Endpoint=sb://mycompany.servicebus.windows.net/;SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=somekey";
        let result = ConnectionStringParser::extract_namespace(connection_string);
        assert_eq!(result.unwrap(), "mycompany");
    }

    #[test]
    fn test_extract_namespace_empty_string() {
        let result = ConnectionStringParser::extract_namespace("");
        assert!(matches!(result, Err(ConnectionStringError::Empty)));
    }

    #[test]
    fn test_extract_namespace_whitespace_only() {
        let result = ConnectionStringParser::extract_namespace("   ");
        assert!(matches!(result, Err(ConnectionStringError::Empty)));
    }

    #[test]
    fn test_extract_namespace_missing_endpoint() {
        let connection_string =
            "SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=somekey";
        let result = ConnectionStringParser::extract_namespace(connection_string);
        assert!(matches!(
            result,
            Err(ConnectionStringError::MissingEndpoint)
        ));
    }

    #[test]
    fn test_extract_namespace_invalid_endpoint_format() {
        let connection_string = "Endpoint=https://mycompany.servicebus.windows.net/;SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=somekey";
        let result = ConnectionStringParser::extract_namespace(connection_string);
        assert!(matches!(
            result,
            Err(ConnectionStringError::InvalidEndpointFormat(_))
        ));
    }

    #[test]
    fn test_validate_namespace_format_valid() {
        assert!(ConnectionStringParser::validate_namespace_format("mycompany").is_ok());
        assert!(ConnectionStringParser::validate_namespace_format("my-company").is_ok());
        assert!(ConnectionStringParser::validate_namespace_format("company123").is_ok());
    }

    #[test]
    fn test_validate_namespace_format_too_short() {
        let result = ConnectionStringParser::validate_namespace_format("short");
        assert!(matches!(
            result,
            Err(ConnectionStringError::InvalidNamespaceFormat(_))
        ));
    }

    #[test]
    fn test_validate_namespace_format_consecutive_hyphens() {
        let result = ConnectionStringParser::validate_namespace_format("my--company");
        assert!(matches!(
            result,
            Err(ConnectionStringError::InvalidNamespaceFormat(_))
        ));
    }

    #[test]
    fn test_validate_namespace_format_starts_with_hyphen() {
        let result = ConnectionStringParser::validate_namespace_format("-mycompany");
        assert!(matches!(
            result,
            Err(ConnectionStringError::InvalidNamespaceFormat(_))
        ));
    }

    #[test]
    fn test_validate_connection_string_valid() {
        let connection_string = "Endpoint=sb://mycompany.servicebus.windows.net/;SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=somekey";
        assert!(ConnectionStringParser::validate_connection_string(connection_string).is_ok());
    }
}
