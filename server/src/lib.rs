//! # Quetty Server Library
//!
//! Core server-side functionality for the Quetty Azure Service Bus terminal application.
//! This crate provides comprehensive Azure Service Bus integration, authentication,
//! message processing, and bulk operations capabilities.
//!
//! ## Architecture Overview
//!
//! The server library is organized into several key modules:
//!
//! - **[`auth`]** - Authentication system supporting Azure AD and connection strings
//! - **[`service_bus_manager`]** - Core Service Bus operations and management
//! - **[`producer`]** and **[`consumer`]** - Message production and consumption
//! - **[`bulk_operations`]** - Efficient bulk message processing
//! - **[`model`]** - Data models and message representations
//! - **[`taskpool`]** - Thread pool management for concurrent operations
//! - **[`utils`]** - Utility functions and helpers
//!
//! ## Key Features
//!
//! ### Multi-Modal Authentication
//! - **Azure Active Directory** - Device Code Flow and Client Credentials
//! - **Connection Strings** - SAS token-based authentication
//! - **Token Management** - Automatic refresh and caching
//!
//! ### High-Performance Message Processing
//! - **Concurrent Operations** - Multi-threaded message processing
//! - **Bulk Operations** - Efficient batch send/delete operations
//! - **Resource Management** - Automatic connection pooling and cleanup
//!
//! ### Azure Integration
//! - **Management API** - Namespace and queue discovery
//! - **Service Bus Operations** - Send, receive, peek, delete messages
//! - **Queue Statistics** - Real-time metrics and monitoring
//!
//! ## Quick Start
//!
//! ### Basic Service Bus Operations
//! ```no_run
//! use server::service_bus_manager::ServiceBusManager;
//! use server::auth::ConnectionStringProvider;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize authentication
//!     let auth_provider = ConnectionStringProvider::new(connection_config)?;
//!
//!     // Create Service Bus manager
//!     let manager = ServiceBusManager::new(auth_provider).await?;
//!
//!     // Send a message
//!     let message_id = manager.send_message(
//!         "my-queue",
//!         "Hello, Service Bus!".to_string(),
//!         None
//!     ).await?;
//!
//!     println!("Message sent with ID: {}", message_id);
//!     Ok(())
//! }
//! ```
//!
//! ### Bulk Operations
//! ```no_run
//! use server::bulk_operations::BulkOperationHandler;
//! use server::service_bus_manager::ServiceBusManager;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let manager = ServiceBusManager::new(auth_provider).await?;
//!     let bulk_handler = BulkOperationHandler::new(manager);
//!
//!     // Bulk send messages
//!     let messages = vec![
//!         "Message 1".to_string(),
//!         "Message 2".to_string(),
//!         "Message 3".to_string(),
//!     ];
//!
//!     let results = bulk_handler.bulk_send("my-queue", messages).await?;
//!     println!("Sent {} messages", results.len());
//!     Ok(())
//! }
//! ```
//!
//! ### Authentication with Azure AD
//! ```no_run
//! use server::auth::{AzureAdProvider, AuthStateManager};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize Azure AD authentication
//!     let auth_state = Arc::new(AuthStateManager::new());
//!     let azure_provider = AzureAdProvider::new(azure_config, http_client)?;
//!
//!     // Authenticate and get token
//!     let token = azure_provider.authenticate().await?;
//!     println!("Authenticated successfully");
//!     Ok(())
//! }
//! ```
//!
//! ## Integration with UI
//!
//! This server library is designed to work seamlessly with the Quetty UI:
//!
//! - **Shared Authentication State** - Synchronized auth across UI and server
//! - **Async Operations** - Non-blocking operations for responsive UI
//! - **Error Propagation** - Structured error handling for UI feedback
//! - **Progress Reporting** - Real-time operation progress for bulk operations
//!
//! ## Performance Considerations
//!
//! - **Connection Pooling** - Efficient Service Bus client management
//! - **Concurrent Operations** - Parallel processing where possible
//! - **Memory Management** - Careful resource cleanup and disposal
//! - **Batch Processing** - Optimized bulk operations for large datasets
//!
//! ## Error Handling
//!
//! The library provides comprehensive error handling with detailed error types:
//!
//! - **Authentication Errors** - Token issues, expired credentials
//! - **Service Bus Errors** - Network issues, quota exceeded, invalid operations
//! - **Validation Errors** - Input validation and constraint violations
//! - **Resource Errors** - Connection failures, timeout issues

pub mod auth;
pub mod bulk_operations;
pub mod common;
pub mod consumer;
pub mod encryption;
pub mod model;
pub mod producer;
pub mod service_bus_manager;
pub mod taskpool;
pub mod utils;
