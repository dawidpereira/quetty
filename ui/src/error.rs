use crate::components::common::{Msg, PopupActivityMsg};
use std::fmt::Display;
use std::sync::mpsc::Sender;

/// Application-wide error types for the Quetty terminal user interface.
///
/// This enum provides comprehensive error classification for all UI operations
/// including Service Bus interactions, component lifecycle, state management,
/// and configuration handling. Each error variant is designed to integrate
/// seamlessly with the UI error reporting and user feedback systems.
///
/// # Error Categories
///
/// ## External Service Errors
/// - [`ServiceBus`] - Azure Service Bus operation failures
/// - [`Auth`] - Authentication and authorization errors
///
/// ## Application Infrastructure Errors
/// - [`Component`] - UI component lifecycle and rendering errors
/// - [`State`] - Application state management issues
/// - [`Channel`] - Inter-component communication failures
///
/// ## System and Configuration Errors
/// - [`Io`] - File system and I/O operation failures
/// - [`Config`] - Configuration loading and validation errors
///
/// # Examples
///
/// ## Basic Error Handling with User Feedback
/// ```no_run
/// use ui::error::{AppError, ErrorReporter};
///
/// async fn handle_app_error(error: AppError, error_reporter: &ErrorReporter) {
///     match error {
///         AppError::ServiceBus(msg) => {
///             error_reporter.report_error(
///                 "Service Bus Error",
///                 &format!("Failed to connect to Azure Service Bus: {}", msg),
///                 Some("Please check your connection and authentication settings.")
///             ).await;
///         }
///         AppError::Auth(msg) => {
///             error_reporter.report_error(
///                 "Authentication Error",
///                 &format!("Authentication failed: {}", msg),
///                 Some("Please sign in again or check your credentials.")
///             ).await;
///         }
///         AppError::Config(msg) => {
///             error_reporter.report_error(
///                 "Configuration Error",
///                 &format!("Configuration problem: {}", msg),
///                 Some("Please check your configuration file and restart the application.")
///             ).await;
///         }
///         AppError::Component(msg) => {
///             // Component errors are typically less critical for users
///             log::error!("Component error: {}", msg);
///             error_reporter.show_warning(&format!("Display issue: {}", msg)).await;
///         }
///         _ => {
///             error_reporter.report_error(
///                 "Application Error",
///                 &error.to_string(),
///                 Some("Please try again. If the problem persists, restart the application.")
///             ).await;
///         }
///     }
/// }
/// ```
///
/// ## Error Context and Recovery
/// ```no_run
/// use ui::error::{AppError, AppResult};
///
/// async fn load_queue_with_recovery(queue_name: &str) -> AppResult<Vec<Message>> {
///     match load_queue_messages(queue_name).await {
///         Ok(messages) => Ok(messages),
///         Err(AppError::ServiceBus(msg)) if msg.contains("QueueNotFound") => {
///             // Specific recovery for queue not found
///             show_queue_selection_dialog().await;
///             Err(AppError::ServiceBus(format!("Queue '{}' not found. Please select a different queue.", queue_name)))
///         }
///         Err(AppError::Auth(_)) => {
///             // Authentication error - trigger re-auth
///             trigger_authentication_flow().await;
///             Err(AppError::Auth("Please authenticate to continue.".to_string()))
///         }
///         Err(other) => Err(other),
///     }
/// }
/// ```
///
/// ## Error Propagation and Conversion
/// ```no_run
/// use ui::error::{AppError, AppResult};
/// use server::service_bus_manager::ServiceBusError;
///
/// // Automatic conversion from server errors
/// async fn send_message(content: &str) -> AppResult<String> {
///     // This automatically converts ServiceBusError to AppError
///     let message_id = server::send_message(content).await?;
///     Ok(message_id)
/// }
///
/// // Manual error conversion with context
/// async fn load_configuration() -> AppResult<Config> {
///     match std::fs::read_to_string("config.toml") {
///         Ok(content) => {
///             match toml::from_str(&content) {
///                 Ok(config) => Ok(config),
///                 Err(e) => Err(AppError::Config(format!("Invalid configuration format: {}", e))),
///             }
///         }
///         Err(e) => Err(AppError::Config(format!("Failed to read configuration file: {}", e))),
///     }
/// }
/// ```
///
/// ## Integration with UI Components
/// ```no_run
/// use ui::error::{AppError, ErrorContext, ErrorSeverity};
/// use ui::components::common::Msg;
///
/// fn create_error_message(error: AppError) -> Msg {
///     let context = match error {
///         AppError::ServiceBus(ref msg) => ErrorContext {
///             component: "ServiceBus".to_string(),
///             operation: "Connection".to_string(),
///             user_message: "Failed to connect to Azure Service Bus".to_string(),
///             technical_details: Some(msg.clone()),
///             suggestion: Some("Check your network connection and authentication settings".to_string()),
///             severity: ErrorSeverity::Error,
///         },
///         AppError::Auth(ref msg) => ErrorContext {
///             component: "Authentication".to_string(),
///             operation: "Login".to_string(),
///             user_message: "Authentication failed".to_string(),
///             technical_details: Some(msg.clone()),
///             suggestion: Some("Please sign in again".to_string()),
///             severity: ErrorSeverity::Error,
///         },
///         AppError::Config(ref msg) => ErrorContext {
///             component: "Configuration".to_string(),
///             operation: "Load".to_string(),
///             user_message: "Configuration error".to_string(),
///             technical_details: Some(msg.clone()),
///             suggestion: Some("Please check your configuration file".to_string()),
///             severity: ErrorSeverity::Error,
///         },
///         _ => ErrorContext {
///             component: "Application".to_string(),
///             operation: "General".to_string(),
///             user_message: "An error occurred".to_string(),
///             technical_details: Some(error.to_string()),
///             suggestion: Some("Please try again".to_string()),
///             severity: ErrorSeverity::Warning,
///         },
///     };
///
///     Msg::ErrorOccurred(context)
/// }
/// ```
///
/// ## Logging Integration
/// ```no_run
/// use ui::error::{AppError, ErrorSeverity};
///
/// fn log_app_error(error: &AppError, severity: ErrorSeverity) {
///     match severity {
///         ErrorSeverity::Critical => {
///             log::error!("CRITICAL: {}", error);
///             // Additional alerting logic
///         }
///         ErrorSeverity::Error => {
///             log::error!("{}", error);
///         }
///         ErrorSeverity::Warning => {
///             log::warn!("{}", error);
///         }
///     }
/// }
/// ```
///
/// # Error Recovery Strategies
///
/// ## Service Bus Errors
/// - **Connection failures**: Retry with exponential backoff
/// - **Authentication errors**: Trigger re-authentication flow
/// - **Queue not found**: Show queue selection interface
/// - **Message send failures**: Offer retry with user confirmation
///
/// ## Configuration Errors
/// - **File not found**: Create default configuration
/// - **Invalid format**: Show configuration editor
/// - **Permission errors**: Display file permission guidance
///
/// ## Component Errors
/// - **Rendering failures**: Refresh component state
/// - **State corruption**: Reset to default state
/// - **Event handling errors**: Log and continue operation
///
/// # User Experience Guidelines
///
/// - **Service Bus errors**: Provide clear network/auth troubleshooting steps
/// - **Configuration errors**: Offer to open configuration in editor
/// - **Authentication errors**: Provide clear re-authentication path
/// - **Component errors**: Minimize user disruption, prefer silent recovery
///
/// [`ServiceBus`]: AppError::ServiceBus
/// [`Auth`]: AppError::Auth
/// [`Component`]: AppError::Component
/// [`State`]: AppError::State
/// [`Channel`]: AppError::Channel
/// [`Config`]: AppError::Config
#[derive(Debug, Clone, PartialEq)]
pub enum AppError {
    /// Azure Service Bus operation failures.
    ///
    /// This error represents failures in Service Bus operations including
    /// connection issues, authentication problems, message send/receive
    /// failures, and Azure API errors. This is typically the most common
    /// error type in the application.
    ///
    /// # Recovery
    /// - Check network connectivity and authentication
    /// - Retry with exponential backoff for transient failures
    /// - Show user-friendly error messages with troubleshooting steps
    ServiceBus(String),

