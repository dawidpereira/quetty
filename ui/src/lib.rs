// Library interface for integration testing
// This allows integration tests to access the modules

pub mod app;

pub mod components;
pub mod config;
pub mod error;
pub mod logger;
pub mod services;
pub mod theme;
pub mod validation;

// Re-export commonly used types for easier access in tests
pub use error::AppError;

// Re-export the Msg type that tests commonly need
pub use components::common::Msg;

// Re-export validation trait for broader use
pub use validation::Validator;
