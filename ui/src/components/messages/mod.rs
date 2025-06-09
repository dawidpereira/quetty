// Main Messages component module - split from original messages.rs
pub mod component;
pub mod rendering;
pub mod selection;

// Re-export main types for backwards compatibility
pub use component::{Messages, PaginationInfo};
