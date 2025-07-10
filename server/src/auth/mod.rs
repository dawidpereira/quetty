//! # Authentication Module
//!
//! Comprehensive authentication system for Azure Service Bus operations supporting
//! multiple authentication methods and providers. This module provides a flexible
//! architecture that can handle various Azure authentication scenarios.
//!
//! ## Supported Authentication Methods
//!
//! ### Azure Active Directory (Azure AD)
//! - **Device Code Flow** - Interactive authentication for CLI applications
//! - **Client Credentials Flow** - Service principal authentication for automated scenarios
//!
//! ### Connection String Authentication
//! - **Shared Access Signature (SAS)** - Token-based authentication using connection strings
//! - **Automatic SAS Token Generation** - Time-limited tokens with configurable expiration
//!
//! ## Architecture Overview
//!
//! The authentication system is built around several key components:
//!
//! - **[`AuthProvider`]** - Core trait defining the authentication interface
//! - **[`AuthStateManager`]** - Centralized state management for authentication
//! - **[`TokenCache`]** - Efficient caching with automatic expiration handling
//! - **[`TokenRefreshService`]** - Background token refresh for long-running operations
//!
//! ## Authentication Providers
//!
//! ### Azure AD Provider
//! ```no_run
//! use server::auth::{AzureAdProvider, AzureAdAuthConfig};
//!
//! let config = AzureAdAuthConfig {
//!     auth_method: "device_code".to_string(),
//!     tenant_id: Some("your-tenant-id".to_string()),
//!     client_id: Some("your-client-id".to_string()),
//!     ..Default::default()
//! };
//!
//! let provider = AzureAdProvider::new(config, http_client)?;
//! let token = provider.authenticate().await?;
//! ```
//!
//! ### Connection String Provider
//! ```no_run
//! use server::auth::{ConnectionStringProvider, ConnectionStringConfig};
//!
//! let config = ConnectionStringConfig {
//!     value: "Endpoint=sb://test.servicebus.windows.net/;SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=...".to_string(),
//! };
//!
//! let provider = ConnectionStringProvider::new(config)?;
//! let token = provider.authenticate().await?;
//! ```
//!
//! ## State Management
//!
//! The [`AuthStateManager`] provides centralized authentication state:
//!
//! ```no_run
//! use server::auth::AuthStateManager;
//! use std::sync::Arc;
//!
//! let auth_manager = Arc::new(AuthStateManager::new());
//!
//! // Check authentication status
//! if auth_manager.is_authenticated().await {
//!     println!("Already authenticated");
//! }
//!
//! // Start automatic token refresh
//! auth_manager.clone().start_refresh_service().await;
//! ```
//!
//! ## Token Caching
//!
//! Automatic token caching with expiration management:
//!
//! ```no_run
//! use server::auth::TokenCache;
//!
//! let cache = TokenCache::new();
//!
//! // Check if token needs refresh
//! if cache.needs_refresh("user_token").await {
//!     // Refresh token...
//! }
//! ```
//!
//! ## Integration with Service Bus
//!
//! The authentication system integrates seamlessly with Service Bus operations:
//!
//! ```no_run
//! use server::auth::{create_service_bus_auth_provider, get_azure_ad_token_with_auth};
//!
//! // Create provider for Service Bus
//! let provider = create_service_bus_auth_provider(
//!     "azure_ad",
//!     None,
//!     &azure_config,
//!     http_client
//! )?;
//!
//! // Get token for operations
//! let token = get_azure_ad_token_with_auth(&provider).await?;
//! ```

pub mod auth_provider;
pub mod auth_setup;
pub mod auth_state;
pub mod azure_ad;
pub mod connection_string;
pub mod provider;
pub mod sas_token_generator;
pub mod service_bus_auth;
pub mod token_cache;
pub mod token_refresh_service;
pub mod types;

pub use crate::common::TokenRefreshError;
pub use auth_setup::{create_auth_provider, set_global_auth_state};
pub use auth_state::{AuthStateManager, AuthenticationState};
pub use azure_ad::{AzureAdProvider, DeviceCodeFlowInfo};
pub use connection_string::ConnectionStringProvider;
pub use provider::{AuthProvider, AuthToken};
pub use sas_token_generator::SasTokenGenerator;
pub use service_bus_auth::{
    create_auth_provider as create_service_bus_auth_provider, get_azure_ad_token_with_auth,
};
pub use token_cache::TokenCache;
pub use token_refresh_service::TokenRefreshService;
pub use types::{AuthConfig, AuthType, DeviceCodeInfo};
