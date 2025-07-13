//! # Services Module
//!
//! Business logic and service layer for the Quetty terminal user interface.
//! This module provides high-level services that encapsulate complex operations
//! and coordinate between different parts of the application.
//!
//! ## Architecture
//!
//! The services layer acts as an intermediary between UI components and the core
//! server functionality, providing:
//!
//! - **Abstraction** - Simplified interfaces for complex operations
//! - **State Management** - Coordinated state between UI and business logic
//! - **Error Handling** - Consistent error handling and user feedback
//! - **Async Coordination** - Proper async operation management
//!
//! ## Available Services
//!
//! ### Authentication Service
//!
//! The [`AuthService`] provides high-level authentication operations:
//!
//! ```ignore
//! use quetty::services::AuthService;
//!
//! let auth_service = AuthService::new();
//!
//! // Handle authentication flow
//! match auth_service.authenticate_user(&config).await {
//!     Ok(token) => println!("Authentication successful"),
//!     Err(e) => eprintln!("Authentication failed: {}", e),
//! }
//! ```
//!
//! ### Shared Authentication State
//!
//! Global authentication state management for the UI:
//!
//! ```ignore
//! use quetty::services::init_shared_auth_state;
//!
//! // Initialize shared authentication state
//! let auth_state = init_shared_auth_state();
//!
//! // Use across the application
//! if auth_state.is_authenticated().await {
//!     // Proceed with authenticated operations
//! }
//! ```
//!
//! ## Integration with Components
//!
//! Services are designed to be easily integrated with UI components:
//!
//! - **Stateless Operations** - Services can be called from any component
//! - **Consistent APIs** - Uniform interfaces across all services
//! - **Error Propagation** - Structured error handling for UI feedback
//! - **Async Support** - Full async/await support for non-blocking UI

pub mod auth_service;
pub mod shared_auth_state;

pub use auth_service::AuthService;
pub use shared_auth_state::init_shared_auth_state;
