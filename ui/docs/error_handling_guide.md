# Error Handling Guidelines

This guide outlines the proper patterns and practices for error handling in the Quetty application.

## Overview

The application uses a centralized `ErrorReporter` system that provides structured error handling with contextual information, appropriate UI responses, and comprehensive logging.

## Core Components

### ErrorReporter
Central system for reporting and handling errors throughout the application. Uses a unified architecture where Model creates a single ErrorReporter instance that is shared with TaskManager and components.

### ErrorContext
Provides rich context for errors including component and operation information.

### AppError Types
```rust
pub enum AppError {
    Io(String),           // File system errors (future-ready)
    ServiceBus(String),   // Azure Service Bus errors
    Component(String),    // UI component errors
    State(String),        // Application state errors
    Config(String),       // Configuration errors
}
```

### Error Severity Levels
- **Error**: Show error popup and log (default)
- **Critical**: Show error popup, log, and potentially exit (future-ready)

## Unified ErrorReporter Architecture

The application uses a **single source of truth** pattern:

1. **Model** creates one ErrorReporter instance
2. **TaskManager** receives this ErrorReporter via constructor
3. **Components** use Model's ErrorReporter directly (no duplication)

```rust
// Model initialization
let error_reporter = ErrorReporter::new(tx_to_main.clone());
let task_manager = TaskManager::new(taskpool, tx_to_main, error_reporter.clone());

// In components, use Model's error_reporter directly
app.error_reporter.report_simple(error, "ComponentName", "operation");
```

## Error Handling Patterns

### 1. Direct ErrorReporter Usage (Recommended)

For components with access to Model's ErrorReporter:

```rust
// In component methods that have access to Model
if let Err(error) = some_operation() {
    self.error_reporter.report_simple(error, "ComponentName", "operation");
}
```

### 2. Integration Helper (Legacy Pattern)

For testing or legacy integration:

```rust
// Note: Now located in tests/error_integration.rs
use error_integration::report_error_simple;

if let Err(error) = some_operation() {
    report_error_simple(&tx_to_main, error, "ComponentName", "operation_name");
}
```

### 3. Contextual Error Reporting

For scenarios requiring custom context:

```rust
use crate::error::{ErrorReporter, ErrorContext};

if let Err(error) = complex_operation() {
    let context = ErrorContext::new("ServiceBusManager", "message_processing")
        .with_user_message("Failed to process message. Please try again.");

    self.error_reporter.report(error, context);
}
```

### 4. TaskManager Error Handling

TaskManager receives ErrorReporter from Model and provides it to async operations:

```rust
// TaskManager already has ErrorReporter from Model
task_manager.execute_with_loading(
    "Operation",
    async_operation,
    None, // success handler
    Some(|error: AppError, error_reporter: &ErrorReporter| {
        error_reporter.report_simple(error, "TaskManager", "async_operation");
    })
);

// Or use simple execute (default error handling)
task_manager.execute("Loading data...", async_operation);
```

## Integration Guidelines

### Model and TaskManager Setup

```rust
// In Model initialization
let error_reporter = ErrorReporter::new(tx_to_main.clone());
let task_manager = TaskManager::new(taskpool, tx_to_main, error_reporter.clone());

// Store error_reporter in Model for component access
self.error_reporter = error_reporter;
```

### Component Error Handling

```rust
// In component methods (with access to Model)
impl<T> Model<T> where T: TerminalAdapter {
    pub fn component_operation(&mut self) -> Option<Msg> {
        if let Err(error) = self.some_operation() {
            self.error_reporter.report_simple(
                error, 
                "ComponentName", 
                "operation_name"
            );
            return None;
        }
        // ... rest of operation
    }
}
```

### Async Operations

```rust
// In async blocks, clone the error_reporter
let error_reporter = self.error_reporter.clone();
taskpool.execute(async move {
    if let Err(error) = async_operation().await {
        error_reporter.report_simple(error, "ComponentName", "async_operation");
    }
});
```

## Best Practices

### Architecture Principles

1. **Single Source of Truth**: Model creates one ErrorReporter instance
2. **No Duplication**: Pass ErrorReporter, don't create multiple instances
3. **Shared Access**: Components use Model's ErrorReporter directly
4. **Consistent Patterns**: Use same error reporting across all components

