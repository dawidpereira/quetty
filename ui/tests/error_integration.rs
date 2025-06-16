use quetty::error::ErrorReporter;
use quetty::{AppError, components::common::Msg};
/// Integration helpers for error handling testing patterns
use std::sync::mpsc::Sender;

/// Quick helper for reporting errors in existing code patterns
///
/// # Example Usage in Existing Code:
/// ```no_run
/// use std::sync::mpsc;
/// use quetty::{AppError, components::common::Msg};
/// use error_integration::report_error_simple;
///
/// // Set up channel
/// let (tx, _rx) = mpsc::channel::<Msg>();
///
/// // Mock operation that might fail
/// fn some_operation() -> Result<(), AppError> {
///     Err(AppError::Config("test error".to_string()))
/// }
///
/// // New pattern with ErrorReporter:
/// if let Err(e) = some_operation() {
///     report_error_simple(&tx, e, "ComponentName", "operation_name");
/// }
/// ```
pub fn report_error_simple(tx: &Sender<Msg>, error: AppError, component: &str, operation: &str) {
    let reporter = ErrorReporter::new(tx.clone());
    reporter.report_simple(error, component, operation);
}

#[cfg(test)]
mod tests {
    use super::*;
    use quetty::components::common::{Msg, PopupActivityMsg};
    use std::sync::mpsc;

    #[test]
    fn test_report_error_simple() {
        let (tx, rx) = mpsc::channel();
        let error = AppError::Config("test error".to_string());

        report_error_simple(&tx, error, "TestComponent", "test_operation");

        let received = rx.recv().unwrap();
        match received {
            Msg::PopupActivity(PopupActivityMsg::ShowError(_)) => {
                // Expected
            }
            _ => panic!("Expected PopupActivity ShowError message"),
        }
    }
}
