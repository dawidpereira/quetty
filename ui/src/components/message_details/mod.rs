// Main MessageDetails component module - split from original message_details.rs
pub mod component;
pub mod editing;
pub mod event_handling;
pub mod navigation;
pub mod rendering;

// Re-export main types for backwards compatibility
pub use component::MessageDetails;
