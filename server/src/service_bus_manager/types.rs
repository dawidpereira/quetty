use serde::{Deserialize, Serialize};

/// Type of Service Bus queue for routing and processing messages.
///
/// Distinguishes between main queues (for normal message processing) and
/// dead letter queues (for messages that cannot be processed successfully).
///
/// # Examples
///
/// ```no_run
/// use quetty_server::service_bus_manager::QueueType;
///
/// let queue_type = QueueType::from_queue_name("my-queue");
/// assert_eq!(queue_type, QueueType::Main);
///
/// let dlq_type = QueueType::from_queue_name("my-queue/$deadletterqueue");
/// assert_eq!(dlq_type, QueueType::DeadLetter);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueType {
    /// Main queue for normal message processing
    Main,
    /// Dead letter queue for failed messages
    DeadLetter,
}

impl QueueType {
    /// Determines the queue type from a queue name.
    ///
    /// Analyzes the queue name to determine if it's a dead letter queue
    /// (ends with `/$deadletterqueue`) or a main queue.
    ///
    /// # Arguments
    ///
    /// * `queue_name` - The full queue name to analyze
    ///
    /// # Returns
    ///
    /// [`QueueType::DeadLetter`] if the name ends with `/$deadletterqueue`,
    /// [`QueueType::Main`] otherwise
    pub fn from_queue_name(queue_name: &str) -> Self {
        if queue_name.ends_with("/$deadletterqueue") {
            QueueType::DeadLetter
        } else {
            QueueType::Main
        }
    }
}

/// Information about a Service Bus queue including name and type.
///
/// Represents a queue with its full name and type classification. Provides
/// utility methods for working with main queues and their corresponding
/// dead letter queues.
///
/// # Examples
///
/// ```no_run
/// use quetty_server::service_bus_manager::{QueueInfo, QueueType};
///
/// // Create a main queue
/// let main_queue = QueueInfo::main_queue("orders".to_string());
/// assert_eq!(main_queue.name, "orders");
/// assert_eq!(main_queue.queue_type, QueueType::Main);
///
/// // Get the corresponding dead letter queue
/// let dlq = main_queue.to_dlq();
/// assert_eq!(dlq.name, "orders/$deadletterqueue");
/// assert_eq!(dlq.queue_type, QueueType::DeadLetter);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueInfo {
    /// Full name of the queue
    pub name: String,
    /// Type classification of the queue
    pub queue_type: QueueType,
}

impl QueueInfo {
    /// Creates a new QueueInfo with the specified name and type.
    ///
    /// # Arguments
    ///
    /// * `name` - The full queue name
    /// * `queue_type` - The type of queue
    ///
    /// # Returns
    ///
    /// A new QueueInfo instance
    pub fn new(name: String, queue_type: QueueType) -> Self {
        Self { name, queue_type }
    }

    /// Creates a QueueInfo for a main queue.
    ///
    /// # Arguments
    ///
    /// * `name` - The base name of the queue
    ///
    /// # Returns
    ///
    /// A QueueInfo representing a main queue
    pub fn main_queue(name: String) -> Self {
        Self::new(name, QueueType::Main)
    }

    /// Creates a QueueInfo for a dead letter queue.
    ///
    /// Automatically appends the dead letter queue suffix to the base name.
    ///
    /// # Arguments
    ///
    /// * `base_name` - The base name of the queue (without DLQ suffix)
    ///
    /// # Returns
    ///
    /// A QueueInfo representing the dead letter queue
    pub fn dead_letter_queue(base_name: String) -> Self {
        let dlq_name = format!("{base_name}/$deadletterqueue");
        Self::new(dlq_name, QueueType::DeadLetter)
    }

