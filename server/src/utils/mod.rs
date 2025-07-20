//! # Server Utilities Module
//!
//! Collection of server-side utility functions and helpers for the Quetty application.
//! This module provides essential infrastructure utilities that support various aspects
//! of the server-side operations including environment variable handling, configuration
//! management, and common helper functions.
//!
//! ## Available Utilities
//!
//! ### Environment Variable Utilities
//!
//! The [`env`] module provides safe and validated access to environment variables:
//!
//! ```no_run
//! use quetty_server::utils::env::EnvUtils;
//!
//! // Check if a variable exists and has a value
//! if EnvUtils::has_non_empty_var("DATABASE_URL") {
//!     // Get the validated value
//!     let url = EnvUtils::get_validated_var("DATABASE_URL")?;
//!     println!("Database URL: {}", url);
//! }
//!
//! // Get optional variables with defaults
//! let debug_level = EnvUtils::get_optional_var("DEBUG_LEVEL")
//!     .unwrap_or_else(|| "info".to_string());
//! ```
//!
//! ## Design Principles
//!
//! - **Safety First** - All utilities prioritize safe operations and proper error handling
//! - **Validation** - Input validation and sanitization for all external inputs
//! - **Error Handling** - Comprehensive error handling with detailed feedback
//! - **Performance** - Efficient implementations suitable for server operations
//! - **Reusability** - Functions designed for use across multiple server components
//!
//! ## Integration with Server Components
//!
//! These utilities are designed to integrate seamlessly with server components:
//!
//! - **Configuration Loading** - Safe environment variable access for configuration
//! - **Service Initialization** - Reliable utilities for service startup procedures
//! - **Error Reporting** - Structured error handling for operational feedback
//! - **Resource Management** - Efficient resource handling and cleanup
//!
//! ## Usage in Authentication Systems
//!
//! Server utilities play a crucial role in authentication configuration:
//!
//! ```no_run
//! use quetty_server::utils::env::EnvUtils;
//!
//! // Load authentication configuration from environment
//! let tenant_id = EnvUtils::get_validated_var("AZURE_TENANT_ID")?;
//! let client_id = EnvUtils::get_validated_var("AZURE_CLIENT_ID")?;
//!
//! // Optional configuration with fallbacks
//! let token_cache_duration = EnvUtils::get_optional_var("TOKEN_CACHE_DURATION")
//!     .unwrap_or_else(|| "3600".to_string());
//! ```

pub mod env;
