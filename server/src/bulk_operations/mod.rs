//! Bulk operations module for handling batch message processing
//!
//! This module provides functionality for bulk send and delete operations on Service Bus messages.
//! It has been refactored into several specialized components:
//!
//! - `types`: Common types and data structures
//! - `resource_guard`: RAII resource management utilities  
//! - `collector`: Message collection from queues
//! - `sender`: Message sending operations
//! - `deleter`: Message deletion operations
//! - `handler`: Main coordinator that orchestrates operations

pub mod collector;
pub mod deleter;
pub mod handler;
pub mod resource_guard;
pub mod sender;
pub mod types;

// Re-export the main public API
pub use handler::BulkOperationHandler;
pub use resource_guard::{acquire_lock_with_timeout, ServiceBusResourceGuard};
pub use types::{
    BatchConfig, BulkOperationContext, BulkOperationResult, BulkSendParams, MessageIdentifier,
    ProcessTargetMessagesParams, QueueOperationType, ServiceBusOperationContext,
}; 