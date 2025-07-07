# TaskManager Guideline

## Overview

The `TaskManager` is the standard component for handling async operations in the Quetty application. It provides consistent loading indicators, error handling, and task execution patterns across the codebase.

## Core Benefits

- **Consistent loading management**: Automatic start/stop of loading indicators
- **Standardized error handling**: Centralized error forwarding and logging
- **Type safety**: Generic operations with compile-time validation
- **Clean separation of concerns**: Business logic separated from UI state management
- **Testability**: Easily mockable for unit testing

## Core Components

### TaskManager

The main component that handles async operations with automatic loading indicators.

```rust
use crate::app::task_manager::TaskManager;

let task_manager = TaskManager::new(self.taskpool.clone(), self.tx_to_main.clone());
```

### TaskBuilder

A builder pattern for complex task configurations with success/error messages.

```rust
use crate::app::task_manager::TaskBuilder;

TaskBuilder::new(&task_manager)
    .loading_message("Processing...")
    .success_message("Operation completed successfully!")
    .execute(async { /* operation */ });
```

## Usage Patterns

### 1. Simple Operations

Use `execute()` for basic async operations that only need loading indicators:

```rust
let task_manager = TaskManager::new(self.taskpool.clone(), self.tx_to_main.clone());

task_manager.execute("Loading namespaces...", async move {
    let namespaces = service_bus.list_namespaces().await?;
    tx_to_main.send(Msg::NamespaceUpdate(NamespaceUpdateMsg::LoadSuccess(namespaces)))?;
    Ok(())
});
```

**When to use**: Basic operations that just need loading start/stop and default error handling.

### 2. Operations with Custom Handlers

Use `execute_with_loading()` when you need specific success or error handling:

```rust
let task_manager = TaskManager::new(self.taskpool.clone(), self.tx_to_main.clone());

task_manager.execute_with_loading(
    "Deleting messages...",
    async move {
        let count = delete_operation().await?;
        Ok(count)
    },
    Some(|count, tx| {
        let success_msg = format!("Successfully deleted {} messages", count);
        tx.send(Msg::PopupActivity(PopupActivityMsg::ShowSuccess(success_msg)))?;
    }),
    Some(|error, tx| {
        tx.send(Msg::PopupActivity(PopupActivityMsg::ShowError(error)))?;
    }),
);
```

**When to use**: Operations that need custom success messages, specific error handling, or need to process the result.

### 3. Long-Running Operations with Progress

Use `execute_with_updates()` for operations that provide progress feedback:

```rust
let task_manager = TaskManager::new(self.taskpool.clone(), self.tx_to_main.clone());

task_manager.execute_with_updates("Processing bulk operation...", |tx| {
    Box::new(async move {
        let total = items.len();
        for (index, item) in items.iter().enumerate() {
            // Process the item
            process_item(item).await?;

            // Send progress update
            let progress_msg = format!("Processed {}/{} items", index + 1, total);
            tx.send(Msg::LoadingActivity(LoadingActivityMsg::Update(progress_msg)))?;
        }
        Ok(())
    })
});
```

**When to use**: Long-running operations where users need progress feedback.

### 4. Builder Pattern for Rich UI Feedback

Use `TaskBuilder` when you want success/error popups with custom messages:

```rust
let task_manager = TaskManager::new(self.taskpool.clone(), self.tx_to_main.clone());

TaskBuilder::new(&task_manager)
    .loading_message("Sending messages...")
    .success_message("All messages sent successfully!")
    .error_message("Failed to send messages")
    .execute(async move {
        let results = send_messages_batch(messages).await?;
        Ok(results)
    });
```

**When to use**: User-initiated actions that need clear success/failure feedback via popups.

## API Reference

### TaskManager

#### Constructor
```rust
TaskManager::new(taskpool: TaskPool, tx_to_main: Sender<Msg>) -> Self
```
Creates a new TaskManager instance with the provided task pool and message channel.

#### Methods

##### `execute<F, R>(&self, loading_message: impl Display, operation: F)`
Executes a simple async operation with automatic loading management.

