use super::provider::AuthProvider;
use super::token_cache::TokenCache;
use super::token_refresh_service::TokenRefreshService;
use super::types::DeviceCodeInfo;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

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
    token_cache: TokenCache,
    service_bus_provider: Arc<RwLock<Option<Arc<dyn AuthProvider>>>>,
    management_provider: Arc<RwLock<Option<Arc<dyn AuthProvider>>>>,
    refresh_service: Arc<RwLock<Option<Arc<TokenRefreshService>>>>,
    refresh_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
}

impl AuthStateManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(AuthenticationState::NotAuthenticated)),
            azure_ad_token: Arc::new(RwLock::new(None)),
            sas_token: Arc::new(RwLock::new(None)),
            token_cache: TokenCache::new(),
            service_bus_provider: Arc::new(RwLock::new(None)),
            management_provider: Arc::new(RwLock::new(None)),
            refresh_service: Arc::new(RwLock::new(None)),
            refresh_handle: Arc::new(RwLock::new(None)),
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

    // Provider management methods

    pub async fn set_service_bus_provider(&self, provider: Arc<dyn AuthProvider>) {
        let mut p = self.service_bus_provider.write().await;
        *p = Some(provider);
    }

    pub async fn get_service_bus_provider(&self) -> Option<Arc<dyn AuthProvider>> {
        self.service_bus_provider.read().await.clone()
    }

    pub async fn set_management_provider(&self, provider: Arc<dyn AuthProvider>) {
        let mut p = self.management_provider.write().await;
        *p = Some(provider);
    }

    pub async fn get_management_provider(&self) -> Option<Arc<dyn AuthProvider>> {
        self.management_provider.read().await.clone()
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

        // Store service and handle
        let mut service = self.refresh_service.write().await;
        *service = Some(refresh_service);

        let mut h = self.refresh_handle.write().await;
        *h = Some(handle);

        log::info!("Token refresh service started");
    }

    pub async fn stop_refresh_service(&self) {
        // Signal shutdown
        if let Some(service) = self.refresh_service.read().await.as_ref() {
            service.shutdown().await;
        }

        // Wait for service to stop
        let mut handle_guard = self.refresh_handle.write().await;
        if let Some(handle) = handle_guard.take() {
            let _ = handle.await;
        }

        // Clear service reference
        let mut service = self.refresh_service.write().await;
        *service = None;

        log::info!("Token refresh service stopped");
    }
}

impl Default for AuthStateManager {
    fn default() -> Self {
        Self::new()
    }
}
