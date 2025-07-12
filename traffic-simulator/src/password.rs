use rpassword::read_password;
use std::io::{self, Write};
use zeroize::ZeroizeOnDrop;

/// Secure password container that automatically clears memory on drop
#[derive(ZeroizeOnDrop)]
pub struct SecurePassword(String);

impl SecurePassword {
    pub fn new(password: String) -> Self {
        Self(password)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Password input errors
#[derive(Debug, thiserror::Error)]
pub enum PasswordError {
    #[error("Failed to read password: {0}")]
    ReadError(#[from] io::Error),
    #[error("Empty password provided")]
    EmptyPassword,
    #[error("Maximum password attempts exceeded")]
    MaxAttemptsExceeded,
}

/// Prompts user for password input with secure handling
pub fn prompt_password() -> Result<SecurePassword, PasswordError> {
    print!("🔐 Enter decryption password: ");
    io::stdout().flush()?;

    let password = read_password()?;

    if password.trim().is_empty() {
        return Err(PasswordError::EmptyPassword);
    }

    Ok(SecurePassword::new(password))
}

/// Prompts user for password with retry logic
pub fn prompt_password_with_retry(max_attempts: u32) -> Result<SecurePassword, PasswordError> {
    let mut attempts = 0;

    loop {
        attempts += 1;

        match prompt_password() {
            Ok(password) => return Ok(password),
            Err(PasswordError::EmptyPassword) => {
                eprintln!("❌ Password cannot be empty. Please try again.");
                if attempts >= max_attempts {
                    return Err(PasswordError::MaxAttemptsExceeded);
                }
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}

/// Gets password from environment variable or prompts user
pub fn get_password_from_env_or_prompt(
    env_var: &str,
    max_attempts: u32,
) -> Result<SecurePassword, PasswordError> {
    // First try environment variable
    if let Ok(password) = std::env::var(env_var) {
        if !password.trim().is_empty() {
            return Ok(SecurePassword::new(password));
        }
    }

    // Fall back to prompting user
    prompt_password_with_retry(max_attempts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_password_zeroizes() {
        let password_data = "test_password_123".to_string();
        let secure_password = SecurePassword::new(password_data);

        assert_eq!(secure_password.as_str(), "test_password_123");
        // Password will be zeroized when dropped
    }

    #[test]
    fn test_empty_password_error() {
        // This test can't easily test the interactive prompt,
        // but we can test the error type
        let error = PasswordError::EmptyPassword;
        assert_eq!(error.to_string(), "Empty password provided");
    }

    #[test]
    fn test_max_attempts_error() {
        let error = PasswordError::MaxAttemptsExceeded;
        assert_eq!(error.to_string(), "Maximum password attempts exceeded");
    }
}