### Error Naming Guidelines

**Component Names**: Use descriptive names that identify the error source:
- "NamespacePicker" not "Picker"
- "QueuePicker" not "Component"
- "TaskManager" not "Manager"

**Operation Names**: Use specific operation descriptions:
- "load_namespaces" not "load"
- "send_message" not "send"
- "validate_configuration" not "validate"

### When to Use Each Pattern

1. **Direct ErrorReporter**: Primary pattern for all component operations
2. **Integration Helper**: Only for testing or legacy code integration
3. **Contextual Reporting**: When custom error messages are needed

## Testing Error Handling

### Unit Tests

```rust
#[test]
fn test_error_reporting() {
    let (tx, rx) = mpsc::channel();
    let error_reporter = ErrorReporter::new(tx);
    
    let error = AppError::Config("test error".to_string());
    error_reporter.report_simple(error, "TestComponent", "test_operation");
    
    // Verify error popup message
    let msg = rx.recv().unwrap();
    assert_matches!(msg, Msg::PopupActivity(PopupActivityMsg::ShowError(_)));
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_task_manager_error_handling() {
    let (tx, rx) = mpsc::channel();
    let error_reporter = ErrorReporter::new(tx.clone());
    let task_manager = TaskManager::new(taskpool, tx, error_reporter);
    
    // Test error handling
    task_manager.execute("Test", async move {
        Err::<(), AppError>(AppError::Config("test error".to_string()))
    });
    
    // Verify error was reported through popup system
    let messages = collect_messages(&rx);
    assert!(messages.iter().any(|msg| 
        matches!(msg, Msg::PopupActivity(PopupActivityMsg::ShowError(_)))
    ));
}
```

## Migration From Old Patterns

### Phase 1: Replace Direct Error Sending ✅ Complete
```rust
// Old pattern (replaced):
tx.send(Msg::Error(error))

// New pattern:
self.error_reporter.report_simple(error, "Component", "operation")
```

### Phase 2: Unified ErrorReporter Architecture ✅ Complete
- Model creates single ErrorReporter instance
- TaskManager receives ErrorReporter via constructor
- Components use Model's ErrorReporter directly

### Current Architecture Benefits

- **Single Source of Truth**: One ErrorReporter instance across the app
- **Better Performance**: No unnecessary ErrorReporter creation
- **Cleaner Ownership**: Clear hierarchy (Model → TaskManager → Components)
- **Consistent Error Handling**: Same patterns throughout the application
- **Zero Breaking Changes**: All existing functionality preserved

## Error Recovery Patterns

### Retry Logic
```rust
let mut attempts = 0;
const MAX_ATTEMPTS: u32 = 3;

loop {
    match risky_operation() {
        Ok(result) => return Ok(result),
        Err(error) if attempts < MAX_ATTEMPTS => {
            attempts += 1;
            self.error_reporter.report_simple(
                error, 
                "ComponentName", 
                &format!("operation_attempt_{}", attempts)
            );
            tokio::time::sleep(Duration::from_millis(1000 * attempts as u64)).await;
        }
        Err(error) => {
            self.error_reporter.report_simple(
                error,
                "ComponentName",
                "operation_final_failure"
            );
            return Err(error);
        }
    }
}
```

### Graceful Degradation
```rust
match critical_operation() {
    Ok(result) => result,
    Err(error) => {
        self.error_reporter.report_simple(error, "ServiceManager", "primary_operation");
        
        // Fall back to alternative approach
        match fallback_operation() {
            Ok(fallback_result) => {
                log::info!("Successfully used fallback operation");
                fallback_result
            }
            Err(fallback_error) => {
                self.error_reporter.report_simple(
                    fallback_error,
                    "ServiceManager", 
                    "fallback_operation"
                );
                return Err(fallback_error);
            }
        }
    }
}
```

## Monitoring and Logging

The ErrorReporter automatically handles:
- Structured logging with component and operation context
- Error popup integration for user notification
- Consistent error formatting across the application
- Technical detail capture for debugging

All errors are logged with full context in the format:
```
[Component:operation] User_Message (AppError_Details) - Technical: Details
```

This provides comprehensive error tracking while maintaining clean separation between user-facing messages and technical details. 