    /// Gets the base name of the queue without any type-specific suffixes.
    ///
    /// For main queues, returns the name as-is. For dead letter queues,
    /// removes the `/$deadletterqueue` suffix to get the base name.
    ///
    /// # Returns
    ///
    /// The base queue name without type suffixes
    pub fn base_name(&self) -> String {
        match self.queue_type {
            QueueType::Main => self.name.clone(),
            QueueType::DeadLetter => {
                if self.name.ends_with("/$deadletterqueue") {
                    self.name
                        .strip_suffix("/$deadletterqueue")
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| {
                            log::warn!("Failed to strip DLQ suffix from queue name: {}", self.name);
                            self.name.clone()
                        })
                } else {
                    self.name.clone()
                }
            }
        }
    }

    /// Creates a QueueInfo for the dead letter queue corresponding to this queue.
    ///
    /// # Returns
    ///
    /// A QueueInfo representing the dead letter queue for this queue's base name
    pub fn to_dlq(&self) -> Self {
        Self::dead_letter_queue(self.base_name())
    }

    /// Creates a QueueInfo for the main queue corresponding to this queue.
    ///
    /// # Returns
    ///
    /// A QueueInfo representing the main queue for this queue's base name
    pub fn to_main(&self) -> Self {
        Self::main_queue(self.base_name())
    }
}

/// Data structure for message content and metadata.
///
/// Represents a message to be sent to a Service Bus queue, including the
/// message content and optional custom properties. Used for sending new
/// messages and representing message data in transit.
///
/// # Examples
///
/// ```no_run
/// use quetty_server::service_bus_manager::MessageData;
/// use std::collections::HashMap;
///
/// // Simple message with just content
/// let message = MessageData::new("Hello, world!".to_string());
///
/// // Message with custom properties
/// let mut properties = HashMap::new();
/// properties.insert("priority".to_string(), "high".to_string());
/// properties.insert("source".to_string(), "orders-service".to_string());
///
/// let message = MessageData::with_properties(
///     "Order processed: #12345".to_string(),
///     properties
/// );
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageData {
    /// The message content/body
    pub content: String,
    /// Optional custom properties for the message
    pub properties: Option<std::collections::HashMap<String, String>>,
}

impl MessageData {
    /// Creates a new MessageData with the specified content.
    ///
    /// # Arguments
    ///
    /// * `content` - The message content/body
    ///
    /// # Returns
    ///
    /// A new MessageData with no custom properties
    pub fn new(content: String) -> Self {
        Self {
            content,
            properties: None,
        }
    }

    /// Creates a new MessageData with content and custom properties.
    ///
    /// # Arguments
    ///
    /// * `content` - The message content/body
    /// * `properties` - Custom key-value properties for the message
    ///
    /// # Returns
    ///
    /// A new MessageData with the specified content and properties
    pub fn with_properties(
        content: String,
        properties: std::collections::HashMap<String, String>,
    ) -> Self {
        Self {
            content,
            properties: Some(properties),
        }
    }
}

/// Statistics about Service Bus operations including success and failure counts.
///
/// Tracks the performance and outcome of Service Bus operations, providing
/// metrics for monitoring and debugging purposes. Used throughout the system
/// to report operation results.
///
/// # Examples
///
/// ```no_run
/// use quetty_server::service_bus_manager::OperationStats;
///
/// let mut stats = OperationStats::new();
/// stats.add_success();
/// stats.add_success();
/// stats.add_failure();
///
/// assert_eq!(stats.successful, 2);
/// assert_eq!(stats.failed, 1);
/// assert_eq!(stats.total, 3);
/// assert_eq!(stats.success_rate(), 2.0 / 3.0);
/// ```
#[derive(Debug, Clone, Default)]
pub struct OperationStats {
    /// Number of successful operations
    pub successful: usize,
    /// Number of failed operations
    pub failed: usize,
    /// Total number of operations attempted
    pub total: usize,
}

impl OperationStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_success(&mut self) {
        self.successful += 1;
        self.total += 1;
    }

    pub fn add_failure(&mut self) {
        self.failed += 1;
        self.total += 1;
    }

    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.successful as f64 / self.total as f64
        }
    }
}
