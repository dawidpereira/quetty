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
pub mod theme_picker;

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
