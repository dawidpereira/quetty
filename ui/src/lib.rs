// Library interface for integration testing
// This allows integration tests to access the modules

pub mod app;
pub mod components;
pub mod config;
pub mod error;
pub mod logger;
pub mod theme;

// Re-export commonly used types for easier access in tests
pub use error::AppError;

// Re-export the Msg type that tests commonly need
pub use components::common::Msg;

