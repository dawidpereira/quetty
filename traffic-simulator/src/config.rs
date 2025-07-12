use crate::password::get_password_from_env_or_prompt;
use serde::{Deserialize, Serialize};
use server::encryption::ConnectionStringEncryption;
use std::env;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub traffic: TrafficConfig,
    pub display: DisplayConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TrafficConfig {
    pub min_messages_per_minute: u32,
    pub max_messages_per_minute: u32,
    pub message_prefix: String,
    pub use_json_format: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DisplayConfig {
    pub stats_update_interval_secs: u64,
    pub show_message_details: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SecurityConfig {
    #[serde(default = "default_max_password_attempts")]
    pub max_password_attempts: u32,
    #[serde(default = "default_encrypted_conn_var")]
    pub encrypted_conn_var: String,
    #[serde(default = "default_salt_var")]
    pub salt_var: String,
}

fn default_max_password_attempts() -> u32 {
    3
}

fn default_encrypted_conn_var() -> String {
    "SERVICEBUS__ENCRYPTED_CONNECTION_STRING".to_string()
}

fn default_salt_var() -> String {
    "SERVICEBUS__ENCRYPTION_SALT".to_string()
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_password_attempts: default_max_password_attempts(),
            encrypted_conn_var: default_encrypted_conn_var(),
            salt_var: default_salt_var(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_content = std::fs::read_to_string("config.toml")?;
        let mut config: Config = toml::from_str(&config_content)?;

        // Allow environment variable overrides
        if let Ok(min_rate) = env::var("TRAFFIC_MIN_RATE") {
            config.traffic.min_messages_per_minute = min_rate
                .parse()
                .unwrap_or(config.traffic.min_messages_per_minute);
        }
        if let Ok(max_rate) = env::var("TRAFFIC_MAX_RATE") {
            config.traffic.max_messages_per_minute = max_rate
                .parse()
                .unwrap_or(config.traffic.max_messages_per_minute);
        }
        if let Ok(prefix) = env::var("TRAFFIC_MESSAGE_PREFIX") {
            config.traffic.message_prefix = prefix;
        }
        if let Ok(max_attempts) = env::var("TRAFFIC_MAX_PASSWORD_ATTEMPTS") {
            config.security.max_password_attempts = max_attempts
                .parse()
                .unwrap_or(config.security.max_password_attempts);
        }

        Ok(config)
    }

    pub fn load_connection_string(&self) -> Result<String, Box<dyn std::error::Error>> {
        dotenv::from_filename("../.env").ok();

        // Try to load encrypted connection string first
        if let (Ok(encrypted_conn), Ok(salt_b64)) = (
            env::var(&self.security.encrypted_conn_var),
            env::var(&self.security.salt_var),
        ) {
            println!("🔒 Found encrypted connection string, prompting for password...");

            // Get password from environment or prompt user
            let password = get_password_from_env_or_prompt(
                "TRAFFIC_PASSWORD",
                self.security.max_password_attempts,
            )
            .map_err(|e| format!("Failed to get password: {}", e))?;

            // Decrypt connection string
            let encryption = ConnectionStringEncryption::from_salt_base64(&salt_b64)
                .map_err(|e| format!("Invalid encryption salt: {}", e))?;

            let decrypted = encryption
                .decrypt_connection_string(&encrypted_conn, password.as_str())
                .map_err(|e| format!("Failed to decrypt connection string: {}", e))?;

            println!("✅ Successfully decrypted connection string");
            Ok(decrypted)
        } else {
            return Err("No encrypted connection string found. Please ensure SERVICEBUS__ENCRYPTED_CONNECTION_STRING and SERVICEBUS__ENCRYPTION_SALT are set in your .env file.".into());
        }
    }
}