    /// UI component lifecycle and rendering errors.
    ///
    /// This error occurs when UI components fail to render, update,
    /// or handle events properly. These errors should generally not
    /// disrupt the overall application flow but should be logged
    /// for debugging.
    ///
    /// # Recovery
    /// - Log error details for debugging
    /// - Attempt component state reset
    /// - Continue application operation when possible
    Component(String),

    /// Application state management issues.
    ///
    /// This error represents problems with application state consistency,
    /// state transitions, or state synchronization between components.
    /// These can be critical as they may affect application reliability.
    ///
    /// # Recovery
    /// - Reset to known good state when possible
    /// - Restart affected subsystems
    /// - Preserve user data when feasible
    State(String),

    /// Configuration loading and validation errors.
    ///
    /// This error occurs when configuration files cannot be loaded,
    /// parsed, or contain invalid values. Configuration errors can
    /// prevent application startup or cause runtime issues.
    ///
    /// # Recovery
    /// - Fall back to default configuration
    /// - Show configuration editor to user
    /// - Validate and sanitize configuration values
    Config(String),

    /// Authentication and authorization errors.
    ///
    /// This error represents authentication failures, token expiration,
    /// authorization issues, or credential problems. These errors
    /// typically require user intervention to resolve.
    ///
    /// # Recovery
    /// - Trigger re-authentication flow
    /// - Clear invalid tokens
    /// - Provide clear guidance for credential setup
    Auth(String),

