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
    /// Informational - log only, no UI popup
    #[allow(dead_code)]
    Info,
    /// Warning severity - show warning popup and log
    Warning,
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
    pub suggestion: Option<String>,
    pub severity: ErrorSeverity,
}

impl ErrorContext {
    /// Create new error context with component and operation
    /// Uses generic message based on component/operation. Use .with_message() for custom messages.
    pub fn new(component: &str, operation: &str) -> Self {
        Self {
            component: component.to_string(),
            operation: operation.to_string(),
            user_message: Self::generate_fallback_message(component),
            technical_details: None,
            suggestion: None,
            severity: ErrorSeverity::Error,
        }
    }

    /// Generate simple generic fallback message
    /// The preferred approach is to use .with_message() for explicit user messages.
    fn generate_fallback_message(component: &str) -> String {
        format!("An error occurred in {}. Please try again.", component)
    }

    /// Builder pattern method for setting custom user message
    pub fn with_message(mut self, message: &str) -> Self {
        self.user_message = message.to_string();
        self
    }

    /// Builder pattern method for adding technical details (restored for debugging)
    pub fn with_technical_details(mut self, details: &str) -> Self {
        self.technical_details = Some(details.to_string());
        self
    }

    /// Builder pattern method for adding user suggestion
    pub fn with_suggestion(mut self, suggestion: &str) -> Self {
        self.suggestion = Some(suggestion.to_string());
        self
    }

    /// Builder pattern method for setting severity
    pub fn with_severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
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

    /// Report an informational message (log only, no UI popup)
    #[allow(dead_code)]
    pub fn report_info(&self, error: AppError, component: &str, operation: &str) {
        let context = ErrorContext::new(component, operation).with_severity(ErrorSeverity::Info);
        self.report(error, context);
    }

    /// Report a warning (shows warning popup)
    pub fn report_warning(&self, error: AppError, component: &str, operation: &str) {
        let context = ErrorContext::new(component, operation).with_severity(ErrorSeverity::Warning);
        self.report(error, context);
    }

    /// Report a critical error (shows error popup, logs extensively)
    /// Use this for errors that cause application termination or major system failures
    #[allow(dead_code)]
    pub fn report_critical(&self, error: AppError, component: &str, operation: &str) {
        let context =
            ErrorContext::new(component, operation).with_severity(ErrorSeverity::Critical);
        self.report(error, context);
    }

    /// Report a critical error that will cause application exit
    /// This method should be used when the error is severe enough to terminate the application
    #[allow(dead_code)]
    pub fn report_critical_and_exit(
        &self,
        error: AppError,
        component: &str,
        operation: &str,
        user_message: &str,
    ) {
        let context = ErrorContext::new(component, operation)
            .with_message(user_message)
            .with_severity(ErrorSeverity::Critical)
            .with_suggestion("The application will terminate. Please fix the issue and restart.");
        self.report(error, context);
    }

    /// Report error with detailed technical information (restored for service bus debugging)
    #[allow(dead_code)]
    pub fn report_detailed(
        &self,
        error: AppError,
        component: &str,
        operation: &str,
        user_message: &str,
        technical_details: &str,
        suggestion: &str,
    ) {
        let context = ErrorContext::new(component, operation)
            .with_message(user_message)
            .with_technical_details(technical_details)
            .with_suggestion(suggestion);
        self.report(error, context);
    }

    /// Report error with full context
    pub fn report(&self, error: AppError, context: ErrorContext) {
        let contextual_error = ContextualError::new(error.clone(), context.clone());

        // Enhanced logging with full context
        match context.severity {
            ErrorSeverity::Info => {
                log::info!(
                    "[{}:{}] {} {}",
                    context.component,
                    context.operation,
                    contextual_error,
                    self.format_additional_context(&context)
                );
            }
            ErrorSeverity::Warning => {
                log::warn!(
                    "[{}:{}] {} {}",
                    context.component,
                    context.operation,
                    contextual_error,
                    self.format_additional_context(&context)
                );
            }
            ErrorSeverity::Error => {
                log::error!(
                    "[{}:{}] {} {}",
                    context.component,
                    context.operation,
                    contextual_error,
                    self.format_additional_context(&context)
                );
            }
            ErrorSeverity::Critical => {
                log::error!(
                    "[CRITICAL] [{}:{}] {} {}",
                    context.component,
                    context.operation,
                    contextual_error,
                    self.format_additional_context(&context)
                );
            }
        }

        // Send to UI based on severity
        match context.severity {
            ErrorSeverity::Info => {
                // Info messages don't show UI popups, only log
            }
            ErrorSeverity::Warning => {
                let popup_msg = Msg::PopupActivity(PopupActivityMsg::ShowWarning(
                    self.format_user_message(&context),
                ));
                if let Err(e) = self.tx.send(popup_msg) {
                    log::error!("Failed to send warning popup message: {}", e);
                }
            }
            ErrorSeverity::Error | ErrorSeverity::Critical => {
                let formatted_error = self.create_formatted_error(&error, &context);
                let popup_msg = Msg::PopupActivity(PopupActivityMsg::ShowError(formatted_error));
                if let Err(e) = self.tx.send(popup_msg) {
                    log::error!("Failed to send error popup message: {}", e);
                }
            }
        }
    }

