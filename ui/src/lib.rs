//! # Quetty UI Library
//!
//! Terminal-based user interface for Azure Service Bus queue management.
//! This library provides a complete TUI application built with Ratatui and tui-realm
//! for managing Azure Service Bus queues, messages, and authentication.
//!
//! ## Features
//!
//! - Interactive terminal interface for queue management
//! - Multi-theme support with customizable styling
//! - Authentication handling (Device Code, Client Credentials, Connection String)
//! - Message browsing, editing, and bulk operations
//! - Configuration management with encryption support
//! - Error handling and user feedback systems
//!
//! ## Modules
//!
//! - [`app`] - Main application logic and component orchestration
//! - [`components`] - UI components and message handling
//! - [`config`] - Configuration management and persistence
//! - [`constants`] - Global constants for environment variables and shared values
//! - [`error`] - Error types and centralized error reporting
//! - [`logger`] - Logging configuration and utilities
//! - [`services`] - Business logic and external service integration
//! - [`theme`] - Theme management and styling
//! - [`utils`] - Utility functions and helpers
//! - [`validation`] - Input validation and sanitization
//!
//! This library interface enables integration testing by providing access to internal modules.

pub mod app;

pub mod components;
pub mod config;
pub mod constants;
pub mod error;
pub mod logger;
pub mod services;
pub mod theme;
pub mod utils;
pub mod validation;

// Re-export commonly used types for easier access in tests
pub use error::AppError;

// Re-export the Msg type that tests commonly need
pub use components::common::Msg;

// Re-export validation trait for broader use
pub use validation::Validator;
