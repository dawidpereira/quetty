use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, KeyInit},
};
use base64::{Engine as _, engine::general_purpose};
use pbkdf2::pbkdf2_hmac;
use rand::{RngCore, rngs::OsRng};
use sha2::Sha256;
use std::fmt;
use zeroize::ZeroizeOnDrop;

const PBKDF2_ITERATIONS: u32 = 100_000;
const SALT_LENGTH: usize = 32;
const KEY_LENGTH: usize = 32;
const NONCE_LENGTH: usize = 12;

// Error messages
const ERROR_EMPTY_CONNECTION_STRING: &str = "Connection string cannot be empty";
const ERROR_EMPTY_CLIENT_SECRET: &str = "Client secret cannot be empty";
const ERROR_EMPTY_PASSWORD: &str = "Password cannot be empty";
const ERROR_EMPTY_ENCRYPTED_DATA: &str = "Encrypted data cannot be empty";
const ERROR_ENCRYPTED_DATA_TOO_SHORT: &str = "Encrypted data too short";

#[derive(Debug)]
pub enum EncryptionError {
    InvalidData(String),
    EncryptionFailed(String),
    DecryptionFailed(String),
    KeyDerivation(String),
}

impl fmt::Display for EncryptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncryptionError::InvalidData(msg) => write!(f, "Invalid data: {msg}"),
            EncryptionError::EncryptionFailed(msg) => write!(f, "Encryption failed: {msg}"),
            EncryptionError::DecryptionFailed(msg) => write!(f, "Decryption failed: {msg}"),
            EncryptionError::KeyDerivation(msg) => write!(f, "Key derivation failed: {msg}"),
        }
    }
}

impl std::error::Error for EncryptionError {}

#[derive(ZeroizeOnDrop)]
struct SecureKey([u8; KEY_LENGTH]);

impl SecureKey {
    fn new(key: [u8; KEY_LENGTH]) -> Self {
        Self(key)
    }

    fn as_bytes(&self) -> &[u8; KEY_LENGTH] {
        &self.0
    }
}

/// Common encryption implementation for AES-256-GCM with PBKDF2 key derivation
pub struct AesEncryption {
    salt: [u8; SALT_LENGTH],
}

impl AesEncryption {
    pub fn new() -> Self {
        let mut salt = [0u8; SALT_LENGTH];
        OsRng.fill_bytes(&mut salt);
        Self { salt }
    }

    pub fn with_salt(salt: [u8; SALT_LENGTH]) -> Self {
        Self { salt }
    }

    pub fn salt_base64(&self) -> String {
        general_purpose::STANDARD.encode(self.salt)
    }

    pub fn from_salt_base64(salt_b64: &str) -> Result<Self, EncryptionError> {
        let salt_bytes = general_purpose::STANDARD
            .decode(salt_b64)
            .map_err(|e| EncryptionError::InvalidData(format!("Invalid salt base64: {e}")))?;

        if salt_bytes.len() != SALT_LENGTH {
            return Err(EncryptionError::InvalidData(format!(
                "Salt length must be {} bytes, got {}",
                SALT_LENGTH,
                salt_bytes.len()
            )));
        }

        let mut salt = [0u8; SALT_LENGTH];
        salt.copy_from_slice(&salt_bytes);
        Ok(Self::with_salt(salt))
    }

    fn derive_key(&self, password: &str) -> Result<SecureKey, EncryptionError> {
        let mut key = [0u8; KEY_LENGTH];
        pbkdf2_hmac::<Sha256>(password.as_bytes(), &self.salt, PBKDF2_ITERATIONS, &mut key);
        Ok(SecureKey::new(key))
    }

    pub fn encrypt(
        &self,
        plaintext: &str,
        password: &str,
        empty_error: &str,
    ) -> Result<String, EncryptionError> {
        if plaintext.trim().is_empty() {
            return Err(EncryptionError::InvalidData(empty_error.to_string()));
        }

        if password.trim().is_empty() {
            return Err(EncryptionError::InvalidData(
                ERROR_EMPTY_PASSWORD.to_string(),
            ));
        }

        let key = self.derive_key(password)?;

        let cipher = Aes256Gcm::new_from_slice(key.as_bytes())
            .map_err(|e| EncryptionError::KeyDerivation(format!("Invalid key: {e}")))?;

        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let ciphertext = cipher.encrypt(&nonce, plaintext.as_bytes()).map_err(|e| {
            EncryptionError::EncryptionFailed(format!("AES-GCM encryption failed: {e}"))
        })?;

        // Format: nonce + ciphertext, all base64 encoded
        let mut combined = Vec::with_capacity(NONCE_LENGTH + ciphertext.len());
        combined.extend_from_slice(&nonce);
        combined.extend_from_slice(&ciphertext);

        Ok(general_purpose::STANDARD.encode(combined))
    }

