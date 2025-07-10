//! # Quetty Server Library
//!
//! Core library for Azure Service Bus integration and message management.
//! This library provides authentication, message consumption/production,
//! bulk operations, and service bus management capabilities.
//!
//! ## Modules
//!
//! - [`auth`] - Authentication providers and token management
//! - [`bulk_operations`] - Bulk message operations (delete, send to DLQ, etc.)
//! - [`consumer`] - Message consumption from Service Bus queues
//! - [`producer`] - Message production to Service Bus queues
//! - [`model`] - Data models for messages and queue information
//! - [`service_bus_manager`] - Core Service Bus connection and queue management
//! - [`taskpool`] - Task pool for managing concurrent operations
//! - [`utils`] - Utility functions and helpers
//! - [`common`] - Common types and structures

pub mod auth;
pub mod bulk_operations;
pub mod common;
pub mod consumer;
pub mod model;
pub mod producer;
pub mod service_bus_manager;
pub mod taskpool;
pub mod utils;