**Parameters:**
- `loading_message`: Text displayed during the operation
- `operation`: Async function returning `Result<R, AppError>`

**Usage:**
```rust
task_manager.execute("Loading data...", async {
    let data = fetch_data().await?;
    process_data(data)?;
    Ok(())
});
```

##### `execute_with_loading<F, R, S, E>(&self, loading_message, operation, on_success, on_error)`
Executes an async operation with custom success and error handlers.

**Parameters:**
- `loading_message`: Text displayed during the operation
- `operation`: Async function returning `Result<R, AppError>`
- `on_success`: Optional callback `FnOnce(R, &Sender<Msg>)` for success handling
- `on_error`: Optional callback `FnOnce(AppError, &Sender<Msg>)` for error handling

**Usage:**
```rust
task_manager.execute_with_loading(
    "Saving configuration...",
    async { save_config().await },
    Some(|result, tx| {
        tx.send(Msg::ConfigSaved(result))?;
    }),
    None, // Use default error handling
);
```

##### `execute_with_updates<F, R>(&self, initial_message, operation)`
Executes an async operation that can send progress updates.

**Parameters:**
- `initial_message`: Initial loading message
- `operation`: Function that receives `Sender<Msg>` and returns a boxed future

**Usage:**
```rust
task_manager.execute_with_updates("Importing data...", |tx| {
    Box::new(async move {
        for (i, batch) in batches.iter().enumerate() {
            import_batch(batch).await?;
            tx.send(Msg::LoadingActivity(LoadingActivityMsg::Update(
                format!("Imported batch {}/{}", i + 1, batches.len())
            )))?;
        }
        Ok(())
    })
});
```

### TaskBuilder

#### Constructor
```rust
TaskBuilder::new(task_manager: &TaskManager) -> Self
```

#### Configuration Methods

##### `loading_message(self, message: impl Display) -> Self`
Sets the loading message displayed during operation.

##### `success_message(self, message: impl Display) -> Self`
Sets the success popup message. When set, shows a success popup on completion.

##### `error_message(self, message: impl Display) -> Self`
Sets the error popup message prefix. When set, shows error popups instead of default error handling.

#### Execution

##### `execute<F, R>(self, operation: F)`
Executes the configured operation with the specified settings.

**Usage:**
```rust
TaskBuilder::new(&task_manager)
    .loading_message("Uploading file...")
    .success_message("File uploaded successfully!")
    .execute(async move {
        upload_file(file_data).await?;
        Ok(())
    });
```

## Design Patterns

### Error Handling Strategy

TaskManager follows a consistent error handling approach:

1. **Default behavior**: Errors are forwarded to `Msg::Error(AppError)`
2. **Custom handlers**: Use `execute_with_loading()` for specific error handling
3. **Builder pattern**: Use `TaskBuilder` for popup-based error display
4. **Logging**: All channel send failures are automatically logged

### Loading State Management

Loading indicators are automatically managed:

1. **Start**: Sent before operation begins
2. **Updates**: Available in `execute_with_updates()` pattern
3. **Stop**: Always sent when operation completes (success or failure)
4. **Error safety**: Loading stops even if operation panics

### Channel Management

TaskManager handles message channel cloning and error handling:

1. **Automatic cloning**: Channels are cloned as needed for async contexts
2. **Send error logging**: Failed sends are logged rather than panicking
3. **Error channel separation**: Separate channels for success/error to avoid conflicts

## Best Practices

### Choosing the Right Method

1. **`execute()`**: Default choice for simple operations
2. **`execute_with_loading()`**: When you need to process results or custom error handling
3. **`execute_with_updates()`**: Only for operations that genuinely need progress feedback
4. **`TaskBuilder`**: For user-facing operations that need success/error popups

### Error Handling

```rust
// Good: Let TaskManager handle default errors
task_manager.execute("Loading...", async {
    let data = risky_operation().await?; // ? operator works
    Ok(data)
});

// Good: Custom error handling when needed
task_manager.execute_with_loading(
    "Loading...",
    async { risky_operation().await },
    Some(|data, tx| { /* handle success */ }),
    Some(|error, tx| { /* handle specific error */ }),
);
```

