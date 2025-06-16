use crate::components::common::{Msg, PopupActivityMsg};
use std::fmt::Display;
use std::sync::mpsc::Sender;

/// Application-wide error types
#[derive(Debug, Clone, PartialEq)]
pub enum AppError {
    /// Input/Output errors (file operations, etc.)
    #[allow(dead_code)]
    Io(String),
    /// Azure Service Bus related errors
    ServiceBus(String),
    /// Component-related errors (UI, state management)
    Component(String),
    /// State-related errors (application state issues)
    State(String),
    /// Configuration errors
    Config(String),
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Io(msg) => write!(f, "IO Error: {}", msg),
            AppError::ServiceBus(msg) => write!(f, "Service Bus Error: {}", msg),
            AppError::Component(msg) => write!(f, "Component Error: {}", msg),
            AppError::State(msg) => write!(f, "State Error: {}", msg),
            AppError::Config(msg) => write!(f, "Configuration Error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

/// Result type alias for application operations
pub type AppResult<T> = Result<T, AppError>;

/// Error severity levels for appropriate UI response
#[derive(Debug, Clone)]
pub enum ErrorSeverity {
    /// High severity - show error popup and log
    Error,
    /// Critical severity - show error popup, log, and potentially exit
    #[allow(dead_code)]
    Critical,
}

/// Context information for errors
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub component: String,
    pub operation: String,
    pub user_message: String,
    pub technical_details: Option<String>,
    pub severity: ErrorSeverity,
}

impl ErrorContext {
    pub fn new(component: &str, operation: &str) -> Self {
        Self {
            component: component.to_string(),
            operation: operation.to_string(),
            user_message: format!("Error in {} during {}", component, operation),
            technical_details: None,
            severity: ErrorSeverity::Error,
        }
    }

    /// Builder pattern method for setting user message
    pub fn with_user_message(mut self, message: &str) -> Self {
        self.user_message = message.to_string();
        self
    }
}

/// Contextual error with rich information
#[derive(Debug, Clone)]
pub struct ContextualError {
    pub error: AppError,
    pub context: ErrorContext,
}

impl ContextualError {
    pub fn new(error: AppError, context: ErrorContext) -> Self {
        Self { error, context }
    }
}

impl Display for ContextualError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.context.user_message, self.error)
    }
}

/// Central error reporting system
#[derive(Clone)]
pub struct ErrorReporter {
    tx: Sender<Msg>,
}

impl ErrorReporter {
    pub fn new(tx: Sender<Msg>) -> Self {
        Self { tx }
    }

    /// Report a simple error with basic context
    pub fn report_simple(&self, error: AppError, component: &str, operation: &str) {
        let context = ErrorContext::new(component, operation);
        self.report(error, context);
    }

    /// Report error with full context
    pub fn report(&self, error: AppError, context: ErrorContext) {
        let contextual_error = ContextualError::new(error.clone(), context.clone());

        // Log the error with full context
        log::error!(
            "[{}:{}] {} - Technical: {}",
            context.component,
            context.operation,
            contextual_error,
            context.technical_details.as_deref().unwrap_or("None")
        );

        // Send to UI based on severity
        match context.severity {
            ErrorSeverity::Error | ErrorSeverity::Critical => {
                let popup_msg = Msg::PopupActivity(PopupActivityMsg::ShowError(error));
                if let Err(e) = self.tx.send(popup_msg) {
                    log::error!("Failed to send error popup message: {}", e);
                }
            }
        }
    }
}

/// Extension trait for Result types to simplify error reporting
pub trait ResultExt<T> {
    /// Report error if Result is Err, with basic context
    #[allow(dead_code)]
    fn report_on_error(self, reporter: &ErrorReporter, component: &str, operation: &str) -> Self;
}

impl<T> ResultExt<T> for Result<T, AppError> {
    fn report_on_error(self, reporter: &ErrorReporter, component: &str, operation: &str) -> Self {
        if let Err(ref e) = self {
            reporter.report_simple(e.clone(), component, operation);
        }
        self
    }
}

/// Legacy error handling function - kept for backward compatibility
pub fn handle_error(error: AppError) {
    // Log the error with appropriate level based on error type
    match &error {
        AppError::Io(msg) => log::error!("IO Error: {}", msg),
        AppError::ServiceBus(msg) => log::error!("Service Bus Error: {}", msg),
        AppError::Component(msg) => log::warn!("Component Error: {}", msg),
        AppError::State(msg) => log::warn!("State Error: {}", msg),
        AppError::Config(msg) => log::warn!("Configuration Error: {}", msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn test_error_context_creation() {
        let context = ErrorContext::new("TestComponent", "test_operation");
        assert_eq!(context.component, "TestComponent");
        assert_eq!(context.operation, "test_operation");
        assert_eq!(
            context.user_message,
            "Error in TestComponent during test_operation"
        );
    }

    #[test]
    fn test_contextual_error_with_custom_message() {
        let error = AppError::Config("Test error".to_string());
        let context = ErrorContext::new("TestComponent", "test_operation")
            .with_user_message("Custom message");

        let contextual = ContextualError::new(error, context);
        assert_eq!(contextual.context.user_message, "Custom message");
    }

    #[test]
    fn test_contextual_error_default_message() {
        let error = AppError::Config("Test error".to_string());
        let context = ErrorContext::new("TestComponent", "test_operation");

        let contextual = ContextualError::new(error, context);
        assert_eq!(
            contextual.context.user_message,
            "Error in TestComponent during test_operation"
        );
    }

    #[test]
    fn test_result_extension_trait() {
        let (tx, _rx) = mpsc::channel();
        let error_reporter = ErrorReporter::new(tx);

        let result: Result<(), AppError> = Err(AppError::Config("Test error".to_string()));
        let _handled = result.report_on_error(&error_reporter, "TestComponent", "test_operation");
    }

    #[test]
    fn test_io_error_variant() {
        let error = AppError::Io("File not found".to_string());
        assert_eq!(error.to_string(), "IO Error: File not found");
    }

    #[test]
    fn test_critical_severity() {
        let context = ErrorContext::new("TestComponent", "test_operation");
        // Test that we can access and use the Critical variant
        let context_with_critical = ErrorContext {
            severity: ErrorSeverity::Critical,
            ..context
        };

        match context_with_critical.severity {
            ErrorSeverity::Critical => {
                // Expected - this ensures the Critical variant is used
            }
            _ => panic!("Expected Critical severity"),
        }
    }
}
