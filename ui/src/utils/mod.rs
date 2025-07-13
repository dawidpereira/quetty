//! # Utilities Module
//!
//! Collection of utility functions and helpers for the Quetty terminal user interface.
//! This module provides reusable functionality that supports various aspects of the
//! application including authentication, connection string handling, and data security.
//!
//! ## Available Utilities
//!
//! ### Authentication Utilities
//!
//! The [`auth`] module provides helper functions for authentication operations:
//!
//! ```ignore
//! use quetty::utils::auth;
//!
//! // Validate authentication configuration
//! if auth::validate_auth_config(&config)? {
//!     // Proceed with authentication
//! }
//! ```
//!
//! ### Connection String Utilities
//!
//! The [`connection_string`] module offers tools for working with Azure Service Bus connection strings:
//!
//! ```ignore
//! use quetty::utils::connection_string;
//!
//! // Parse and validate connection strings
//! let conn_str = "Endpoint=sb://example.servicebus.windows.net/";
//! let parsed = connection_string::parse_connection_string(&conn_str)?;
//! let is_valid = connection_string::validate_connection_string(&conn_str);
//! ```
//!
//! ### Encryption Utilities
//!
//! The [`encryption`] module provides secure data handling capabilities:
//!
//! ```ignore
//! use quetty::utils::encryption;
//!
//! // Encrypt sensitive configuration data
//! let sensitive_data = "secret_key";
//! let password = "user_password";
//! let encrypted = encryption::encrypt_data(&sensitive_data, &password)?;
//! let decrypted = encryption::decrypt_data(&encrypted, &password)?;
//! ```
//!
//! ## Design Principles
//!
//! - **Security First** - All utilities prioritize data security and safe operations
//! - **Error Handling** - Comprehensive error handling with detailed feedback
//! - **Reusability** - Functions designed for use across multiple components
//! - **Performance** - Efficient implementations suitable for terminal UI responsiveness
//! - **Validation** - Input validation and sanitization where appropriate

pub mod auth;
pub mod connection_string;
pub mod encryption;