    /// Format additional context information for logging
    fn format_additional_context(&self, context: &ErrorContext) -> String {
        let mut parts = Vec::new();

        if let Some(ref technical_details) = context.technical_details {
            parts.push(format!("🔍 Technical: {}", technical_details));
        }

        if let Some(ref suggestion) = context.suggestion {
            parts.push(format!("💡 Suggestion: {}", suggestion));
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!("\n{}", parts.join("\n"))
        }
    }

    /// Format user-friendly message for UI display
    fn format_user_message(&self, context: &ErrorContext) -> String {
        let mut message = context.user_message.clone();

        if let Some(ref suggestion) = context.suggestion {
            message.push_str(&format!("\n\n💡 Suggestion: {}", suggestion));
        }

        message
    }

    /// Create a warning error with proper formatting
    pub fn create_warning_error(&self, message: String) -> AppError {
        // Create formatted warning with warning emoji
        let mut formatted_message = String::new();
        formatted_message.push_str("⚠️ Warning");
        formatted_message.push_str(&format!("\n\n{}", message));

        AppError::Component(formatted_message)
    }

    /// Create a beautifully formatted error for UI display
    fn create_formatted_error(&self, error: &AppError, context: &ErrorContext) -> AppError {
        let mut formatted_message = String::new();

        // Add appropriate emoji based on error type
        let emoji = match error {
            AppError::Config(_) => "⚙️",
            AppError::ServiceBus(_) => "🔗",
            AppError::Component(_) => "🎛️",
            AppError::State(_) => "📊",
            AppError::Io(_) => "📁",
        };

        // Add error title with emoji
        formatted_message.push_str(&format!("{} {}", emoji, self.get_error_title(error)));

        // Add main user message with proper formatting
        formatted_message.push_str(&format!("\n\n{}", context.user_message));

        // Add technical details if available (for debugging)
        if let Some(ref technical) = context.technical_details {
            formatted_message.push_str(&format!("\n\n🔍 Details: {}", technical));
        }

        // Add suggestion if available
        if let Some(ref suggestion) = context.suggestion {
            formatted_message.push_str(&format!("\n\n💡 Suggestion: {}", suggestion));
        }

        // Create a new AppError with the formatted message
        match error {
            AppError::Config(_) => AppError::Config(formatted_message),
            AppError::ServiceBus(_) => AppError::ServiceBus(formatted_message),
            AppError::Component(_) => AppError::Component(formatted_message),
            AppError::State(_) => AppError::State(formatted_message),
            AppError::Io(_) => AppError::Io(formatted_message),
        }
    }

    /// Get appropriate title for error type
    fn get_error_title(&self, error: &AppError) -> String {
        match error {
            AppError::Config(_) => "Configuration Error".to_string(),
            AppError::ServiceBus(_) => "Service Bus Error".to_string(),
            AppError::Component(_) => "Component Error".to_string(),
            AppError::State(_) => "Application State Error".to_string(),
            AppError::Io(_) => "File System Error".to_string(),
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
            "An error occurred in TestComponent. Please try again."
        );
    }

