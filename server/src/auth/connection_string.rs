use super::provider::{AuthProvider, AuthToken};
use super::sas_token_generator::SasTokenGenerator;
use super::types::{AuthType, ConnectionStringConfig};
use crate::service_bus_manager::ServiceBusError;
use async_trait::async_trait;

#[derive(Clone)]
pub struct ConnectionStringProvider {
    config: ConnectionStringConfig,
    key_name: String,
    key: String,
    sas_generator: SasTokenGenerator,
}

impl ConnectionStringProvider {
    pub fn new(config: ConnectionStringConfig) -> Result<Self, ServiceBusError> {
        if config.value.is_empty() {
            return Err(ServiceBusError::ConfigurationError(
                "Connection string cannot be empty".to_string(),
            ));
        }

        // Parse connection string to extract components
        let mut namespace = None;
        let mut key_name = None;
        let mut key = None;

        for part in config.value.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            if let Some(endpoint) = part.strip_prefix("Endpoint=") {
                // Extract namespace from endpoint like "sb://namespace.servicebus.windows.net/"
                if let Some(ns_start) = endpoint.find("://") {
                    let ns_part = &endpoint[ns_start + 3..];
                    if let Some(dot_pos) = ns_part.find('.') {
                        namespace = Some(ns_part[..dot_pos].to_string());
                    }
                }
            } else if let Some(kn) = part.strip_prefix("SharedAccessKeyName=") {
                key_name = Some(kn.to_string());
            } else if let Some(k) = part.strip_prefix("SharedAccessKey=") {
                key = Some(k.to_string());
            }
        }

        let namespace = namespace.ok_or_else(|| {
            ServiceBusError::ConfigurationError(
                "Missing namespace in connection string".to_string(),
            )
        })?;
        let key_name = key_name.ok_or_else(|| {
            ServiceBusError::ConfigurationError(
                "Missing SharedAccessKeyName in connection string".to_string(),
            )
        })?;
        let key = key.ok_or_else(|| {
            ServiceBusError::ConfigurationError(
                "Missing SharedAccessKey in connection string".to_string(),
            )
        })?;

        let sas_generator = SasTokenGenerator::new(namespace.clone());

        Ok(Self {
            config,
            key_name,
            key,
            sas_generator,
        })
    }

    pub fn connection_string(&self) -> &str {
        &self.config.value
    }
}

#[async_trait]
impl AuthProvider for ConnectionStringProvider {
    async fn authenticate(&self) -> Result<AuthToken, ServiceBusError> {
        // Generate a SAS token valid for 24 hours
        let sas_token = self.sas_generator.generate_sas_token(
            &self.key_name,
            &self.key,
            24, // 24 hours validity
        )?;

        // Create a connection string with the SAS token
        let connection_string = self
            .sas_generator
            .create_connection_string_from_sas(&sas_token);

        Ok(AuthToken {
            token: connection_string,
            token_type: "ConnectionString".to_string(),
            expires_in_secs: Some(24 * 3600), // 24 hours in seconds
        })
    }

    fn auth_type(&self) -> AuthType {
        AuthType::ConnectionString
    }

    fn requires_refresh(&self) -> bool {
        true // SAS tokens expire, so we need refresh
    }
}
