use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuthType {
    ConnectionString,
    AzureAd,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    pub primary_method: AuthType,
    pub fallback_enabled: bool,
    pub connection_string: Option<ConnectionStringConfig>,
    pub azure_ad: Option<AzureAdAuthConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectionStringConfig {
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AzureAdAuthConfig {
    pub flow: AzureAdFlowType,
    pub tenant_id: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub subscription_id: Option<String>,
    pub resource_group: Option<String>,
    pub namespace: Option<String>,
    pub authority_host: Option<String>,
    pub scope: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AzureAdFlowType {
    DeviceCode,
}

#[derive(Clone, Debug)]
pub struct CachedToken {
    pub token: String,
    pub expires_at: Instant,
    pub token_type: String,
}

impl CachedToken {
    pub fn new(token: String, expires_in: Duration, token_type: String) -> Self {
        Self {
            token,
            expires_at: Instant::now() + expires_in,
            token_type,
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    pub fn needs_refresh(&self) -> bool {
        let buffer = Duration::from_secs(300); // 5 minute buffer
        Instant::now() + buffer >= self.expires_at
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceCodeInfo {
    pub user_code: String,
    pub verification_uri: String,
    pub message: String,
}
