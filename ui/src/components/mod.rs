//! # Components Module
//!
//! Comprehensive UI component system for the Quetty terminal user interface.
//! This module provides a complete set of reusable, interactive components
//! built on the TUI-Realm framework for building rich terminal applications.
//!
//! ## Component Architecture
//!
//! The component system follows a hierarchical design with several categories:
//!
//! ### Core Components
//! - **[`common`]** - Shared message types and common functionality
//! - **[`state`]** - Component state management and synchronization
//!
//! ### Input Components
//! Interactive components for user input and selection:
//! - **[`namespace_picker`]** - Azure Service Bus namespace selection
//! - **[`queue_picker`]** - Queue selection and browsing
//! - **[`resource_group_picker`]** - Azure resource group selection
//! - **[`subscription_picker`]** - Azure subscription selection
//! - **[`theme_picker`]** - Theme and color scheme selection
//!
//! ### Configuration Components
//! Components for application configuration and setup:
//! - **[`config_screen`]** - Main configuration interface
//! - **[`password_popup`]** - Secure password input
//!
//! ### Popup Components
//! Modal dialogs and overlay components:
//! - **[`auth_popup`]** - Authentication flow interface
//! - **[`confirmation_popup`]** - User confirmation dialogs
//! - **[`error_popup`]** - Error message display
//! - **[`number_input_popup`]** - Numeric input dialog
//! - **[`page_size_popup`]** - Pagination configuration
//! - **[`success_popup`]** - Success message display
//!
//! ### Display Components
//! Components for information display and user guidance:
//! - **[`help`]** - Context-sensitive help system
//! - **[`help_bar`]** - Quick help and shortcut display
//! - **[`help_screen`]** - Comprehensive help interface
//! - **[`loading_indicator`]** - Progress and loading feedback
//! - **[`text_label`]** - Formatted text display
//!
//! ### Complex Components
//! Full-featured components for core application functionality:
//! - **[`message_details`]** - Message viewing and editing
//! - **[`messages`]** - Message list and management
//!
//! ### System Components
//! Infrastructure components for application behavior:
//! - **[`global_key_watcher`]** - Global keyboard event handling
//!
//! ### Utility Components
//! Shared patterns and reusable utilities:
//! - **[`base_popup`]** - Base popup implementation
//! - **[`validation_patterns`]** - Input validation utilities
//!
//! ## Component Lifecycle
//!
//! All components follow the TUI-Realm component lifecycle:
//!
//! ```ignore
//! use tuirealm::{Component, MockComponent, Event, EventListenerCfg};
//! use quetty::components::common::Msg;
//!
//! // Component initialization
//! let mut component = MyComponent::new();
//!
//! // Mount component to application
//! app.mount(ComponentId::MyComponent, Box::new(component))?;
//!
//! // Handle events
//! let result = app.tick(Event::Key(key_event))?;
//! match result {
//!     Some(Msg::ComponentMessage(data)) => {
//!         // Handle component message
//!     }
//!     None => {} // No message generated
//! }
//!
//! // Unmount when done
//! app.umount(&ComponentId::MyComponent)?;
//! ```
//!
//! ## Message Passing System
//!
//! Components communicate through a unified message system:
//!
//! ```ignore
//! use quetty::components::common::Msg;
//!
//! // User interaction messages
//! let msg = Msg::QueueSelected("my-queue".to_string());
//! let msg = Msg::MessageSelected(message_id);
//!
//! // Operation messages
//! let msg = Msg::BulkOperationStarted;
//! let msg = Msg::AuthenticationRequired;
//!
//! // System messages
//! let msg = Msg::ErrorOccurred(error_details);
//! let msg = Msg::OperationCompleted;
//! ```
//!
//! ## Styling and Theming
//!
//! All components support dynamic theming:
//!
//! ```ignore
//! use quetty::theme::ThemeManager;
//! use tuirealm::props::{PropPayload, PropValue};
//!
//! // Apply theme colors to component
//! component.attr(
//!     tuirealm::Attribute::Foreground,
//!     tuirealm::AttrValue::Color(ThemeManager::text_primary()),
//! );
//!
//! component.attr(
//!     tuirealm::Attribute::Background,
//!     tuirealm::AttrValue::Color(ThemeManager::surface()),
//! );
//! ```
//!
//! ## Validation and Input Handling
//!
//! Components include comprehensive input validation:
//!
//! ```ignore
//! use quetty::components::validation_patterns::{ValidationPattern, InputValidator};
//!
//! // Validate user input
//! let validator = InputValidator::new(ValidationPattern::QueueName);
//! let is_valid = validator.validate(&user_input);
//!
//! if !is_valid {
//!     // Show validation error
//!     component.show_error("Invalid queue name format");
//! }
//! ```
//!
//! ## Error Handling
//!
//! Components provide consistent error handling:
//!
//! - **Input Validation** - Real-time validation feedback
//! - **Operation Errors** - Clear error messages and recovery options
//! - **Network Errors** - Graceful handling of connectivity issues
//! - **Authentication Errors** - Guided re-authentication flows
//!
//! ## Accessibility Features
//!
//! - **Keyboard Navigation** - Full keyboard accessibility
//! - **Screen Reader Support** - Proper ARIA-like labeling
//! - **High Contrast** - Support for accessibility themes
//! - **Focus Management** - Clear focus indicators and navigation
//!
//! ## Performance Optimizations
//!
//! - **Lazy Rendering** - Components render only when visible
//! - **Efficient Updates** - Minimal redraws and state changes
//! - **Memory Management** - Proper cleanup of component resources
//! - **Event Debouncing** - Throttled input handling for responsiveness

// Core components
pub mod common;
pub mod state;

// Reusable patterns and utilities
pub mod base_popup;
pub mod validation_patterns;

// UI Components organized by category
// Input components
pub mod namespace_picker;
pub mod queue_picker;
pub mod resource_group_picker;
pub mod subscription_picker;
pub mod theme_picker;

// Configuration components
pub mod config_screen;
pub mod password_popup;

// Popup components
pub mod auth_popup;
pub mod confirmation_popup;
pub mod error_popup;
pub mod number_input_popup;
pub mod page_size_popup;
pub mod success_popup;

// Display components
pub mod help;
pub mod help_bar;
pub mod help_screen;
pub mod loading_indicator;
pub mod text_label;

// Complex components
pub mod message_details;
pub mod messages;

// System components
pub mod global_key_watcher;
