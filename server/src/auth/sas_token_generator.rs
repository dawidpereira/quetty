use crate::service_bus_manager::ServiceBusError;
use base64::{Engine as _, engine::general_purpose};
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct SasTokenGenerator {
    namespace: String,
}

impl SasTokenGenerator {
    pub fn new(namespace: String) -> Self {
        Self { namespace }
    }

    /// Generate a SAS token given a key
    pub fn generate_sas_token(
        &self,
        key_name: &str,
        key: &str,
        duration_hours: i64,
    ) -> Result<String, ServiceBusError> {
        let expiry = Utc::now() + Duration::hours(duration_hours);
        let expiry_timestamp = expiry.timestamp();

        let resource_uri = format!("sb://{}.servicebus.windows.net/", self.namespace);
        let string_to_sign = format!(
            "{}\n{}",
            urlencoding::encode(&resource_uri),
            expiry_timestamp
        );

        let key_bytes = general_purpose::STANDARD.decode(key).map_err(|e| {
            ServiceBusError::AuthenticationError(format!("Failed to decode key: {e}"))
        })?;

        let mut mac = HmacSha256::new_from_slice(&key_bytes).map_err(|e| {
            ServiceBusError::AuthenticationError(format!("Failed to create HMAC: {e}"))
        })?;

        mac.update(string_to_sign.as_bytes());
        let signature = mac.finalize();
        let signature_base64 = general_purpose::STANDARD.encode(signature.into_bytes());

        let sas_token = format!(
            "SharedAccessSignature sr={}&sig={}&se={}&skn={}",
            urlencoding::encode(&resource_uri),
            urlencoding::encode(&signature_base64),
            expiry_timestamp,
            key_name
        );

        Ok(sas_token)
    }

    /// Create a connection string from a SAS token
    pub fn create_connection_string_from_sas(&self, sas_token: &str) -> String {
        format!(
            "Endpoint=sb://{}.servicebus.windows.net/;SharedAccessSignature={}",
            self.namespace, sas_token
        )
    }
}