    /// Inter-component communication failures.
    ///
    /// This error occurs when communication between UI components
    /// fails, typically through message passing or event systems.
    /// These can indicate serious application state issues.
    ///
    /// # Recovery
    /// - Reset communication channels
    /// - Restart affected components
    /// - Log detailed information for debugging
    Channel(String),
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::ServiceBus(msg) => write!(f, "Service Bus Error: {msg}"),
            AppError::Component(msg) => write!(f, "Component Error: {msg}"),
            AppError::State(msg) => write!(f, "State Error: {msg}"),
            AppError::Config(msg) => write!(f, "Configuration Error: {msg}"),
            AppError::Auth(msg) => write!(f, "Authentication Error: {msg}"),
            AppError::Channel(msg) => write!(f, "Channel Error: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<server::service_bus_manager::ServiceBusError> for AppError {
    fn from(err: server::service_bus_manager::ServiceBusError) -> Self {
        AppError::ServiceBus(err.to_string())
    }
}

/// Result type alias for application operations
pub type AppResult<T> = Result<T, AppError>;

/// Error severity levels for appropriate UI response
#[derive(Debug, Clone)]
pub enum ErrorSeverity {
    /// Warning severity - show warning popup and log
    Warning,
    /// High severity - show error popup and log
    Error,
    /// Critical severity - show error popup, log, and potentially exit
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
        format!("An error occurred in {component}. Please try again.")
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
        let context =
            ErrorContext::new(component, operation).with_technical_details(&error.to_string());
        self.report(error, context);
    }

    /// Report a warning (shows warning popup)
    pub fn report_warning(&self, error: AppError, component: &str, operation: &str) {
        let context = ErrorContext::new(component, operation).with_severity(ErrorSeverity::Warning);
        self.report(error, context);
    }

    /// Report a critical error that will cause application exit
    /// This method should be used when the error is severe enough to terminate the application
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

    /// Report error with auto-generated context
    pub fn report_error(&self, error: AppError) {
        self.report_simple(error, "Application", "operation");
    }

    /// Report error with full context
    pub fn report(&self, error: AppError, context: ErrorContext) {
        let contextual_error = ContextualError::new(error.clone(), context.clone());

        // Enhanced logging with full context
        match context.severity {
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
            ErrorSeverity::Warning => {
                let popup_msg = Msg::PopupActivity(PopupActivityMsg::ShowWarning(
                    self.format_user_message(&context),
                ));
                if let Err(e) = self.tx.send(popup_msg) {
                    log::error!("Failed to send warning popup message: {e}");
                }
            }
            ErrorSeverity::Error | ErrorSeverity::Critical => {
                let formatted_error = self.create_formatted_error(&error, &context);
                let popup_msg = Msg::PopupActivity(PopupActivityMsg::ShowError(formatted_error));
                if let Err(e) = self.tx.send(popup_msg) {
                    log::error!("Failed to send error popup message: {e}");
                }
            }
        }
    }

    /// Format additional context information for logging
    fn format_additional_context(&self, context: &ErrorContext) -> String {
        let mut parts = Vec::new();

        if let Some(ref technical_details) = context.technical_details {
            parts.push(format!("🔍 Technical: {technical_details}"));
        }

        if let Some(ref suggestion) = context.suggestion {
            parts.push(format!("💡 Suggestion: {suggestion}"));
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
            message.push_str(&format!("\n\n💡 Suggestion: {suggestion}"));
        }

        message
    }

    /// Create a warning error with proper formatting
    pub fn create_warning_error(&self, message: String) -> AppError {
        // Create formatted warning with warning emoji
        let mut formatted_message = String::new();
        formatted_message.push_str("⚠️ Warning");
        formatted_message.push_str(&format!("\n\n{message}"));

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
            AppError::Auth(_) => "🔐",
            AppError::Channel(_) => "📡",
        };

        // Add error title with emoji
        formatted_message.push_str(&format!("{} {}", emoji, self.get_error_title(error)));

        // Add main user message with proper formatting
        formatted_message.push_str(&format!("\n\n{}", context.user_message));

        // Add technical details if available (for debugging)
        if let Some(ref technical) = context.technical_details {
            formatted_message.push_str(&format!("\n\n🔍 Details: {technical}"));
        }

        // Add suggestion if available
        if let Some(ref suggestion) = context.suggestion {
            formatted_message.push_str(&format!("\n\n💡 Suggestion: {suggestion}"));
        }