    pub fn decrypt(&self, encrypted: &str, password: &str) -> Result<String, EncryptionError> {
        if encrypted.trim().is_empty() {
            return Err(EncryptionError::InvalidData(
                ERROR_EMPTY_ENCRYPTED_DATA.to_string(),
            ));
        }

        if password.trim().is_empty() {
            return Err(EncryptionError::InvalidData(
                ERROR_EMPTY_PASSWORD.to_string(),
            ));
        }

        let combined = general_purpose::STANDARD
            .decode(encrypted)
            .map_err(|e| EncryptionError::InvalidData(format!("Invalid base64: {e}")))?;

        if combined.len() < NONCE_LENGTH {
            return Err(EncryptionError::InvalidData(
                ERROR_ENCRYPTED_DATA_TOO_SHORT.to_string(),
            ));
        }

        let (nonce_bytes, ciphertext) = combined.split_at(NONCE_LENGTH);

        let nonce = Nonce::from_slice(nonce_bytes);

        let key = self.derive_key(password)?;

        let cipher = Aes256Gcm::new_from_slice(key.as_bytes())
            .map_err(|e| EncryptionError::KeyDerivation(format!("Invalid key: {e}")))?;

        let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|e| {
            EncryptionError::DecryptionFailed(format!("AES-GCM decryption failed: {e}"))
        })?;

        String::from_utf8(plaintext)
            .map_err(|e| EncryptionError::DecryptionFailed(format!("Invalid UTF-8: {e}")))
    }
}

impl Default for AesEncryption {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ConnectionStringEncryption {
    inner: AesEncryption,
}

impl ConnectionStringEncryption {
    pub fn new() -> Self {
        Self {
            inner: AesEncryption::new(),
        }
    }

    pub fn with_salt(salt: [u8; SALT_LENGTH]) -> Self {
        Self {
            inner: AesEncryption::with_salt(salt),
        }
    }

    pub fn salt_base64(&self) -> String {
        self.inner.salt_base64()
    }

    pub fn from_salt_base64(salt_b64: &str) -> Result<Self, EncryptionError> {
        Ok(Self {
            inner: AesEncryption::from_salt_base64(salt_b64)?,
        })
    }

    pub fn encrypt_connection_string(
        &self,
        plaintext: &str,
        password: &str,
    ) -> Result<String, EncryptionError> {
        self.inner
            .encrypt(plaintext, password, ERROR_EMPTY_CONNECTION_STRING)
    }

    pub fn decrypt_connection_string(
        &self,
        encrypted: &str,
        password: &str,
    ) -> Result<String, EncryptionError> {
        self.inner.decrypt(encrypted, password)
    }
}

impl Default for ConnectionStringEncryption {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ClientSecretEncryption {
    inner: AesEncryption,
}

impl ClientSecretEncryption {
    pub fn new() -> Self {
        Self {
            inner: AesEncryption::new(),
        }
    }

    pub fn with_salt(salt: [u8; SALT_LENGTH]) -> Self {
        Self {
            inner: AesEncryption::with_salt(salt),
        }
    }

    pub fn salt_base64(&self) -> String {
        self.inner.salt_base64()
    }

    pub fn from_salt_base64(salt_b64: &str) -> Result<Self, EncryptionError> {
        Ok(Self {
            inner: AesEncryption::from_salt_base64(salt_b64)?,
        })
    }

    pub fn encrypt_client_secret(
        &self,
        plaintext: &str,
        password: &str,
    ) -> Result<String, EncryptionError> {
        self.inner
            .encrypt(plaintext, password, ERROR_EMPTY_CLIENT_SECRET)
    }

    pub fn decrypt_client_secret(
        &self,
        encrypted: &str,
        password: &str,
    ) -> Result<String, EncryptionError> {
        self.inner.decrypt(encrypted, password)
    }
}

impl Default for ClientSecretEncryption {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let encryption = ConnectionStringEncryption::new();
        let plaintext = "Endpoint=sb://test.servicebus.windows.net/;SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=test123";
        let password = "test_password_123";

        let encrypted = encryption
            .encrypt_connection_string(plaintext, password)
            .expect("Encryption should succeed");

