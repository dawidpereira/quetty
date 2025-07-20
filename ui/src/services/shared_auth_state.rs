use quetty_server::auth::auth_state::AuthStateManager;
use std::sync::{Arc, OnceLock};

static SHARED_AUTH_STATE: OnceLock<Arc<AuthStateManager>> = OnceLock::new();

/// Initialize the shared authentication state
pub fn init_shared_auth_state() -> Arc<AuthStateManager> {
    SHARED_AUTH_STATE
        .get_or_init(|| Arc::new(AuthStateManager::new()))
        .clone()
}
