use super::auth_provider::AuthProvider;
use super::auth_state::AuthStateManager;
use super::provider::AuthProvider as AuthProviderTrait;
use crate::service_bus_manager::ServiceBusError;
use std::sync::{Arc, Mutex};

// Global auth state that can be set by the UI
static GLOBAL_AUTH_STATE: Mutex<Option<Arc<AuthStateManager>>> = Mutex::new(None);

/// Set the global auth state manager (called by UI)
pub fn set_global_auth_state(auth_state: Arc<AuthStateManager>) {
    let mut global = GLOBAL_AUTH_STATE.lock().unwrap();
    *global = Some(auth_state);
}

/// Create an auth provider that uses the global auth state
pub fn create_auth_provider(
    fallback_provider: Option<Arc<dyn AuthProviderTrait>>,
) -> Result<Arc<dyn AuthProviderTrait>, ServiceBusError> {
    let global = GLOBAL_AUTH_STATE.lock().unwrap();

    if let Some(auth_state) = global.as_ref() {
        Ok(Arc::new(AuthProvider::new(
            auth_state.clone(),
            fallback_provider,
        )))
    } else {
        Err(ServiceBusError::ConfigurationError(
            "UI auth state not initialized".to_string(),
        ))
    }
}
