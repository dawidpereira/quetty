use crate::service_bus_manager::ServiceBusError;
use base64::{Engine as _, engine::general_purpose};
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Generator for Azure Service Bus Shared Access Signature (SAS) tokens.
///
/// Creates time-limited authentication tokens using HMAC-SHA256 signing with
/// shared access keys. SAS tokens provide a secure way to grant limited access
/// to Service Bus resources without sharing the primary keys.
///
/// # Security Notes
///
/// - Generated tokens have configurable expiration times
/// - Uses HMAC-SHA256 for cryptographic signing
/// - Keys are base64 decoded before use in signing
/// - Tokens include URL-encoded resource URIs for security
///
/// # Examples
///
/// ```no_run
/// use server::auth::SasTokenGenerator;
///
/// let generator = SasTokenGenerator::new("my-namespace".to_string());
/// let token = generator.generate_sas_token(
///     "RootManageSharedAccessKey",
///     "base64_encoded_key",
///     24 // 24 hours
/// )?;
/// ```
#[derive(Clone)]
pub struct SasTokenGenerator {
    namespace: String,
}

impl SasTokenGenerator {
    /// Creates a new SAS token generator for the specified Service Bus namespace.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The Service Bus namespace (without .servicebus.windows.net)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use server::auth::SasTokenGenerator;
    ///
    /// let generator = SasTokenGenerator::new("my-servicebus-namespace".to_string());
    /// ```
    pub fn new(namespace: String) -> Self {
        Self { namespace }
    }

    /// Generates a SAS token for Service Bus authentication.
    ///
    /// Creates a time-limited Shared Access Signature token using HMAC-SHA256
    /// signing with the provided shared access key. The token grants access to
    /// the entire Service Bus namespace.
    ///
    /// # Arguments
    ///
    /// * `key_name` - The name of the shared access key policy
    /// * `key` - The base64-encoded shared access key
    /// * `duration_hours` - Token validity period in hours
    ///
    /// # Returns
    ///
    /// A complete SAS token string ready for use in Service Bus operations
    ///
    /// # Errors
    ///
    /// Returns [`ServiceBusError::AuthenticationError`] if:
    /// - The key cannot be base64 decoded
    /// - HMAC generation fails
    /// - Token signing fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use server::auth::SasTokenGenerator;
    ///
    /// let generator = SasTokenGenerator::new("namespace".to_string());
    /// let token = generator.generate_sas_token(
    ///     "RootManageSharedAccessKey",
    ///     "base64_encoded_key_here",
    ///     24 // Valid for 24 hours
    /// )?;
    /// ```
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

    /// Creates a Service Bus connection string from a SAS token.
    ///
    /// Combines the namespace endpoint with the SAS token to create a complete
    /// connection string that can be used for Service Bus operations.
    ///
    /// # Arguments
    ///
    /// * `sas_token` - A valid SAS token (typically from [`generate_sas_token`])
    ///
    /// # Returns
    ///
    /// A complete Service Bus connection string
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use server::auth::SasTokenGenerator;
    ///
    /// let generator = SasTokenGenerator::new("namespace".to_string());
    /// let token = generator.generate_sas_token("key_name", "key", 24)?;
    /// let connection_string = generator.create_connection_string_from_sas(&token);
    /// ```
    pub fn create_connection_string_from_sas(&self, sas_token: &str) -> String {
        format!(
            "Endpoint=sb://{}.servicebus.windows.net/;SharedAccessSignature={}",
            self.namespace, sas_token
        )
    }
}
