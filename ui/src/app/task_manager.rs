use crate::components::common::{LoadingActivityMsg, Msg};
use crate::error::AppError;
use server::taskpool::TaskPool;
use std::fmt::Display;
use std::future::Future;
use std::sync::mpsc::Sender;

/// Reusable task manager for async operations with loading indicators
/// Note: This is currently unused but designed for future migration of async patterns.
/// See docs/taskmanager.md for usage patterns and migration guidelines.
#[allow(dead_code)]
pub struct TaskManager {
    taskpool: TaskPool,
    tx_to_main: Sender<Msg>,
}

/// TaskManager implementation with async execution patterns
/// Note: Methods are unused pending migration from existing async patterns.
#[allow(dead_code)]
impl TaskManager {
    pub fn new(taskpool: TaskPool, tx_to_main: Sender<Msg>) -> Self {
        Self {
            taskpool,
            tx_to_main,
        }
    }

    /// Execute an async operation with loading indicator management
    ///
    /// # Arguments
    /// * `loading_message` - Message to show during loading
    /// * `operation` - Async operation to execute
    /// * `on_success` - Optional callback for successful completion
    /// * `on_error` - Optional callback for error handling
    pub fn execute_with_loading<F, R, S, E>(
        &self,
        loading_message: impl Display,
        operation: F,
        on_success: Option<S>,
        on_error: Option<E>,
    ) where
        F: Future<Output = Result<R, AppError>> + Send + 'static,
        R: Send + 'static,
        S: FnOnce(R, &Sender<Msg>) + Send + 'static,
        E: FnOnce(AppError, &Sender<Msg>) + Send + 'static,
    {
        // Start loading indicator
        Self::send_message_or_log_error(
            &self.tx_to_main,
            Msg::LoadingActivity(LoadingActivityMsg::Start(loading_message.to_string())),
            "loading start",
        );

        let tx_to_main = self.tx_to_main.clone();
        let tx_to_main_err = tx_to_main.clone();

        // Execute the async operation
        self.taskpool.execute(async move {
            let result = operation.await;

            // Stop loading indicator
            Self::send_message_or_log_error(
                &tx_to_main,
                Msg::LoadingActivity(LoadingActivityMsg::Stop),
                "loading stop",
            );

            match result {
                Ok(success_value) => {
                    if let Some(success_handler) = on_success {
                        success_handler(success_value, &tx_to_main);
                    }
                }
                Err(error) => {
                    if let Some(error_handler) = on_error {
                        error_handler(error, &tx_to_main_err);
                    } else {
                        // Default error handling
                        let _ = tx_to_main_err.send(Msg::Error(error));
                    }
                }
            }
        });
    }

    /// Execute an async operation with loading updates
    ///
    /// # Arguments
    /// * `initial_message` - Initial loading message
    /// * `operation` - Async operation that can send loading updates
    pub fn execute_with_updates<F, R>(&self, initial_message: impl Display, operation: F)
    where
        F: FnOnce(Sender<Msg>) -> Box<dyn Future<Output = Result<R, AppError>> + Send + Unpin>
            + Send
            + 'static,
        R: Send + 'static,
    {
        // Start loading indicator
        Self::send_message_or_log_error(
            &self.tx_to_main,
            Msg::LoadingActivity(LoadingActivityMsg::Start(initial_message.to_string())),
            "loading start",
        );

        let tx_to_main = self.tx_to_main.clone();
        let tx_to_main_err = tx_to_main.clone();

        self.taskpool.execute(async move {
            let operation_future = operation(tx_to_main.clone());
            let result = operation_future.await;

            // Stop loading indicator
            Self::send_message_or_log_error(
                &tx_to_main,
                Msg::LoadingActivity(LoadingActivityMsg::Stop),
                "loading stop",
            );

            if let Err(error) = result {
                let _ = tx_to_main_err.send(Msg::Error(error));
            }
        });
    }

    /// Execute a async operation with just loading start/stop
    pub fn execute<F, R>(&self, loading_message: impl Display, operation: F)
    where
        F: Future<Output = Result<R, AppError>> + Send + 'static,
        R: Send + 'static,
    {
        self.execute_with_loading(
            loading_message,
            operation,
            None::<fn(R, &Sender<Msg>)>,
            None::<fn(AppError, &Sender<Msg>)>,
        );
    }

