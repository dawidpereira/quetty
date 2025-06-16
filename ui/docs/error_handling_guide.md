# Quetty Error Handling Guide

This guide explains the error handling architecture in Quetty, which uses a unified `ErrorReporter` system for consistent error management across the entire application.

## Architecture Overview

### Unified ErrorReporter System

Quetty uses a centralized error reporting architecture where:

1. **Model** creates one `ErrorReporter` instance
2. **TaskManager** receives the `ErrorReporter` and shares it with async operations
3. **All components** use the same `ErrorReporter` instance for consistent error handling

```rust
// In Model initialization
let error_reporter = ErrorReporter::new(tx_to_main.clone());
let task_manager = TaskManager::new(taskpool, tx_to_main.clone(), error_reporter.clone());

// In components
self.error_reporter.report_simple(error, "ComponentName", "operation_name");
```

## Enhanced Error Reporting Features

### 🎯 Severity Levels

The system supports multiple severity levels with different behaviors:

```rust
// Error (default) - Shows error popup + logs
self.error_reporter.report_simple(error, "Component", "operation");

// Warning - Shows warning popup + logs
self.error_reporter.report_warning(error, "Component", "operation");

// Info - Logs only, no popup
self.error_reporter.report_info(error, "Component", "operation");

// Critical - Shows error popup + enhanced logging
self.error_reporter.report_critical(error, "Component", "operation");
```

### 💬 User-Friendly Messages

Errors automatically generate user-friendly messages based on component context:

```rust
// Automatic user-friendly messages
ErrorContext::new("MessageLoader", "load_messages")
// → "Failed to load messages. Please check your connection and try again."

ErrorContext::new("ThemeManager", "switch_theme")
// → "Theme change failed. The current theme will be preserved."

ErrorContext::new("BulkDeleteHandler", "delete_messages")
// → "Bulk delete operation failed. Some messages may not have been deleted."
```

### 🔧 Enhanced Context & Suggestions

For complex errors, provide detailed context and user suggestions:

```rust
// With suggestion
self.error_reporter.report_with_suggestion(
    error,
    "MessageStateHandler",
    "handle_consumer_created",
    "Failed to load messages after connecting to the queue",
    "Please check your connection and try refreshing the queue"
);

// With full context
self.error_reporter.report_detailed(
    error,
    "BulkDeleteHandler",
    "handle_bulk_delete_execution",
    "Cannot delete messages - not connected to queue",
    "Queue consumer is not initialized. This usually happens when the queue connection is lost.",
    "Please try reconnecting to the queue or refreshing the application"
);
```

### 🏗️ Builder Pattern for Context

Create rich error contexts using the builder pattern:

```rust
let context = ErrorContext::new("ComponentName", "operation_name")
    .with_user_message("Custom user-friendly message")
    .with_technical_details("Technical debugging information")
    .with_suggestion("What the user should try next")
    .with_severity(ErrorSeverity::Warning);

self.error_reporter.report(error, context);
```

## Error Types

### AppError Enum

```rust
pub enum AppError {
    Io(String),         // File system, network I/O
    ServiceBus(String), // Azure Service Bus operations
    Component(String),  // UI component errors
    State(String),      // Application state issues
    Config(String),     // Configuration problems
}
```

### Severity Classification

- **Info**: Informational messages, logging only
- **Warning**: Non-critical issues, shows warning popup
- **Error**: Standard errors, shows error popup
- **Critical**: Severe issues, enhanced logging and error popup

## Migration from Legacy Patterns

### Before (Legacy)
```rust
// Direct error returns
return Some(Msg::Error(e));

// Direct channel sends
let _ = tx_to_main_err.send(Msg::Error(e));
```

### After (Enhanced)
```rust
// Simple error reporting
self.error_reporter.report_simple(e, "ComponentName", "operation_name");
return None;

// Enhanced error reporting
self.error_reporter.report_with_suggestion(
    e,
    "ComponentName", 
    "operation_name",
    "User-friendly message",
    "Helpful suggestion"
);
```

## Implementation Examples

### Message Loading with Context
```rust
pub fn handle_consumer_created(&mut self, consumer: Consumer) -> Option<Msg> {
    self.queue_state.consumer = Some(Arc::new(Mutex::new(consumer)));
    
    if let Err(e) = self.load_messages() {
        self.error_reporter.report_with_suggestion(
            e,
            "MessageStateHandler",
            "handle_consumer_created",
            "Failed to load messages after connecting to the queue",
            "Please check your connection and try refreshing the queue"
        );
        return None;
    }
    None
}
```

### Theme Management with Warnings
```rust
if let Err(e) = manager.switch_theme_from_config(&theme_config) {
    // Theme errors are warnings since they don't break core functionality
    self.error_reporter.report_warning(e, "ThemeManager", "handle_theme_selected");
    return None;
}
```

