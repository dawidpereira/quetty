//! Bulk operations module for handling batch message processing
//!
//! This module provides functionality for bulk send and delete operations on Service Bus messages.
//! It has been refactored into several specialized components:
//!
//! - `types`: Common types and data structures
//! - `resource_guard`: RAII resource management utilities
//! - `deleter`: Message deletion operations
//! - `handler`: Main coordinator that orchestrates operations

pub mod deleter;
pub mod handler;
pub mod resource_guard;
pub mod types;

// Re-export the main types and components
pub use deleter::{BulkDeleter, MessageDeleter};
pub use handler::BulkOperationHandler;
pub use types::{
    BatchConfig, // Keep for backward compatibility
    BulkOperationContext,
    BulkOperationResult,
    BulkSendParams,
    MessageIdentifier,
};

// Re-export resource guard utilities
pub use resource_guard::acquire_lock_with_timeout;
