use super::provider::AuthProvider;
use super::token_cache::TokenCache;
use super::token_refresh_service::TokenRefreshService;
use super::types::DeviceCodeInfo;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

#[derive(Clone, Debug, Default)]
pub enum AuthenticationState {
    #[default]
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

// Consolidated state structure to prevent deadlocks
#[derive(Default)]
struct AuthState {
    authentication_state: AuthenticationState,
    azure_ad_token: Option<(String, Instant)>,
    sas_token: Option<(String, Instant)>,
    service_bus_provider: Option<Arc<dyn AuthProvider>>,
    management_provider: Option<Arc<dyn AuthProvider>>,
    refresh_service: Option<Arc<TokenRefreshService>>,
    refresh_handle: Option<JoinHandle<()>>,
}

pub struct AuthStateManager {
    inner: Arc<RwLock<AuthState>>,
    token_cache: TokenCache,
}

impl AuthStateManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(AuthState::default())),
            token_cache: TokenCache::new(),
        }
    }

    pub async fn get_state(&self) -> AuthenticationState {
        self.inner.read().await.authentication_state.clone()
    }

    pub async fn set_device_code_pending(&self, info: DeviceCodeInfo) {
        let mut state = self.inner.write().await;
        state.authentication_state = AuthenticationState::AwaitingDeviceCode {
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
        let mut state = self.inner.write().await;
        let expires_at = Instant::now() + expires_in;

        state.authentication_state = AuthenticationState::Authenticated {
            token: token.clone(),
            expires_at,
            connection_string,
        };

        // Store Azure AD token
        state.azure_ad_token = Some((token, expires_at));
    }

    pub async fn set_failed(&self, error: String) {
        let mut state = self.inner.write().await;
        state.authentication_state = AuthenticationState::Failed(error);
    }

    pub async fn logout(&self) {
        let mut state = self.inner.write().await;
        state.authentication_state = AuthenticationState::NotAuthenticated;
        state.azure_ad_token = None;
        state.sas_token = None;
    }

    pub async fn is_authenticated(&self) -> bool {
        let state = self.inner.read().await;
        matches!(
            state.authentication_state,
            AuthenticationState::Authenticated { .. }
        )
    }

    pub async fn needs_reauthentication(&self) -> bool {
        let state = self.inner.read().await;
        match &state.authentication_state {
            AuthenticationState::Authenticated { expires_at, .. } => {
                // Check if token expires in less than 5 minutes
                Instant::now() + Duration::from_secs(300) >= *expires_at
            }
            _ => true,
        }
    }

    pub async fn get_azure_ad_token(&self) -> Option<String> {
        let state = self.inner.read().await;
        if let Some((token_str, expires_at)) = &state.azure_ad_token {
            if Instant::now() < *expires_at {
                return Some(token_str.clone());
            }
        }
        None
    }

    pub async fn get_sas_token(&self) -> Option<String> {
        let state = self.inner.read().await;
        if let Some((token_str, expires_at)) = &state.sas_token {
            if Instant::now() < *expires_at {
                return Some(token_str.clone());
            }
        }
        None
    }

    pub async fn set_sas_token(&self, token: String, expires_in: Duration) {
        let mut state = self.inner.write().await;
        state.sas_token = Some((token, Instant::now() + expires_in));
    }

    pub async fn get_connection_string(&self) -> Option<String> {
        let state = self.inner.read().await;
        match &state.authentication_state {
            AuthenticationState::Authenticated {
                connection_string, ..
            } => connection_string.clone(),
            _ => None,
        }
    }

    pub async fn get_device_code_info(&self) -> Option<DeviceCodeInfo> {
        let state = self.inner.read().await;
        match &state.authentication_state {
            AuthenticationState::AwaitingDeviceCode { info, .. } => Some(info.clone()),
            _ => None,
        }
    }

    // Provider management methods

    pub async fn set_service_bus_provider(&self, provider: Arc<dyn AuthProvider>) {
        let mut state = self.inner.write().await;
        state.service_bus_provider = Some(provider);
    }

    pub async fn get_service_bus_provider(&self) -> Option<Arc<dyn AuthProvider>> {
        self.inner.read().await.service_bus_provider.clone()
    }

    pub async fn set_management_provider(&self, provider: Arc<dyn AuthProvider>) {
        let mut state = self.inner.write().await;
        state.management_provider = Some(provider);
    }

    pub async fn get_management_provider(&self) -> Option<Arc<dyn AuthProvider>> {
        self.inner.read().await.management_provider.clone()
    }

    pub fn get_token_cache(&self) -> &TokenCache {
        &self.token_cache
    }

    // Token refresh service management

    pub async fn start_refresh_service(self: Arc<Self>) {
        self.start_refresh_service_with_callback(None).await;
    }

    pub async fn start_refresh_service_with_callback(
        self: Arc<Self>,
        failure_callback: Option<super::token_refresh_service::RefreshFailureCallback>,
    ) {
        // Stop any existing service
        self.stop_refresh_service().await;

        // Create and start new service
        let mut refresh_service = TokenRefreshService::new(self.clone());
        if let Some(callback) = failure_callback {
            refresh_service = refresh_service.with_failure_callback(callback);
        }

        let refresh_service = Arc::new(refresh_service);
        let handle = refresh_service.clone().start();

        // Store service and handle in consolidated state
        let mut state = self.inner.write().await;
        state.refresh_service = Some(refresh_service);
        state.refresh_handle = Some(handle);

        log::info!("Token refresh service started");
    }

    pub async fn stop_refresh_service(&self) {
        // Get service reference and signal shutdown
        let service_ref = {
            let state = self.inner.read().await;
            state.refresh_service.clone()
        };

        if let Some(service) = service_ref {
            service.shutdown().await;
        }

        // Wait for service to stop and clear references
        let mut state = self.inner.write().await;
        if let Some(handle) = state.refresh_handle.take() {
            // Drop the write lock before waiting
            drop(state);
            let _ = handle.await;

            // Re-acquire write lock to clear service reference
            let mut state = self.inner.write().await;
            state.refresh_service = None;
        } else {
            state.refresh_service = None;
        }

        log::info!("Token refresh service stopped");
    }
}

impl Default for AuthStateManager {
    fn default() -> Self {
        Self::new()
    }
}
