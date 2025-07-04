use std::fmt;

#[derive(Debug, Clone)]
pub enum ServiceBusError {
    /// Connection related errors
    ConnectionFailed(String),
    ConnectionLost(String),
    AuthenticationFailed(String),

    /// Consumer related errors
    ConsumerCreationFailed(String),
    ConsumerNotFound,
    ConsumerAlreadyExists(String),

    /// Producer related errors
    ProducerCreationFailed(String),
    ProducerNotFound(String),

    /// Message operation errors
    MessageReceiveFailed(String),
    MessageSendFailed(String),
    MessageCompleteFailed(String),
    MessageAbandonFailed(String),
    MessageDeadLetterFailed(String),

    /// Bulk operation errors
    BulkOperationFailed(String),
    BulkOperationPartialFailure {
        successful: usize,
        failed: usize,
        errors: Vec<String>,
    },

    /// Queue operation errors
    QueueNotFound(String),
    QueueSwitchFailed(String),
    InvalidQueueName(String),

    /// Configuration errors
    ConfigurationError(String),
    InvalidConfiguration(String),

    /// Timeout errors
    OperationTimeout(String),

    /// Generic errors
    InternalError(String),
    Unknown(String),
}

impl fmt::Display for ServiceBusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceBusError::ConnectionFailed(msg) => write!(f, "Connection failed: {msg}"),
            ServiceBusError::ConnectionLost(msg) => write!(f, "Connection lost: {msg}"),
            ServiceBusError::AuthenticationFailed(msg) => {
                write!(f, "Authentication failed: {msg}")
            }

            ServiceBusError::ConsumerCreationFailed(msg) => {
                write!(f, "Consumer creation failed: {msg}")
            }
            ServiceBusError::ConsumerNotFound => write!(f, "Consumer not found"),
            ServiceBusError::ConsumerAlreadyExists(queue) => {
                write!(f, "Consumer already exists for queue: {queue}")
            }

            ServiceBusError::ProducerCreationFailed(msg) => {
                write!(f, "Producer creation failed: {msg}")
            }
            ServiceBusError::ProducerNotFound(queue) => {
                write!(f, "Producer not found for queue: {queue}")
            }

            ServiceBusError::MessageReceiveFailed(msg) => {
                write!(f, "Message receive failed: {msg}")
            }
            ServiceBusError::MessageSendFailed(msg) => write!(f, "Message send failed: {msg}"),
            ServiceBusError::MessageCompleteFailed(msg) => {
                write!(f, "Message complete failed: {msg}")
            }
            ServiceBusError::MessageAbandonFailed(msg) => {
                write!(f, "Message abandon failed: {msg}")
            }
            ServiceBusError::MessageDeadLetterFailed(msg) => {
                write!(f, "Message dead letter failed: {msg}")
            }

            ServiceBusError::BulkOperationFailed(msg) => {
                write!(f, "Bulk operation failed: {msg}")
            }
            ServiceBusError::BulkOperationPartialFailure {
                successful,
                failed,
                errors,
            } => {
                write!(
                    f,
                    "Bulk operation partially failed: {} successful, {} failed. Errors: {}",
                    successful,
                    failed,
                    errors.join("; ")
                )
            }

            ServiceBusError::QueueNotFound(queue) => write!(f, "Queue not found: {queue}"),
            ServiceBusError::QueueSwitchFailed(msg) => write!(f, "Queue switch failed: {msg}"),
            ServiceBusError::InvalidQueueName(queue) => write!(f, "Invalid queue name: {queue}"),

            ServiceBusError::ConfigurationError(msg) => write!(f, "Configuration error: {msg}"),
            ServiceBusError::InvalidConfiguration(msg) => {
                write!(f, "Invalid configuration: {msg}")
            }

            ServiceBusError::OperationTimeout(msg) => write!(f, "Operation timeout: {msg}"),

            ServiceBusError::InternalError(msg) => write!(f, "Internal error: {msg}"),
            ServiceBusError::Unknown(msg) => write!(f, "Unknown error: {msg}"),
        }
    }
}

impl std::error::Error for ServiceBusError {}

impl From<azure_core::Error> for ServiceBusError {
    fn from(err: azure_core::Error) -> Self {
        ServiceBusError::InternalError(err.to_string())
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for ServiceBusError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        ServiceBusError::InternalError(err.to_string())
    }
}

impl From<tokio::time::error::Elapsed> for ServiceBusError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        ServiceBusError::OperationTimeout(err.to_string())
    }
}

// Result type alias for convenience
pub type ServiceBusResult<T> = Result<T, ServiceBusError>;