    /// Helper method to send a message to the main thread or log an error if it fails
    pub fn send_message_or_log_error(tx: &Sender<Msg>, msg: Msg, context: &str) {
        if let Err(e) = tx.send(msg) {
            log::error!("Failed to send {} message: {}", context, e);
        }
    }
}

/// Builder pattern for more complex task configurations
/// Note: This is currently unused but designed for future migration.
/// See docs/taskmanager.md for usage patterns and migration guidelines.
#[allow(dead_code)]
pub struct TaskBuilder<'a> {
    task_manager: &'a TaskManager,
    loading_message: Option<String>,
    success_message: Option<String>,
    error_message: Option<String>,
}

/// TaskBuilder implementation for fluent configuration
/// Note: Methods are unused pending migration from existing async patterns.
#[allow(dead_code)]
impl<'a> TaskBuilder<'a> {
    pub fn new(task_manager: &'a TaskManager) -> Self {
        Self {
            task_manager,
            loading_message: None,
            success_message: None,
            error_message: None,
        }
    }

    pub fn loading_message(mut self, message: impl Display) -> Self {
        self.loading_message = Some(message.to_string());
        self
    }

    pub fn success_message(mut self, message: impl Display) -> Self {
        self.success_message = Some(message.to_string());
        self
    }

    pub fn error_message(mut self, message: impl Display) -> Self {
        self.error_message = Some(message.to_string());
        self
    }

