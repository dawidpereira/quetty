use crate::utils::encryption::{ConnectionStringEncryption, EncryptionError};
use serde::Deserialize;
use std::sync::{Arc, Mutex};

/// Service Bus configuration
#[derive(Debug, Deserialize, Default, Clone)]
pub struct ServicebusConfig {
    encrypted_connection_string: Option<String>,
    encryption_salt: Option<String>,
}

/// Thread-safe password storage for runtime decryption
static MASTER_PASSWORD: std::sync::OnceLock<Arc<Mutex<Option<String>>>> =
    std::sync::OnceLock::new();

impl ServicebusConfig {
    /// Get the encrypted Service Bus connection string if available
    pub fn encrypted_connection_string(&self) -> Option<&str> {
        self.encrypted_connection_string
            .as_deref()
            .filter(|s| !s.trim().is_empty())
    }

    /// Get the encryption salt if available
    pub fn encryption_salt(&self) -> Option<&str> {
        self.encryption_salt
            .as_deref()
            .filter(|s| !s.trim().is_empty())
    }

    /// Decrypt and get the Service Bus connection string
    /// Returns None if no encrypted connection string is configured
    /// Returns Err if decryption fails or password is not set
    pub fn connection_string(&self) -> Result<Option<String>, EncryptionError> {
        let encrypted = match self.encrypted_connection_string() {
            Some(enc) => enc,
            None => return Ok(None),
        };

        let salt = match self.encryption_salt() {
            Some(s) => s,
            None => {
                return Err(EncryptionError::InvalidData(
                    "Encryption salt not found in configuration".to_string(),
                ));
            }
        };

        let password = get_master_password().ok_or_else(|| {
            EncryptionError::DecryptionFailed(
                "Master password not set. Please set password first.".to_string(),
            )
        })?;

        let encryption = ConnectionStringEncryption::from_salt_base64(salt)?;
        let decrypted = encryption.decrypt_connection_string(encrypted, &password)?;

        Ok(Some(decrypted))
    }

    /// Check if a connection string is configured (encrypted)
    pub fn has_connection_string(&self) -> bool {
        self.encrypted_connection_string().is_some() && self.encryption_salt().is_some()
    }
}

/// Set the master password for decryption
pub fn set_master_password(password: String) {
    let password_storage = MASTER_PASSWORD.get_or_init(|| Arc::new(Mutex::new(None)));
    if let Ok(mut guard) = password_storage.lock() {
        *guard = Some(password);
    }
}

/// Get the master password for decryption
pub fn get_master_password() -> Option<String> {
    let password_storage = MASTER_PASSWORD.get_or_init(|| Arc::new(Mutex::new(None)));
    if let Ok(guard) = password_storage.lock() {
        guard.clone()
    } else {
        None
    }
}

/// Check if master password is set
pub fn is_master_password_set() -> bool {
    get_master_password().is_some()
}

pub fn clear_master_password() {
    let password_storage = MASTER_PASSWORD.get_or_init(|| Arc::new(Mutex::new(None)));
    if let Ok(mut guard) = password_storage.lock() {
        *guard = None;
    }
}
