use super::types::DeviceCodeInfo;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub enum AuthenticationState {
    NotAuthenticated,
    AwaitingDeviceCode {
        info: DeviceCodeInfo,
        started_at: Instant,
    },
    Authenticated {
        token: String,
        expires_at: Instant,
        connection_string: Option<String>,
    },
    Failed(String),
}

pub struct AuthStateManager {
    state: Arc<RwLock<AuthenticationState>>,
    azure_ad_token: Arc<RwLock<Option<(String, Instant)>>>,
    sas_token: Arc<RwLock<Option<(String, Instant)>>>,
}

impl AuthStateManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(AuthenticationState::NotAuthenticated)),
            azure_ad_token: Arc::new(RwLock::new(None)),
            sas_token: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn get_state(&self) -> AuthenticationState {
        self.state.read().await.clone()
    }

    pub async fn set_device_code_pending(&self, info: DeviceCodeInfo) {
        let mut state = self.state.write().await;
        *state = AuthenticationState::AwaitingDeviceCode {
            info,
            started_at: Instant::now(),
        };
    }

    pub async fn set_authenticated(
        &self,
        token: String,
        expires_in: Duration,
        connection_string: Option<String>,
    ) {
        let mut state = self.state.write().await;
        *state = AuthenticationState::Authenticated {
            token: token.clone(),
            expires_at: Instant::now() + expires_in,
            connection_string,
        };

        // Store Azure AD token
        let mut ad_token = self.azure_ad_token.write().await;
        *ad_token = Some((token, Instant::now() + expires_in));
    }

    pub async fn set_failed(&self, error: String) {
        let mut state = self.state.write().await;
        *state = AuthenticationState::Failed(error);
    }

    pub async fn logout(&self) {
        let mut state = self.state.write().await;
        *state = AuthenticationState::NotAuthenticated;

        let mut ad_token = self.azure_ad_token.write().await;
        *ad_token = None;

        let mut sas_token = self.sas_token.write().await;
        *sas_token = None;
    }

    pub async fn is_authenticated(&self) -> bool {
        let state = self.state.read().await;
        matches!(*state, AuthenticationState::Authenticated { .. })
    }

    pub async fn needs_reauthentication(&self) -> bool {
        let state = self.state.read().await;
        match &*state {
            AuthenticationState::Authenticated { expires_at, .. } => {
                // Check if token expires in less than 5 minutes
                Instant::now() + Duration::from_secs(300) >= *expires_at
            }
            _ => true,
        }
    }

    pub async fn get_azure_ad_token(&self) -> Option<String> {
        let token = self.azure_ad_token.read().await;
        if let Some((token_str, expires_at)) = token.as_ref() {
            if Instant::now() < *expires_at {
                return Some(token_str.clone());
            }
        }
        None
    }

    pub async fn get_sas_token(&self) -> Option<String> {
        let token = self.sas_token.read().await;
        if let Some((token_str, expires_at)) = token.as_ref() {
            if Instant::now() < *expires_at {
                return Some(token_str.clone());
            }
        }
        None
    }

    pub async fn set_sas_token(&self, token: String, expires_in: Duration) {
        let mut sas_token = self.sas_token.write().await;
        *sas_token = Some((token, Instant::now() + expires_in));
    }

    pub async fn get_connection_string(&self) -> Option<String> {
        let state = self.state.read().await;
        match &*state {
            AuthenticationState::Authenticated {
                connection_string, ..
            } => connection_string.clone(),
            _ => None,
        }
    }

    pub async fn get_device_code_info(&self) -> Option<DeviceCodeInfo> {
        let state = self.state.read().await;
        match &*state {
            AuthenticationState::AwaitingDeviceCode { info, .. } => Some(info.clone()),
            _ => None,
        }
    }
}

impl Default for AuthStateManager {
    fn default() -> Self {
        Self::new()
    }
}