        let decrypted = encryption
            .decrypt_connection_string(&encrypted, password)
            .expect("Decryption should succeed");

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_wrong_password_fails() {
        let encryption = ConnectionStringEncryption::new();
        let plaintext = "test connection string";
        let password = "correct_password";
        let wrong_password = "wrong_password";

        let encrypted = encryption
            .encrypt_connection_string(plaintext, password)
            .expect("Encryption should succeed");

        let result = encryption.decrypt_connection_string(&encrypted, wrong_password);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_inputs() {
        let encryption = ConnectionStringEncryption::new();

        assert!(
            encryption
                .encrypt_connection_string("", "password")
                .is_err()
        );
        assert!(encryption.encrypt_connection_string("data", "").is_err());
        assert!(
            encryption
                .decrypt_connection_string("", "password")
                .is_err()
        );
        assert!(encryption.decrypt_connection_string("data", "").is_err());
    }

    #[test]
    fn test_salt_persistence() {
        let salt_b64 = "dGVzdF9zYWx0XzEyMzQ1Njc4OTBfYWJjZGVmZ2hpams=";
        let encryption1 = ConnectionStringEncryption::from_salt_base64(salt_b64)
            .expect("Should create from base64 salt");
        let encryption2 = ConnectionStringEncryption::from_salt_base64(salt_b64)
            .expect("Should create from same base64 salt");

        let plaintext = "test connection string";
        let password = "test_password";

        let encrypted1 = encryption1
            .encrypt_connection_string(plaintext, password)
            .expect("Encryption 1 should succeed");

        let decrypted2 = encryption2
            .decrypt_connection_string(&encrypted1, password)
            .expect("Decryption 2 should succeed");

        assert_eq!(plaintext, decrypted2);
    }

    #[test]
    fn test_different_salts_produce_different_ciphertexts() {
        let encryption1 = ConnectionStringEncryption::new();
        let encryption2 = ConnectionStringEncryption::new();

        let plaintext = "test connection string";
        let password = "test_password";

        let encrypted1 = encryption1
            .encrypt_connection_string(plaintext, password)
            .expect("Encryption 1 should succeed");

        let encrypted2 = encryption2
            .encrypt_connection_string(plaintext, password)
            .expect("Encryption 2 should succeed");

        assert_ne!(
            encrypted1, encrypted2,
            "Different salts should produce different ciphertexts"
        );
    }

    #[test]
    fn test_client_secret_encrypt_decrypt_roundtrip() {
        let encryption = ClientSecretEncryption::new();
        let plaintext = "secret_client_value_123";
        let password = "test_password_456";

        let encrypted = encryption
            .encrypt_client_secret(plaintext, password)
            .expect("Encryption should succeed");

        let decrypted = encryption
            .decrypt_client_secret(&encrypted, password)
            .expect("Decryption should succeed");

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_client_secret_wrong_password_fails() {
        let encryption = ClientSecretEncryption::new();
        let plaintext = "test client secret";
        let password = "correct_password";
        let wrong_password = "wrong_password";

        let encrypted = encryption
            .encrypt_client_secret(plaintext, password)
            .expect("Encryption should succeed");

        let result = encryption.decrypt_client_secret(&encrypted, wrong_password);
        assert!(result.is_err());
    }

    #[test]
    fn test_client_secret_empty_inputs() {
        let encryption = ClientSecretEncryption::new();

        assert!(encryption.encrypt_client_secret("", "password").is_err());
        assert!(encryption.encrypt_client_secret("data", "").is_err());
        assert!(encryption.decrypt_client_secret("", "password").is_err());
        assert!(encryption.decrypt_client_secret("data", "").is_err());
    }

    #[test]
    fn test_client_secret_salt_persistence() {
        // Generate a valid 32-byte salt and encode it to base64
        let salt_b64 = "J+CP5+9lfcD/SndIFvvdIEnltiA4UVtsraLndlzXSVk="; // exactly 32 bytes when decoded
        let encryption1 = ClientSecretEncryption::from_salt_base64(salt_b64)
            .expect("Should create from base64 salt");
        let encryption2 = ClientSecretEncryption::from_salt_base64(salt_b64)
            .expect("Should create from same base64 salt");

        let plaintext = "test client secret";
        let password = "test_password";

        let encrypted1 = encryption1
            .encrypt_client_secret(plaintext, password)
            .expect("Encryption 1 should succeed");

        let decrypted2 = encryption2
            .decrypt_client_secret(&encrypted1, password)
            .expect("Decryption 2 should succeed");

        assert_eq!(plaintext, decrypted2);
    }

    #[test]
    fn test_client_secret_different_salts_produce_different_ciphertexts() {
        let encryption1 = ClientSecretEncryption::new();
        let encryption2 = ClientSecretEncryption::new();

        let plaintext = "test client secret";
        let password = "test_password";

        let encrypted1 = encryption1
            .encrypt_client_secret(plaintext, password)
            .expect("Encryption 1 should succeed");

        let encrypted2 = encryption2
            .encrypt_client_secret(plaintext, password)
            .expect("Encryption 2 should succeed");

        assert_ne!(
            encrypted1, encrypted2,
            "Different salts should produce different ciphertexts"
        );
    }
}