### Progress Updates

```rust
// Good: Meaningful progress updates
task_manager.execute_with_updates("Processing files...", |tx| {
    Box::new(async move {
        for (i, file) in files.iter().enumerate() {
            process_file(file).await?;
            if i % 10 == 0 { // Update every 10 files
                tx.send(Msg::LoadingActivity(LoadingActivityMsg::Update(
                    format!("Processed {}/{} files", i + 1, files.len())
                )))?;
            }
        }
        Ok(())
    })
});
```

### Builder Pattern Usage

```rust
// Good: Clear, fluent configuration
TaskBuilder::new(&task_manager)
    .loading_message("Connecting to server...")
    .success_message("Connection established!")
    .execute(async { establish_connection().await });

// Avoid: Overusing builder for simple operations
// Just use task_manager.execute() instead
```

## Testing

### Unit Testing Operations

Separate business logic from TaskManager for easier testing:

```rust
// Testable business logic
async fn load_user_data(user_id: u32) -> Result<UserData, AppError> {
    // Business logic here
}

// TaskManager integration
task_manager.execute("Loading user...", async move {
    let data = load_user_data(user_id).await?;
    update_ui_with_user_data(data)?;
    Ok(())
});

// Test the business logic separately
#[tokio::test]
async fn test_load_user_data() {
    let result = load_user_data(123).await;
    assert!(result.is_ok());
}
```

### Mocking TaskManager

For integration tests, TaskManager can be mocked:

```rust
struct MockTaskManager;

impl MockTaskManager {
    fn execute<F, R>(&self, _message: impl Display, operation: F)
    where F: Future<Output = Result<R, AppError>> + Send + 'static
    {
        // Execute immediately in tests or capture for verification
        tokio::spawn(operation);
    }
}
```

## Common Patterns

### Loading Data on Component Init

```rust
fn init_component(&mut self) {
    let task_manager = TaskManager::new(self.taskpool.clone(), self.tx_to_main.clone());

    task_manager.execute("Loading initial data...", async move {
        let data = fetch_initial_data().await?;
        tx_to_main.send(Msg::DataLoaded(data))?;
        Ok(())
    });
}
```

### User-Initiated Actions

```rust
fn handle_delete_action(&mut self, item_id: u32) {
    let task_manager = TaskManager::new(self.taskpool.clone(), self.tx_to_main.clone());

    TaskBuilder::new(&task_manager)
        .loading_message("Deleting item...")
        .success_message("Item deleted successfully!")
        .execute(async move {
            delete_item(item_id).await?;
            tx_to_main.send(Msg::ItemDeleted(item_id))?;
            Ok(())
        });
}
```

### Bulk Operations

```rust
fn process_bulk_items(&mut self, items: Vec<Item>) {
    let task_manager = TaskManager::new(self.taskpool.clone(), self.tx_to_main.clone());

    task_manager.execute_with_updates("Processing items...", |tx| {
        Box::new(async move {
            let total = items.len();
            for (index, item) in items.iter().enumerate() {
                process_item(item).await?;

                if index % 5 == 0 {
                    tx.send(Msg::LoadingActivity(LoadingActivityMsg::Update(
                        format!("Processed {}/{} items", index + 1, total)
                    )))?;
                }
            }
            Ok(())
        })
    });
}
```

## Troubleshooting

### Common Issues

**Generic type inference errors:**
```rust
// If you get type inference errors, specify types explicitly
task_manager.execute::<_, ()>("message", async { Ok(()) });
```

**Channel send errors:**
```rust
// TaskManager logs send errors automatically
// Check application logs if operations seem to hang
```

**Loading indicator not showing:**
```rust
// Ensure your async operation returns Result<T, AppError>
// TaskManager only starts loading for operations that return Results
```

**Operations not executing:**
```rust
// Verify taskpool is properly initialized
// Check that tx_to_main channel receiver is being processed
```