    #[test]
    fn test_contextual_error_with_custom_message() {
        let error = AppError::Config("Test error".to_string());
        let context =
            ErrorContext::new("TestComponent", "test_operation").with_message("Custom message");

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
            "An error occurred in TestComponent. Please try again."
        );
    }

    #[test]
    fn test_result_extension_trait() {
        let (tx, _rx) = mpsc::channel();
        let reporter = ErrorReporter::new(tx);
        let error = AppError::Config("Test error".to_string());

        // Test that error is reported when Result is Err
        let result: Result<(), AppError> = Err(error.clone());
        let returned_result = result.report_on_error(&reporter, "TestComponent", "test_operation");

        assert_eq!(returned_result, Err(error));
    }

    #[test]
    fn test_io_error_variant() {
        let error = AppError::Io("File not found".to_string());
        assert_eq!(error.to_string(), "IO Error: File not found");
    }

    #[test]
    fn test_critical_severity() {
        let context = ErrorContext::new("TestComponent", "test_operation")
            .with_severity(ErrorSeverity::Critical);

        matches!(context.severity, ErrorSeverity::Critical);
    }

    #[test]
    fn test_warning_severity_reporting() {
        let (tx, rx) = mpsc::channel();
        let reporter = ErrorReporter::new(tx);
        let error = AppError::Component("Warning message".to_string());

        // Test warning reporting
        reporter.report_warning(error, "TestComponent", "test_operation");

        // Verify warning message was sent
        let msg = rx.recv().unwrap();
        assert!(matches!(
            msg,
            Msg::PopupActivity(PopupActivityMsg::ShowWarning(_))
        ));
    }

    #[test]
    fn test_info_severity_no_popup() {
        let (tx, rx) = mpsc::channel();
        let reporter = ErrorReporter::new(tx);
        let error = AppError::Component("Info message".to_string());

        // Test info reporting (should not send popup)
        reporter.report_info(error, "TestComponent", "test_operation");

        // Verify no message was sent (info only logs)
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn test_user_friendly_message_generation() {
        let context = ErrorContext::new("MessageLoader", "load_messages");
        assert_eq!(
            context.user_message,
            "An error occurred in MessageLoader. Please try again."
        );

        let context = ErrorContext::new("UnknownComponent", "unknown_operation");
        assert_eq!(
            context.user_message,
            "An error occurred in UnknownComponent. Please try again."
        );
    }

    #[test]
    fn test_error_context_builder_pattern() {
        let context = ErrorContext::new("TestComponent", "test_operation")
            .with_message("Custom message")
            .with_technical_details("Technical information")
            .with_suggestion("Try this solution")
            .with_severity(ErrorSeverity::Warning);

        assert_eq!(context.user_message, "Custom message");
        assert_eq!(
            context.technical_details,
            Some("Technical information".to_string())
        );
        assert_eq!(context.suggestion, Some("Try this solution".to_string()));
        assert!(matches!(context.severity, ErrorSeverity::Warning));
    }

    #[test]
    fn test_contextual_error_display() {
        let error = AppError::Config("Test error".to_string());
        let context = ErrorContext::new("TestComponent", "test_operation");

        let contextual = ContextualError::new(error, context);
        let display_str = format!("{}", contextual);

        assert!(display_str.contains("TestComponent"));
        assert!(display_str.contains("Test error"));
    }

    #[test]
    fn test_new_with_message_method() {
        let context = ErrorContext::new("TestComponent", "test_operation")
            .with_message("Custom error message");

        assert_eq!(context.component, "TestComponent");
        assert_eq!(context.operation, "test_operation");
        assert_eq!(context.user_message, "Custom error message");
        assert!(matches!(context.severity, ErrorSeverity::Error));
    }

    #[test]
    fn test_critical_error_reporting() {
        let (tx, rx) = mpsc::channel();
        let reporter = ErrorReporter::new(tx);
        let error = AppError::Config("Critical configuration error".to_string());

        // Test critical error reporting
        reporter.report_critical(error, "Config", "load_config");

        // Verify message was sent
        let msg = rx.recv().unwrap();
        assert!(matches!(
            msg,
            Msg::PopupActivity(PopupActivityMsg::ShowError(_))
        ));
    }

    #[test]
    fn test_detailed_error_reporting() {
        let (tx, rx) = mpsc::channel();
        let reporter = ErrorReporter::new(tx);
        let error = AppError::ServiceBus("Service bus connection failed".to_string());

        // Test detailed error reporting (restored functionality)
        reporter.report_detailed(
            error.clone(),
            "ServiceBus",
            "connect",
            "Failed to connect to service bus",
            "Connection timeout after 30 seconds",
            "Check network connectivity and service bus configuration",
        );

        // Verify message was sent
        let msg = rx.recv().unwrap();
        assert!(matches!(
            msg,
            Msg::PopupActivity(PopupActivityMsg::ShowError(_))
        ));
    }

    #[test]
    fn test_format_additional_context_consistency() {
        let (tx, _rx) = mpsc::channel();
        let reporter = ErrorReporter::new(tx);

        let context = ErrorContext::new("TestComponent", "test_operation")
            .with_technical_details("Technical error details")
            .with_suggestion("Try this fix");

        let formatted = reporter.format_additional_context(&context);

        // Should use emojis and newline formatting like format_user_message
        assert!(formatted.contains("🔍 Technical:"));
        assert!(formatted.contains("💡 Suggestion:"));
        assert!(formatted.contains("\n"));
    }
}
