//! Authentication setup and global state management.
//!
//! This module provides functionality for setting up and managing global authentication
//! state that can be shared between the UI and server components. It handles the
//! coordination of authentication providers and state management across the application.

use super::auth_provider::AuthProvider;
use super::auth_state::AuthStateManager;
use super::provider::AuthProvider as AuthProviderTrait;
use crate::service_bus_manager::ServiceBusError;
use std::sync::{Arc, Mutex};

/// Global authentication state manager shared across the application.
///
/// This static variable holds the authentication state manager that can be
/// set by the UI and used by server components for authentication operations.
static GLOBAL_AUTH_STATE: Mutex<Option<Arc<AuthStateManager>>> = Mutex::new(None);

/// Sets the global authentication state manager.
///
/// This function is typically called by the UI component during initialization
/// to establish a shared authentication state that can be used by server
/// components for authentication operations.
///
/// # Arguments
///
/// * `auth_state` - The authentication state manager to set as global
///
/// # Examples
///
/// ```no_run
/// use server::auth::{AuthStateManager, set_global_auth_state};
/// use std::sync::Arc;
///
/// let auth_state = Arc::new(AuthStateManager::new());
/// set_global_auth_state(auth_state);
/// ```
pub fn set_global_auth_state(auth_state: Arc<AuthStateManager>) {
    let mut global = GLOBAL_AUTH_STATE.lock().unwrap();
    *global = Some(auth_state);
}

/// Creates an authentication provider that uses the global authentication state.
///
/// This function creates an [`AuthProvider`] that integrates with the global
/// authentication state manager. It provides a bridge between the UI authentication
/// state and server-side authentication operations.
///
/// # Arguments
///
/// * `fallback_provider` - Optional fallback provider to use if the global state fails
///
/// # Returns
///
/// An [`AuthProvider`] that can be used for authentication operations
///
/// # Errors
///
/// Returns [`ServiceBusError::ConfigurationError`] if:
/// - The global authentication state has not been initialized
/// - The global state is in an invalid state
///
/// # Examples
///
/// ```no_run
/// use server::auth::{create_auth_provider, set_global_auth_state, AuthStateManager};
/// use std::sync::Arc;
///
/// // First, initialize the global state
/// let auth_state = Arc::new(AuthStateManager::new());
/// set_global_auth_state(auth_state);
///
/// // Then create a provider that uses the global state
/// let provider = create_auth_provider(None)?;
/// let token = provider.authenticate().await?;
/// ```
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
