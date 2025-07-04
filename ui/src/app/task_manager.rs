use crate::components::common::{LoadingActivityMsg, Msg};
use crate::error::{AppError, ErrorReporter};
use server::taskpool::TaskPool;
use std::collections::HashMap;
use std::fmt::Display;
use std::future::Future;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// Task manager for executing async operations with loading indicators and error handling
#[derive(Clone)]
pub struct TaskManager {
    taskpool: TaskPool,
    tx_to_main: Sender<Msg>,
    error_reporter: ErrorReporter,
    /// Active cancellation tokens for running operations
    active_operations: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

impl TaskManager {
    pub fn new(taskpool: TaskPool, tx_to_main: Sender<Msg>, error_reporter: ErrorReporter) -> Self {
        Self {
            taskpool,
            tx_to_main,
            error_reporter,
            active_operations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Execute an async operation with loading indicator and timeout support.
    /// Uses a default 30-second timeout to prevent hanging operations.
    /// For operations that need user cancellation, use execute_with_progress().
    pub fn execute<F, R>(&self, loading_message: impl Display, operation: F)
    where
        F: Future<Output = Result<R, AppError>> + Send + 'static,
        R: Send + 'static,
    {
        let timeout = Duration::from_secs(30); // Default 30 second timeout

        // Start loading indicator
        Self::send_message_or_report_error(
            &self.tx_to_main,
            Msg::LoadingActivity(LoadingActivityMsg::Start(loading_message.to_string())),
            "loading start",
            &self.error_reporter,
        );

        let tx_to_main = self.tx_to_main.clone();
        let error_reporter = self.error_reporter.clone();

        self.taskpool.execute(async move {
            let result = tokio::time::timeout(timeout, operation).await;

            let final_result = match result {
                Ok(operation_result) => operation_result,
                Err(_) => {
                    log::warn!("Operation timed out after {timeout:?}");
                    Err(AppError::Component(format!(
                        "Operation timed out after {} seconds",
                        timeout.as_secs()
                    )))
                }
            };

            // Stop loading indicator
            Self::send_message_or_report_error(
                &tx_to_main,
                Msg::LoadingActivity(LoadingActivityMsg::Stop),
                "loading stop",
                &error_reporter,
            );

            if let Err(error) = final_result {
                error_reporter.report_simple(error, "TaskManager", "async_operation_timeout");
            }
        });
    }

    /// Helper method to send a message to the main thread or report error if it fails
    pub fn send_message_or_report_error(
        tx: &Sender<Msg>,
        msg: Msg,
        context: &str,
        error_reporter: &ErrorReporter,
    ) {
        if let Err(e) = tx.send(msg) {
            error_reporter.report_send_error(context, e);
        }
    }

    pub fn execute_background<F, R>(&self, operation: F)
    where
        F: Future<Output = Result<R, AppError>> + Send + 'static,
        R: Send + 'static,
    {
        let error_reporter = self.error_reporter.clone();
        self.taskpool.execute(async move {
            if let Err(error) = operation.await {
                error_reporter.report_simple(error, "TaskManager", "async_operation_bg");
            }
            // no loading indicator messages
        });
    }

    /// Execute with progress reporting and user cancellation support
    pub fn execute_with_progress<F, R>(
        &self,
        loading_message: impl Display,
        operation_id: impl Display,
        operation: F,
    ) where
        F: FnOnce(
                ProgressReporter,
            )
                -> std::pin::Pin<Box<dyn Future<Output = Result<R, AppError>> + Send>>
            + Send
            + 'static,
        R: Send + 'static,
    {
        let operation_id = operation_id.to_string();
        let cancel_token = CancellationToken::new();

        // Store the cancellation token
        {
            let mut operations = self.active_operations.lock().unwrap();
            operations.insert(operation_id.clone(), cancel_token.clone());
        }

        // Start loading indicator with cancel button
        Self::send_message_or_report_error(
            &self.tx_to_main,
            Msg::LoadingActivity(LoadingActivityMsg::Start(loading_message.to_string())),
            "loading start",
            &self.error_reporter,
        );

        Self::send_message_or_report_error(
            &self.tx_to_main,
            Msg::LoadingActivity(LoadingActivityMsg::ShowCancelButton(operation_id.clone())),
            "show cancel button",
            &self.error_reporter,
        );

        let tx_to_main = self.tx_to_main.clone();
        let error_reporter = self.error_reporter.clone();
        let active_operations = self.active_operations.clone();
        let operation_id_cleanup = operation_id.clone();

        self.taskpool.execute(async move {
            let progress_reporter = ProgressReporter::new(tx_to_main.clone());
            let operation_future = operation(progress_reporter);

            let result = tokio::select! {
                result = operation_future => result,
                _ = cancel_token.cancelled() => {
                    log::info!("Operation '{operation_id}' cancelled by user");
                    Err(AppError::Component("Operation cancelled by user".to_string()))
                }
            };

            // Cleanup: remove from active operations
            {
                let mut operations = active_operations.lock().unwrap();
                operations.remove(&operation_id_cleanup);
            }

            // Hide cancel button and stop loading indicator
            Self::send_message_or_report_error(
                &tx_to_main,
                Msg::LoadingActivity(LoadingActivityMsg::HideCancelButton),
                "hide cancel button",
                &error_reporter,
            );

            Self::send_message_or_report_error(
                &tx_to_main,
                Msg::LoadingActivity(LoadingActivityMsg::Stop),
                "loading stop",
                &error_reporter,
            );

            if let Err(error) = result {
                // Don't report cancellation as an error to the user
                if !error.to_string().contains("cancelled") {
                    error_reporter.report_simple(error, "TaskManager", "async_operation_progress");
                }
            }
        });
    }

    /// Cancel an active operation by ID
    pub fn cancel_operation(&self, operation_id: &str) {
        let mut operations = self.active_operations.lock().unwrap();
        if let Some(token) = operations.remove(operation_id) {
            token.cancel();
            log::info!("Cancelled operation: {operation_id}");
        }
    }

    /// Get list of active operation IDs
    pub fn get_active_operations(&self) -> Vec<String> {
        let operations = self.active_operations.lock().unwrap();
        operations.keys().cloned().collect()
    }
}

/// Progress reporter for long-running operations
#[derive(Clone)]
pub struct ProgressReporter {
    tx_to_main: Sender<Msg>,
}

impl ProgressReporter {
    pub fn new(tx_to_main: Sender<Msg>) -> Self {
        Self { tx_to_main }
    }

    /// Report progress update to the UI
    pub fn report_progress(&self, message: impl Display) {
        if let Err(e) = self
            .tx_to_main
            .send(Msg::LoadingActivity(LoadingActivityMsg::Update(
                message.to_string(),
            )))
        {
            log::error!("Failed to send progress update: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::common::LoadingActivityMsg;
    use claims::*;
    use server::taskpool::TaskPool;
    use std::sync::mpsc;

    use std::time::Duration;
    use tokio::time::sleep;

    mod helpers {
        use super::*;

        // Helper to create a mock TaskPool and message channel
        pub fn create_test_setup() -> (TaskManager, mpsc::Receiver<Msg>) {
            let taskpool = TaskPool::new(4); // Use 4 threads for tests
            let (tx, rx) = mpsc::channel();
            let error_reporter = ErrorReporter::new(tx.clone());
            let task_manager = TaskManager::new(taskpool, tx, error_reporter);
            (task_manager, rx)
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

            task_manager.execute("Test", async move { Err::<(), AppError>(expected_error) });

            sleep(Duration::from_millis(100)).await;
            let messages = collect_messages_with_timeout(&rx, 3, 1000);

            // Look for PopupActivityMsg::ShowError instead of direct Msg::Error
            // because ErrorReporter now sends errors through the popup system
            let error_msg = messages.iter().find_map(|msg| match msg {
                Msg::PopupActivity(crate::components::common::PopupActivityMsg::ShowError(
                    error,
                )) => Some(error),
                _ => None,
            });

            assert_some!(error_msg);
            // The error should be formatted nicely but still contain the original message
            let error_str = error_msg.expect("Error message should be Some").to_string();
            assert!(error_str.contains("⚙️"), "Should contain config emoji");
            assert!(
                error_str.contains("Configuration Error"),
                "Should contain error title"
            );
            assert!(
                error_str.contains("TaskManager"),
                "Should contain component info"
            );
        }

        #[test]
        fn test_send_message_or_report_error_success() {
            let (tx, rx) = mpsc::channel();
            let error_reporter = ErrorReporter::new(tx.clone());
            let test_msg = Msg::LoadingActivity(LoadingActivityMsg::Stop);

            TaskManager::send_message_or_report_error(&tx, test_msg, "test", &error_reporter);

            let received = assert_ok!(rx.try_recv());
            assert_matches!(received, Msg::LoadingActivity(LoadingActivityMsg::Stop));
        }

        #[test]
        fn test_send_message_or_report_error_failure() {
            let (tx, rx) = mpsc::channel();
            let error_reporter = ErrorReporter::new(tx.clone());
            drop(rx); // Drop receiver to cause send error

            let test_msg = Msg::LoadingActivity(LoadingActivityMsg::Stop);

            // This should not panic, just report the error
            TaskManager::send_message_or_report_error(&tx, test_msg, "test", &error_reporter);

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

        #[tokio::test]
        async fn test_execute_timeout_behavior() {
            let (task_manager, rx) = create_test_setup();

            task_manager.execute("Test timeout operation", async move {
                sleep(Duration::from_millis(10)).await;
                Ok::<i32, AppError>(42)
            });

            sleep(Duration::from_millis(100)).await;
            let messages = collect_messages_with_timeout(&rx, 2, 2000);

            assert_eq!(messages.len(), 2);
            assert_start_message(&messages[0], "Test timeout operation");
            assert_stop_message(&messages[1]);
        }

        #[tokio::test]
        async fn test_cancel_operation() {
            let (task_manager, _rx) = create_test_setup();

            // Start a long-running operation
            let operation_id = "test_operation";
            task_manager.execute_with_progress(
                "Long running operation",
                operation_id,
                move |_progress| {
                    Box::pin(async move {
                        sleep(Duration::from_secs(10)).await; // Very long operation
                        Ok::<(), AppError>(())
                    })
                },
            );

            sleep(Duration::from_millis(50)).await;

            // Verify operation is active
            let active_ops = task_manager.get_active_operations();
            assert_eq!(active_ops.len(), 1);
            assert_eq!(active_ops[0], operation_id);

            // Cancel the operation
            task_manager.cancel_operation(operation_id);

            // Wait a bit for cancellation to take effect
            sleep(Duration::from_millis(100)).await;

            // Verify operation is no longer active
            let active_ops = task_manager.get_active_operations();
            assert_eq!(active_ops.len(), 0);
        }

        #[tokio::test]
        async fn test_progress_reporter() {
            let (task_manager, rx) = create_test_setup();

            task_manager.execute_with_progress(
                "Progress test operation",
                "progress_test",
                move |progress| {
                    Box::pin(async move {
                        progress.report_progress("Step 1 of 3");
                        sleep(Duration::from_millis(10)).await;
                        progress.report_progress("Step 2 of 3");
                        sleep(Duration::from_millis(10)).await;
                        progress.report_progress("Step 3 of 3");
                        Ok::<(), AppError>(())
                    })
                },
            );

            sleep(Duration::from_millis(200)).await;
            let messages = collect_messages_with_timeout(&rx, 8, 2000);

            // Should have: Start, ShowCancelButton, Update x3, HideCancelButton, Stop
            assert!(messages.len() >= 7);

            // Find progress updates
            let progress_updates: Vec<&String> = messages
                .iter()
                .filter_map(|msg| match msg {
                    Msg::LoadingActivity(LoadingActivityMsg::Update(msg)) => Some(msg),
                    _ => None,
                })
                .collect();

            assert_eq!(progress_updates.len(), 3);
            assert_eq!(progress_updates[0], "Step 1 of 3");
            assert_eq!(progress_updates[1], "Step 2 of 3");
            assert_eq!(progress_updates[2], "Step 3 of 3");
        }
    }
}