        // Create a new AppError with the formatted message
        match error {
            AppError::Config(_) => AppError::Config(formatted_message),
            AppError::ServiceBus(_) => AppError::ServiceBus(formatted_message),
            AppError::Component(_) => AppError::Component(formatted_message),
            AppError::State(_) => AppError::State(formatted_message),
            AppError::Auth(_) => AppError::Auth(formatted_message),
            AppError::Channel(_) => AppError::Channel(formatted_message),
        }
    }

    /// Get appropriate title for error type
    fn get_error_title(&self, error: &AppError) -> String {
        match error {
            AppError::Config(_) => "Configuration Error".to_string(),
            AppError::ServiceBus(_) => "Service Bus Error".to_string(),
            AppError::Component(_) => "Component Error".to_string(),
            AppError::State(_) => "Application State Error".to_string(),
            AppError::Auth(_) => "Authentication Error".to_string(),
            AppError::Channel(_) => "Communication Error".to_string(),
        }
    }

    // ========== Helper Methods for Common Error Patterns ==========

    /// Report component mounting/unmounting errors
    pub fn report_mount_error(
        &self,
        component: &str,
        operation: &str,
        error: impl std::fmt::Display,
    ) {
        let app_error = AppError::Component(format!("Failed to {operation} {component}: {error}"));
        self.report_simple(app_error, component, operation);
    }

    /// Report message sending errors (mpsc channel errors)
    pub fn report_send_error(&self, context: &str, error: impl std::fmt::Display) {
        let app_error = AppError::Component(format!("Failed to send {context}: {error}"));
        self.report_simple(app_error, "MessageChannel", "send_message");
    }

    /// Report activation/focus errors for UI components
    pub fn report_activation_error(&self, component: &str, error: impl std::fmt::Display) {
        let app_error = AppError::Component(format!("Failed to activate {component}: {error}"));
        self.report_simple(app_error, component, "activate");
    }

    /// Report global key watcher update errors
    pub fn report_key_watcher_error(&self, error: impl std::fmt::Display) {
        let app_error =
            AppError::Component(format!("Failed to update global key watcher: {error}"));
        self.report_simple(app_error, "GlobalKeyWatcher", "update_state");
    }

    /// Report clipboard operation errors (non-critical, use warning)
    pub fn report_clipboard_error(&self, operation: &str, error: impl std::fmt::Display) {
        let app_error = AppError::Component(format!("Failed to {operation}: {error}"));
        self.report_warning(app_error, "Clipboard", operation);
    }

    /// Report theme-related errors (non-critical, use warning)
    pub fn report_theme_error(&self, operation: &str, error: impl std::fmt::Display) {
        let app_error = AppError::Component(format!("Theme {operation} failed: {error}"));
        self.report_warning(app_error, "ThemeManager", operation);
    }

    /// Report loading/pagination errors with suggestions
    pub fn report_loading_error(
        &self,
        component: &str,
        operation: &str,
        error: impl std::fmt::Display,
    ) {
        let context = ErrorContext::new(component, operation)
            .with_message(&format!("Failed to {operation} data"))
            .with_technical_details(&error.to_string())
            .with_suggestion("Check your connection and try again");

        let app_error = AppError::ServiceBus(error.to_string());
        self.report(app_error, context);
    }

    /// Report service bus connection/operation errors with helpful context
    pub fn report_service_bus_error(
        &self,
        operation: &str,
        error: impl std::fmt::Display,
        suggestion: Option<&str>,
    ) {
        let context = ErrorContext::new("ServiceBus", operation)
            .with_message(&format!("Service bus {operation} failed"))
            .with_technical_details(&error.to_string())
            .with_suggestion(suggestion.unwrap_or("Check your Azure connection and credentials"));

        let app_error = AppError::ServiceBus(error.to_string());
        self.report(app_error, context);
    }

    /// Report bulk operation errors with operation counts
    pub fn report_bulk_operation_error(
        &self,
        operation: &str,
        count: usize,
        error: impl std::fmt::Display,
    ) {
        let context = ErrorContext::new("BulkOperationHandler", operation)
            .with_message(&format!("Failed to {operation} {count} messages"))
            .with_technical_details(&error.to_string())
            .with_suggestion(
                "Some messages may have been processed. Check the queue and try again if needed",
            );

        let app_error = AppError::ServiceBus(error.to_string());
        self.report(app_error, context);
    }

    /// Report configuration errors with suggestions
    pub fn report_config_error(&self, config_type: &str, error: impl std::fmt::Display) {
        let context = ErrorContext::new("Configuration", "load_config")
            .with_message(&format!("Failed to load {config_type} configuration"))
            .with_technical_details(&error.to_string())
            .with_suggestion("Check your configuration file and restart the application");

        let app_error = AppError::Config(error.to_string());
        self.report(app_error, context);
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
        let msg = rx.recv().expect("Should receive warning message");
        assert!(matches!(
            msg,
            Msg::PopupActivity(PopupActivityMsg::ShowWarning(_))
        ));
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
        let display_str = format!("{contextual}");

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
