use crate::components::common::{LoadingActivityMsg, Msg};
use crate::error::{AppError, ErrorReporter};
use server::taskpool::TaskPool;
use std::fmt::Display;
use std::future::Future;
use std::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

/// Task manager for executing async operations with loading indicators and error handling
#[derive(Clone)]
pub struct TaskManager {
    taskpool: TaskPool,
    tx_to_main: Sender<Msg>,
    error_reporter: ErrorReporter,
}

impl TaskManager {
    pub fn new(taskpool: TaskPool, tx_to_main: Sender<Msg>, error_reporter: ErrorReporter) -> Self {
        Self {
            taskpool,
            tx_to_main,
            error_reporter,
        }
    }

    /// Simple execute method with default error handling (most common use case)
    pub fn execute<F, R>(&self, loading_message: impl Display, operation: F)
    where
        F: Future<Output = Result<R, AppError>> + Send + 'static,
        R: Send + 'static,
    {
        self.execute_with_cancellation(loading_message, operation, None);
    }

    /// Execute with cancellation support
    pub fn execute_with_cancellation<F, R>(
        &self,
        loading_message: impl Display,
        operation: F,
        cancel_token: Option<CancellationToken>,
    ) where
        F: Future<Output = Result<R, AppError>> + Send + 'static,
        R: Send + 'static,
    {
        // Start loading indicator
        Self::send_message_or_log_error(
            &self.tx_to_main,
            Msg::LoadingActivity(LoadingActivityMsg::Start(loading_message.to_string())),
            "loading start",
        );

        let tx_to_main = self.tx_to_main.clone();
        let error_reporter = self.error_reporter.clone();

        self.taskpool.execute(async move {
            let result = if let Some(token) = cancel_token {
                tokio::select! {
                    result = operation => result,
                    _ = token.cancelled() => {
                        log::info!("Task cancelled");
                        Err(AppError::Component("Operation cancelled".to_string()))
                    }
                }
            } else {
                operation.await
            };

            // Stop loading indicator
            Self::send_message_or_log_error(
                &tx_to_main,
                Msg::LoadingActivity(LoadingActivityMsg::Stop),
                "loading stop",
            );

            if let Err(error) = result {
                // Don't report cancellation as an error to the user
                if !error.to_string().contains("cancelled") {
                    error_reporter.report_simple(error, "TaskManager", "async_operation");
                }
            }
        });
    }

    /// Helper method to send a message to the main thread or log an error if it fails
    pub fn send_message_or_log_error(tx: &Sender<Msg>, msg: Msg, context: &str) {
        if let Err(e) = tx.send(msg) {
            log::error!("Failed to send {} message: {}", context, e);
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