    pub fn execute<F, R>(self, operation: F)
    where
        F: Future<Output = Result<R, AppError>> + Send + 'static,
        R: Send + 'static,
    {
        let loading_msg = self
            .loading_message
            .unwrap_or_else(|| "Loading...".to_string());

        let success_msg = self.success_message;
        let error_msg = self.error_message;

        let success_handler = success_msg.map(|msg| {
            move |_result: R, tx: &Sender<Msg>| {
                use crate::components::common::PopupActivityMsg;
                TaskManager::send_message_or_log_error(
                    tx,
                    Msg::PopupActivity(PopupActivityMsg::ShowSuccess(msg)),
                    "success popup",
                );
            }
        });

        let error_handler = error_msg.map(|_msg| {
            move |error: AppError, tx: &Sender<Msg>| {
                use crate::components::common::PopupActivityMsg;
                log::error!("Task failed: {}", error);
                TaskManager::send_message_or_log_error(
                    tx,
                    Msg::PopupActivity(PopupActivityMsg::ShowError(error)),
                    "error popup",
                );
            }
        });

        self.task_manager.execute_with_loading(
            loading_msg,
            operation,
            success_handler,
            error_handler,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::common::LoadingActivityMsg;
    use claims::*;
    use server::taskpool::TaskPool;
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::time::sleep;

    mod helpers {
        use super::*;

        // Helper to create a mock TaskPool and message channel
        pub fn create_test_setup() -> (TaskManager, mpsc::Receiver<Msg>) {
            let taskpool = TaskPool::new(4); // Use 4 threads for tests
            let (tx, rx) = mpsc::channel();
            let task_manager = TaskManager::new(taskpool, tx);
            (task_manager, rx)
        }

        // Helper to create an Unpin boxed future for execute_with_updates
        pub fn boxed_unpin<F, T>(future: F) -> Box<dyn Future<Output = T> + Send + Unpin>
        where
            F: Future<Output = T> + Send + 'static,
        {
            Box::new(Box::pin(future))
        }

        // Helper to collect messages with timeout
        pub fn collect_messages_with_timeout(
            rx: &mpsc::Receiver<Msg>,
            expected_count: usize,
            timeout_ms: u64,
        ) -> Vec<Msg> {
            let mut messages = Vec::new();
            let start = std::time::Instant::now();

            while messages.len() < expected_count
                && start.elapsed().as_millis() < timeout_ms as u128
            {
                match rx.recv_timeout(Duration::from_millis(50)) {
                    Ok(msg) => messages.push(msg),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // Continue waiting if we haven't reached the total timeout
                        continue;
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        break;
                    }
                }
            }

            // If we still haven't received all expected messages, try a few more times
            if messages.len() < expected_count {
                for _ in 0..10 {
                    match rx.try_recv() {
                        Ok(msg) => messages.push(msg),
                        Err(_) => std::thread::sleep(Duration::from_millis(10)),
                    }
                    if messages.len() >= expected_count {
                        break;
                    }
                }
            }

            messages
        }

        // Claims-powered assertions for common patterns
        pub fn assert_start_message(msg: &Msg, expected_text: &str) {
            assert_matches!(msg,
                Msg::LoadingActivity(LoadingActivityMsg::Start(text))
                if text == expected_text
            );
        }

        pub fn assert_stop_message(msg: &Msg) {
            assert_matches!(msg, Msg::LoadingActivity(LoadingActivityMsg::Stop));
        }

        pub fn assert_error_message(msg: &Msg, expected_error: &AppError) {
            assert_matches!(msg,
                Msg::Error(error)
                if error.to_string() == expected_error.to_string()
            );
        }

        pub fn assert_update_message(msg: &Msg, expected_text: &str) {
            assert_matches!(msg,
                Msg::LoadingActivity(LoadingActivityMsg::Update(text))
                if text == expected_text
            );
        }
    }

    // Unit tests - focused on individual behaviors and components
    mod unit {
        use super::*;
        use helpers::*;

        #[test]
        fn test_task_manager_creation() {
            let (_task_manager, _rx) = create_test_setup();
            // If we get here without panicking, TaskManager was created successfully
            // Test that we can create multiple instances
            let (_task_manager2, _rx2) = create_test_setup();
        }

        #[tokio::test]
        async fn test_execute_sends_start_message() {
            let (task_manager, rx) = create_test_setup();

            task_manager.execute("Test Message", async move { Ok::<(), AppError>(()) });

            let messages = collect_messages_with_timeout(&rx, 1, 1000);
            assert_ge!(messages.len(), 1, "Should receive at least start message");
            assert_start_message(&messages[0], "Test Message");
        }

        #[tokio::test]
        async fn test_execute_sends_stop_message_on_success() {
            let (task_manager, rx) = create_test_setup();

            task_manager.execute("Test", async move {
                sleep(Duration::from_millis(10)).await;
                Ok::<(), AppError>(())
            });

            sleep(Duration::from_millis(100)).await;
            let messages = collect_messages_with_timeout(&rx, 2, 1000);

            assert_ge!(messages.len(), 2, "Should receive start and stop messages");
            assert_stop_message(&messages[1]);
        }

        #[tokio::test]
        async fn test_execute_sends_stop_message_on_error() {
            let (task_manager, rx) = create_test_setup();

            task_manager.execute("Test", async move {
                Err::<(), AppError>(AppError::Config("test error".to_string()))
            });

            sleep(Duration::from_millis(100)).await;
            let messages = collect_messages_with_timeout(&rx, 3, 1000);

            assert_ge!(
                messages.len(),
                2,
                "Should receive at least start and stop messages"
            );
            assert_stop_message(&messages[1]);
        }

        #[tokio::test]
        async fn test_execute_propagates_error() {
            let (task_manager, rx) = create_test_setup();
            let expected_error = AppError::Config("Specific error".to_string());
            let expected_error_clone = expected_error.clone(); // Clone for comparison

            task_manager.execute("Test", async move { Err::<(), AppError>(expected_error) });

            sleep(Duration::from_millis(100)).await;
            let messages = collect_messages_with_timeout(&rx, 3, 1000);

            let error_msg = messages.iter().find_map(|msg| match msg {
                Msg::Error(error) => Some(error),
                _ => None,
            });

            assert_some!(error_msg);
            assert_error_message(&messages[2], &expected_error_clone);
        }

        #[tokio::test]
        async fn test_custom_success_handler_called() {
            let (task_manager, rx) = create_test_setup();
            let success_called = Arc::new(Mutex::new(false));
            let success_called_clone = success_called.clone();

            task_manager.execute_with_loading(
                "Test",
                async move { Ok::<String, AppError>("result".to_string()) },
                Some(move |result: String, _tx: &Sender<Msg>| {
                    assert_eq!(result, "result");
                    *success_called_clone.lock().unwrap() = true;
                }),
                None::<fn(AppError, &Sender<Msg>)>,
            );

            sleep(Duration::from_millis(100)).await;
            let _messages = collect_messages_with_timeout(&rx, 2, 1000);

            let guard = success_called.lock().unwrap();
            assert!(*guard, "Success handler was not called");
        }

        #[tokio::test]
        async fn test_custom_error_handler_called() {
            let (task_manager, rx) = create_test_setup();
            let error_called = Arc::new(Mutex::new(false));
            let error_called_clone = error_called.clone();
            let expected_error = AppError::Config("Test error".to_string());
            let expected_error_clone = expected_error.clone();

            task_manager.execute_with_loading(
                "Test",
                async move { Err::<(), AppError>(expected_error) },
                None::<fn((), &Sender<Msg>)>,
                Some(move |error: AppError, _tx: &Sender<Msg>| {
                    assert_eq!(error.to_string(), expected_error_clone.to_string());
                    *error_called_clone.lock().unwrap() = true;
                }),
            );

            sleep(Duration::from_millis(100)).await;
            let _messages = collect_messages_with_timeout(&rx, 2, 1000);

            let guard = error_called.lock().unwrap();
            assert!(*guard, "Error handler was not called");
        }

        #[tokio::test]
        async fn test_execute_with_updates_sends_progress() {
            let (task_manager, rx) = create_test_setup();

            task_manager.execute_with_updates("Test", |tx| {
                boxed_unpin(async move {
                    tx.send(Msg::LoadingActivity(LoadingActivityMsg::Update(
                        "Progress: 50%".to_string(),
                    )))
                    .unwrap();
                    Ok::<(), AppError>(())
                })
            });

            sleep(Duration::from_millis(100)).await;
            let messages = collect_messages_with_timeout(&rx, 3, 1000);

            // Find the update message
            let update_msg = messages
                .iter()
                .find(|msg| matches!(msg, Msg::LoadingActivity(LoadingActivityMsg::Update(_))));

            assert_some!(update_msg);
            assert_update_message(update_msg.unwrap(), "Progress: 50%");
        }

        #[test]
        fn test_task_builder_creation() {
            let (task_manager, _rx) = create_test_setup();

            let builder = TaskBuilder::new(&task_manager);

            // Verify initial state
            assert_none!(builder.loading_message);
            assert_none!(builder.success_message);
            assert_none!(builder.error_message);
        }

        #[test]
        fn test_task_builder_fluent_api() {
            let (task_manager, _rx) = create_test_setup();

            let builder = TaskBuilder::new(&task_manager)
                .loading_message("Custom loading...")
                .success_message("Operation successful!")
                .error_message("Operation failed!");

            assert_eq!(
                builder.loading_message,
                Some("Custom loading...".to_string())
            );
            assert_eq!(
                builder.success_message,
                Some("Operation successful!".to_string())
            );
            assert_eq!(builder.error_message, Some("Operation failed!".to_string()));
        }

        #[tokio::test]
        async fn test_builder_default_loading_message() {
            let (task_manager, rx) = create_test_setup();

            TaskBuilder::new(&task_manager)
                .success_message("Test")
                .execute(async move { Ok::<(), AppError>(()) });

            sleep(Duration::from_millis(100)).await;
            let messages = collect_messages_with_timeout(&rx, 3, 1000);

            assert_start_message(&messages[0], "Loading...");
        }

        #[test]
        fn test_send_message_or_log_error_success() {
            let (tx, rx) = mpsc::channel();
            let test_msg = Msg::LoadingActivity(LoadingActivityMsg::Stop);

            TaskManager::send_message_or_log_error(&tx, test_msg, "test");

            let received = assert_ok!(rx.try_recv());
            assert_matches!(received, Msg::LoadingActivity(LoadingActivityMsg::Stop));
        }

        #[test]
        fn test_send_message_or_log_error_failure() {
            let (tx, rx) = mpsc::channel();
            drop(rx); // Drop receiver to cause send error

            let test_msg = Msg::LoadingActivity(LoadingActivityMsg::Stop);

            // This should not panic, just log the error
            TaskManager::send_message_or_log_error(&tx, test_msg, "test");

            // If we reach here, the function handled the error gracefully
        }

        #[tokio::test]
        async fn test_display_formatting_for_loading_message() {
            let (task_manager, rx) = create_test_setup();

            // Test with different Display implementations
            task_manager.execute(42, async move { Ok::<(), AppError>(()) });
            task_manager.execute(format!("Dynamic {}", "message"), async move {
                Ok::<(), AppError>(())
            });

            sleep(Duration::from_millis(100)).await;
            let messages = collect_messages_with_timeout(&rx, 4, 2000);

            // Since operations run concurrently, we need to check all start messages
            let start_messages: Vec<&String> = messages
                .iter()
                .filter_map(|msg| match msg {
                    Msg::LoadingActivity(LoadingActivityMsg::Start(msg)) => Some(msg),
                    _ => None,
                })
                .collect();

            assert_eq!(start_messages.len(), 2);
            assert!(start_messages.contains(&&"42".to_string()));
            assert!(start_messages.contains(&&"Dynamic message".to_string()));
        }
    }
}