### Bulk Operations with Detailed Context
```rust
let consumer = match model.queue_state.consumer.clone() {
    Some(consumer) => consumer,
    None => {
        model.error_reporter.report_detailed(
            AppError::State("No consumer available for bulk delete operation".to_string()),
            "BulkDeleteHandler",
            "handle_bulk_delete_execution",
            "Cannot delete messages - not connected to queue",
            "Queue consumer is not initialized. This usually happens when the queue connection is lost.",
            "Please try reconnecting to the queue or refreshing the application"
        );
        return None;
    }
};
```

## Async Operations

For async operations, clone the ErrorReporter:

```rust
let error_reporter = self.error_reporter.clone();
taskpool.execute(async move {
    let result = some_async_operation().await;
    if let Err(e) = result {
        error_reporter.report_simple(e, "AsyncComponent", "async_operation");
    }
});
```

## Testing Error Handling

### Basic Error Testing
```rust
#[test]
fn test_error_reporting() {
    let (tx, rx) = mpsc::channel();
    let reporter = ErrorReporter::new(tx);
    let error = AppError::Config("Test error".to_string());
    
    reporter.report_simple(error, "TestComponent", "test_operation");
    
    let msg = rx.recv().unwrap();
    assert!(matches!(msg, Msg::PopupActivity(PopupActivityMsg::ShowError(_))));
}
```

### Enhanced Features Testing
```rust
#[test]
fn test_warning_severity() {
    let (tx, rx) = mpsc::channel();
    let reporter = ErrorReporter::new(tx);
    
    reporter.report_warning(
        AppError::Component("Warning message".to_string()),
        "TestComponent",
        "test_operation"
    );
    
    let msg = rx.recv().unwrap();
    assert!(matches!(msg, Msg::PopupActivity(PopupActivityMsg::ShowWarning(_))));
}

#[test]
fn test_info_no_popup() {
    let (tx, rx) = mpsc::channel();
    let reporter = ErrorReporter::new(tx);
    
    reporter.report_info(
        AppError::Component("Info message".to_string()),
        "TestComponent",
        "test_operation"
    );
    
    // Info level should not send popup messages
    assert!(rx.try_recv().is_err());
}
```

## Best Practices

### 1. **Use Appropriate Severity Levels**
- `report_info()`: For debugging and informational messages
- `report_warning()`: For non-critical issues that don't break functionality
- `report_simple()`: For standard errors that affect functionality
- `report_critical()`: For severe errors that might require application restart

### 2. **Provide Context and Suggestions**
```rust
// Good: Helpful context and suggestion
self.error_reporter.report_with_suggestion(
    error,
    "MessageLoader",
    "load_messages",
    "Failed to load messages from Azure Service Bus",
    "Check your internet connection and Azure credentials"
);

// Bad: Generic error without context
self.error_reporter.report_simple(error, "Component", "operation");
```

### 3. **Component Naming Convention**
Use descriptive component names that indicate the functional area:
- `MessageLoader`, `MessageEditor`, `MessageStateHandler`
- `ThemeManager`, `NamespaceHandler`, `QueueHandler`
- `BulkDeleteHandler`, `BulkSendHandler`, `BulkSelection`

### 4. **Operation Naming Convention**
Use clear operation names that describe what was being attempted:
- `load_messages`, `save_configuration`, `connect_to_queue`
- `handle_user_selection`, `process_bulk_operation`
- `validate_input`, `update_ui_state`

### 5. **Return Consistency**
After reporting an error, always return `None` instead of `Some(Msg::Error(...))`:

```rust
// Correct pattern
if let Err(e) = operation() {
    self.error_reporter.report_simple(e, "Component", "operation");
    return None; // ← Always return None after error reporting
}
```

## Error Flow Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Model         │───▶│   TaskManager    │───▶│   Components    │
│ ┌─────────────┐ │    │ ┌──────────────┐ │    │ ┌─────────────┐ │
│ │ErrorReporter│ │    │ │ErrorReporter │ │    │ │ErrorReporter│ │
│ └─────────────┘ │    │ └──────────────┘ │    │ └─────────────┘ │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                                 ▼
                    ┌─────────────────────────┐
                    │    UI Error System      │
                    │                         │
                    │ • Error Popups          │
                    │ • Warning Popups        │
                    │ • Structured Logging    │
                    │ • User-Friendly Messages│
                    └─────────────────────────┘
```

## Summary

The enhanced error handling system provides:

✅ **Unified Architecture**: Single ErrorReporter instance across the entire application  
✅ **Severity Levels**: Info, Warning, Error, Critical with appropriate UI responses  
✅ **User-Friendly Messages**: Automatic generation of helpful error messages  
✅ **Rich Context**: Technical details, suggestions, and contextual information  
✅ **Builder Pattern**: Flexible error context creation  
✅ **Async Safety**: Clone-based sharing for thread-safe async operations  
✅ **Comprehensive Testing**: Full test coverage for all error reporting features  
✅ **Zero Breaking Changes**: Backward compatibility maintained  

This system ensures consistent, helpful, and user-friendly error handling throughout the Quetty application while providing developers with rich debugging information.
 