use claims::*;
use quetty::app::task_manager::TaskManager;
use quetty::components::common::{LoadingActivityMsg, PopupActivityMsg};
use quetty::error::ErrorReporter;
use quetty::{AppError, Msg};
use server::taskpool::TaskPool;
use std::sync::mpsc;
use std::time::Duration;
use tokio::time::sleep;

// Helper modules for integration tests
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

        while messages.len() < expected_count && start.elapsed().as_millis() < timeout_ms as u128 {
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

// Integration tests - testing complete workflows through public API
use helpers::*;

#[tokio::test]
async fn test_execute_success_complete_flow() {
    let (task_manager, rx) = create_test_setup();

    task_manager.execute("Testing execution", async move {
        // Simulate some work
        sleep(Duration::from_millis(10)).await;
        Ok::<i32, AppError>(42)
    });

    // Give some time for the task to execute
    sleep(Duration::from_millis(100)).await;

    // Collect messages: Start -> Stop
    let messages = collect_messages_with_timeout(&rx, 2, 2000);

    assert_eq!(messages.len(), 2, "Expected complete success workflow");
    assert_start_message(&messages[0], "Testing execution");
    assert_stop_message(&messages[1]);
}

#[tokio::test]
async fn test_execute_error_complete_flow() {
    let (task_manager, rx) = create_test_setup();

    let expected_error = AppError::Config("Test error".to_string());
    task_manager.execute("Testing error handling", async move {
        sleep(Duration::from_millis(10)).await;
        Err::<(), AppError>(expected_error)
    });

    // Give some time for the task to execute
    sleep(Duration::from_millis(100)).await;

    // Collect messages: Start -> Stop -> Error
    let messages = collect_messages_with_timeout(&rx, 3, 2000);

    assert_eq!(messages.len(), 3, "Expected complete error workflow");
    assert_start_message(&messages[0], "Testing error handling");
    assert_stop_message(&messages[1]);

    // Expect PopupActivity::ShowError with formatted error message
    assert_matches!(&messages[2],
        Msg::PopupActivity(PopupActivityMsg::ShowError(error))
        if error.to_string().contains("Configuration Error") &&
           error.to_string().contains("TaskManager")
    );
}



#[tokio::test]
async fn test_multiple_concurrent_operations_complete_flow() {
    let (task_manager, rx) = create_test_setup();

    // Execute multiple operations concurrently
    task_manager.execute("Operation 1", async move {
        sleep(Duration::from_millis(20)).await;
        Ok::<i32, AppError>(1)
    });

    task_manager.execute("Operation 2", async move {
        sleep(Duration::from_millis(10)).await;
        Ok::<i32, AppError>(2)
    });

    task_manager.execute("Operation 3", async move {
        sleep(Duration::from_millis(30)).await;
        Ok::<i32, AppError>(3)
    });

    sleep(Duration::from_millis(150)).await;
    let messages = collect_messages_with_timeout(&rx, 6, 3000);

    assert_eq!(messages.len(), 6, "Expected messages from all 3 operations");

    // Count start and stop messages
    let start_count = messages
        .iter()
        .filter(|msg| matches!(msg, Msg::LoadingActivity(LoadingActivityMsg::Start(_))))
        .count();
    let stop_count = messages
        .iter()
        .filter(|msg| matches!(msg, Msg::LoadingActivity(LoadingActivityMsg::Stop)))
        .count();

    assert_eq!(start_count, 3, "Should have 3 start messages");
    assert_eq!(stop_count, 3, "Should have 3 stop messages");
}


