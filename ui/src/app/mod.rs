//! # Application Module
//!
//! Core application logic and state management for the Quetty TUI application.
//! This module contains the main application model, lifecycle management, and
//! coordination between different UI components and business logic.
//!
//! ## Architecture
//!
//! The application follows a model-view-update architecture:
//! - **Model** - Application state and data
//! - **View** - UI rendering and layout
//! - **Updates** - Message handling and state transitions
//!
//! ## Core Components
//!
//! - [`application_lifecycle`] - Application startup, shutdown, and lifecycle management
//! - [`model`] - Core application state and data structures
//! - [`view`] - UI rendering and layout logic
//! - [`updates`] - Message processing and state updates
//! - [`task_manager`] - Background task coordination
//! - [`queue_state`] - Queue-specific state management
//! - [`bulk_operation_processor`] - Bulk message operation handling
//!
//! ## Features
//!
//! - **State Management** - Centralized application state with type-safe updates
//! - **Component Coordination** - Message-based communication between UI components
//! - **Background Tasks** - Async task management for Service Bus operations
//! - **Queue Operations** - Message browsing, editing, and bulk operations
//! - **Authentication** - Integrated authentication flow management
//! - **Error Handling** - Comprehensive error reporting and user feedback
//!
//! ## Usage
//!
//! ```no_run
//! use quetty::app::application_lifecycle::ApplicationLifecycle;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut model = ApplicationLifecycle::initialize().await?;
//!     ApplicationLifecycle::setup_terminal(&mut model)?;
//!     ApplicationLifecycle::run_application_loop(&mut model)?;
//!     ApplicationLifecycle::shutdown_application(model)?;
//!     Ok(())
//! }
//! ```

/// Application lifecycle management - startup, shutdown, and main loop
pub mod application_lifecycle;
/// Bulk message operation processing and coordination
pub mod bulk_operation_processor;
/// Component managers for different UI areas
pub mod managers;
/// Core application model and state structures
pub mod model;
/// Queue-specific state management and operations
pub mod queue_state;
/// Component remounting and view management
pub mod remount;
/// Background task management and coordination
pub mod task_manager;
/// Message processing and state update logic
pub mod updates;
/// UI rendering and view composition
pub mod view;
