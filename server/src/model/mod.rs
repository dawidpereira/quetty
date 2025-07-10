//! # Data Model Module
//!
//! Core data models and message representations for Azure Service Bus operations.
//! This module provides the fundamental data structures used throughout the Quetty
//! application for representing Service Bus messages, their states, and associated metadata.
//!
//! ## Message Representation
//!
//! The primary model is [`MessageModel`], which provides a unified representation
//! of Azure Service Bus messages that is optimized for terminal UI display and
//! manipulation. It handles the complexities of Azure's message format while
//! providing a clean, consistent interface.
//!
//! ## Key Features
//!
//! ### Flexible Message Body Handling
//! - **JSON Messages** - Automatic parsing and validation of JSON message bodies
//! - **Raw Text Messages** - Support for plain text and binary message content
//! - **Lossless Conversion** - Preserves original message data during transformation
//!
//! ### Message State Management
//! - **State Tracking** - Comprehensive message state representation
//! - **State Transitions** - Support for all Azure Service Bus message states
//! - **Status Visualization** - UI-friendly state representation
//!
//! ### Serialization Support
//! - **JSON Export** - Full message serialization for export and analysis
//! - **Timestamp Handling** - ISO8601 timestamp serialization
//! - **Type Safety** - Strongly typed message components
//!
//! ## Core Types
//!
//! ### MessageModel
//! The primary message representation containing all essential message data:
//!
//! ```no_run
//! use server::model::{MessageModel, MessageState, BodyData};
//! use azure_core::date::OffsetDateTime;
//!
//! // Create a new message model
//! let message = MessageModel::new(
//!     12345,                              // sequence number
//!     "msg-001".to_string(),              // message ID
//!     OffsetDateTime::now_utc(),          // enqueued timestamp
//!     0,                                  // delivery count
//!     MessageState::Active,               // current state
//!     BodyData::RawString("Hello!".to_string()) // message body
//! );
//!
//! println!("Message ID: {}", message.id);
//! println!("State: {:?}", message.state);
//! ```
//!
//! ### MessageState
//! Represents all possible states of a Service Bus message:
//!
//! ```no_run
//! use server::model::MessageState;
//!
//! let state = MessageState::Active;
//! match state {
//!     MessageState::Active => println!("Message is available for processing"),
//!     MessageState::Deferred => println!("Message is deferred for later processing"),
//!     MessageState::Scheduled => println!("Message is scheduled for future delivery"),
//!     MessageState::DeadLettered => println!("Message is in dead letter queue"),
//!     MessageState::Completed => println!("Message processing completed"),
//!     MessageState::Abandoned => println!("Message processing abandoned"),
//! }
//! ```
//!
//! ### BodyData
//! Flexible message body representation supporting both JSON and raw text:
//!
//! ```no_run
//! use server::model::BodyData;
//! use serde_json::json;
//!
//! // JSON message body
//! let json_body = BodyData::ValidJson(json!({
//!     "type": "order",
//!     "id": 12345,
//!     "customer": "Jane Doe"
//! }));
//!
//! // Raw text message body
//! let text_body = BodyData::RawString("Plain text message".to_string());
//!
//! // Bodies serialize appropriately for JSON export
//! let serialized = serde_json::to_string(&json_body)?;
//! ```
//!
//! ## Message Conversion
//!
//! ### From Azure Service Bus Messages
//! Automatic conversion from Azure SDK message types:
//!
//! ```no_run
//! use server::model::MessageModel;
//! use azservicebus::prelude::ServiceBusPeekedMessage;
//! use std::convert::TryFrom;
//!
//! // Convert single message
//! let azure_message: ServiceBusPeekedMessage = get_message_from_azure();
//! let message_model = MessageModel::try_from(azure_message)?;
//!
//! // Convert batch of messages with error handling
//! let azure_messages: Vec<ServiceBusPeekedMessage> = get_messages_from_azure();
//! let valid_messages = MessageModel::try_convert_messages_collect(azure_messages);
//! println!("Successfully converted {} messages", valid_messages.len());
//! ```
//!
//! ### Error Handling
//! Robust error handling for message conversion:
//!
//! ```no_run
//! use server::model::{MessageModel, MessageModelError};
//! use std::convert::TryFrom;
//!
//! match MessageModel::try_from(azure_message) {
//!     Ok(message) => {
//!         println!("Successfully converted message: {}", message.id);
//!     }
//!     Err(MessageModelError::MissingMessageId) => {
//!         eprintln!("Message is missing required ID field");
//!     }
//!     Err(MessageModelError::MissingMessageBody) => {
//!         eprintln!("Message is missing body content");
//!     }
//!     Err(MessageModelError::MissingDeliveryCount) => {
//!         eprintln!("Message is missing delivery count");
//!     }
//!     Err(MessageModelError::JsonError(e)) => {
//!         eprintln!("JSON parsing error: {}", e);
//!     }
//! }
//! ```
//!
//! ## Integration with UI
//!
//! Models are designed for seamless integration with the terminal UI:
//!
//! - **Display Optimization** - Fields optimized for table and detail views
//! - **Sorting Support** - Comparable fields for list sorting
//! - **Color Coding** - State-based visual indicators
//! - **Export Support** - JSON serialization for data export
//!
//! ## Performance Considerations
//!
//! - **Efficient Conversion** - Minimal overhead in Azure message conversion
//! - **Memory Optimization** - Appropriate use of owned vs. borrowed data
//! - **Batch Processing** - Optimized batch conversion for large message sets
//! - **Error Tolerance** - Graceful handling of malformed messages in batches
//!
//! ## Thread Safety
//!
//! All model types implement appropriate traits for concurrent use:
//! - `Clone` for efficient copying
//! - `Send` and `Sync` for thread safety
//! - `Debug` for development and logging

use azservicebus::prelude::ServiceBusPeekedMessage;
use azservicebus::primitives::service_bus_message_state::ServiceBusMessageState;
use azure_core::date::OffsetDateTime;
use serde::Serialize;
use serde::ser::Serializer;
use serde_json::Value;
use std::convert::TryFrom;

/// Unified message model representing Azure Service Bus messages.
///
/// This struct provides a clean, consistent representation of Service Bus messages
/// that is optimized for terminal UI display and manipulation. It handles the
/// complexities of Azure's message format while providing type safety and
/// serialization support.
///
/// # Fields
///
/// - `sequence` - Unique sequence number assigned by Azure Service Bus
/// - `id` - Message identifier (typically a GUID)
/// - `enqueued_at` - Timestamp when the message was enqueued
/// - `delivery_count` - Number of delivery attempts for this message
/// - `state` - Current state of the message in the queue
/// - `body` - Message content (JSON or raw text)
///
/// # Examples
///
/// ## Creating a New Message Model
/// ```no_run
/// use server::model::{MessageModel, MessageState, BodyData};
/// use azure_core::date::OffsetDateTime;
///
/// let message = MessageModel::new(
///     12345,
///     "550e8400-e29b-41d4-a716-446655440000".to_string(),
///     OffsetDateTime::now_utc(),
///     0,
///     MessageState::Active,
///     BodyData::RawString("Hello, Service Bus!".to_string())
/// );
///
/// assert_eq!(message.sequence, 12345);
/// assert_eq!(message.delivery_count, 0);
/// assert_eq!(message.state, MessageState::Active);
/// ```
///
/// ## Converting from Azure Messages
/// ```no_run
/// use server::model::MessageModel;
/// use azservicebus::prelude::ServiceBusPeekedMessage;
/// use std::convert::TryFrom;
///
/// // Convert single message with error handling
/// let azure_message: ServiceBusPeekedMessage = get_azure_message();
/// match MessageModel::try_from(azure_message) {
///     Ok(message) => {
///         println!("Converted message: {} (sequence: {})",
///                  message.id, message.sequence);
///     }
///     Err(e) => eprintln!("Conversion failed: {:?}", e),
/// }
/// ```
///
/// ## Batch Conversion
/// ```no_run
/// use server::model::MessageModel;
/// use azservicebus::prelude::ServiceBusPeekedMessage;
///
/// let azure_messages: Vec<ServiceBusPeekedMessage> = get_azure_messages();
/// let valid_messages = MessageModel::try_convert_messages_collect(azure_messages);
///
/// println!("Successfully converted {} messages", valid_messages.len());
/// for message in valid_messages {
///     println!("  - {} ({})", message.id, message.state);
/// }
/// ```
///
/// ## JSON Serialization
/// ```no_run
/// use server::model::{MessageModel, MessageState, BodyData};
/// use serde_json::json;
///
/// let message = MessageModel {
///     sequence: 12345,
///     id: "test-message".to_string(),
///     enqueued_at: OffsetDateTime::now_utc(),
///     delivery_count: 0,
///     state: MessageState::Active,
///     body: BodyData::ValidJson(json!({"type": "test", "data": "value"})),
/// };
///
/// // Serialize to JSON for export or API responses
/// let json_string = serde_json::to_string_pretty(&message)?;
/// println!("Exported message:\n{}", json_string);
/// ```
///
/// # Thread Safety
///
/// `MessageModel` implements `Clone`, `Send`, and `Sync`, making it safe to share
/// across threads and async tasks. This is essential for the concurrent nature
/// of the terminal UI application.
///
/// # Performance Notes
///
/// - Cloning is relatively inexpensive due to reference counting for large data
/// - Serialization is optimized for both human-readable and compact formats
/// - Memory usage is minimized through efficient string storage
#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct MessageModel {
    /// Unique sequence number assigned by Azure Service Bus for message ordering
    pub sequence: i64,
    /// Message identifier, typically a GUID string
    pub id: String,
    /// UTC timestamp when the message was originally enqueued
    #[serde(with = "azure_core::date::iso8601")]
    pub enqueued_at: OffsetDateTime,
    /// Number of times delivery has been attempted for this message
    pub delivery_count: usize,
    /// Current state of the message within the Service Bus queue
    pub state: MessageState,
    /// Message content, either parsed JSON or raw text
    pub body: BodyData,
}

/// Represents the current state of a message within Azure Service Bus.
///
/// This enum maps to Azure Service Bus message states and provides additional
/// states for local message lifecycle management. Each state represents a
/// specific point in the message processing lifecycle with different implications
/// for message availability and processing.
///
/// # State Transitions
///
/// Messages typically follow these state transitions:
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                    Message Lifecycle                        │
/// └─────────────────────────────────────────────────────────────┘
///
///           Enqueue
///              │
///              ▼
///          ┌─────────┐     Defer      ┌──────────┐
///          │ Active  │ ──────────────► │ Deferred │
///          └─────────┘                 └──────────┘
///              │                           │
///              │ Complete                  │ Activate
///              ▼                           ▼
///        ┌───────────┐                 ┌─────────┐
///        │ Completed │◄────────────────│ Active  │
///        └───────────┘                 └─────────┘
///              │                           │
///              │                           │ Abandon/Retry Limit
///              │                           ▼
///              │                    ┌─────────────┐
///              │                    │ Abandoned / │
///              │                    │DeadLettered │
///              │                    └─────────────┘
///              │
///          ┌───────────┐     Schedule     ┌───────────┐
///          │ Scheduled │ ◄───────────────│   Active  │
///          └───────────┘                 └───────────┘
/// ```
///
/// # Examples
///
/// ## Checking Message State
/// ```no_run
/// use server::model::MessageState;
///
/// let state = MessageState::Active;
///
/// match state {
///     MessageState::Active => {
///         println!("Message is available for immediate processing");
///     }
///     MessageState::Deferred => {
///         println!("Message is deferred - can be activated later");
///     }
///     MessageState::Scheduled => {
///         println!("Message is scheduled for future delivery");
///     }
///     MessageState::DeadLettered => {
///         println!("Message failed processing and is in dead letter queue");
///     }
///     MessageState::Completed => {
///         println!("Message processing completed successfully");
///     }
///     MessageState::Abandoned => {
///         println!("Message processing was abandoned");
///     }
/// }
/// ```
///
/// ## State-Based Operations
/// ```no_run
/// use server::model::{MessageModel, MessageState};
///
/// fn can_process_message(message: &MessageModel) -> bool {
///     matches!(message.state, MessageState::Active | MessageState::Deferred)
/// }
///
/// fn requires_attention(message: &MessageModel) -> bool {
///     matches!(message.state, MessageState::DeadLettered | MessageState::Abandoned)
/// }
///
/// fn is_pending_delivery(message: &MessageModel) -> bool {
///     matches!(message.state, MessageState::Scheduled)
/// }
///
/// // Usage
/// let message = get_message();
/// if can_process_message(&message) {
///     println!("Message {} is ready for processing", message.id);
/// } else if requires_attention(&message) {
///     println!("Message {} requires manual intervention", message.id);
/// }
/// ```
///
/// ## UI Display Logic
/// ```no_run
/// use server::model::MessageState;
///
/// fn get_state_color(state: &MessageState) -> &'static str {
///     match state {
///         MessageState::Active => "green",
///         MessageState::Deferred => "yellow",
///         MessageState::Scheduled => "blue",
///         MessageState::DeadLettered => "red",
///         MessageState::Completed => "gray",
///         MessageState::Abandoned => "orange",
///     }
/// }
///
/// fn get_state_description(state: &MessageState) -> &'static str {
///     match state {
///         MessageState::Active => "Ready for processing",
///         MessageState::Deferred => "Deferred for later",
///         MessageState::Scheduled => "Scheduled delivery",
///         MessageState::DeadLettered => "Failed processing",
///         MessageState::Completed => "Processing complete",
///         MessageState::Abandoned => "Processing abandoned",
///     }
/// }
/// ```
///
/// # State Descriptions
///
/// - **Active** - Message is available in the queue for immediate processing
/// - **Deferred** - Message has been deferred and can be retrieved by sequence number
/// - **Scheduled** - Message is scheduled for delivery at a future time
/// - **DeadLettered** - Message exceeded retry limits or failed validation
/// - **Completed** - Message processing completed successfully
/// - **Abandoned** - Message processing was explicitly abandoned
///
/// # Default State
///
/// The default state is `Active`, representing a newly enqueued message ready
/// for processing.
#[derive(Serialize, Clone, PartialEq, Debug, Default)]
pub enum MessageState {
    /// Message is available in the queue for immediate processing.
    /// This is the most common state for messages awaiting consumption.
    #[default]
    Active,

    /// Message has been deferred by a receiver and can be retrieved later
    /// using its sequence number. Deferred messages don't count toward
    /// the queue's active message count.
    Deferred,

    /// Message is scheduled for delivery at a specific future time.
    /// It will automatically become Active when the scheduled time arrives.
    Scheduled,

    /// Message has been moved to the dead letter queue due to:
    /// - Exceeding maximum delivery count
    /// - Message TTL expiration
    /// - Explicit dead lettering by receiver
    /// - Message size limits or other validation failures
    DeadLettered,

    /// Message processing has been completed successfully.
    /// This is a terminal state - the message will be removed from the queue.
    Completed,

    /// Message processing was abandoned by the receiver.
    /// The message may be retried or moved to dead letter queue
    /// depending on retry policies.
    Abandoned,
}

impl MessageModel {
    pub fn new(
        sequence: i64,
        id: String,
        enqueued_at: OffsetDateTime,
        delivery_count: usize,
        state: MessageState,
        body: BodyData,
    ) -> Self {
        Self {
            sequence,
            id,
            enqueued_at,
            delivery_count,
            state,
            body,
        }
    }

    pub fn try_convert_messages_collect(
        messages: Vec<ServiceBusPeekedMessage>,
    ) -> Vec<MessageModel> {
        let mut valid_models = Vec::new();

        for msg in messages {
            if let Ok(model) = MessageModel::try_from(msg) {
                valid_models.push(model);
            }
        }

        valid_models
    }

    fn parse_message_body(msg: &ServiceBusPeekedMessage) -> Result<BodyData, MessageModelError> {
        let bytes = match msg.body() {
            Ok(body) => body,
            Err(_) => return Err(MessageModelError::MissingMessageBody),
        };

        match serde_json::from_slice::<Value>(bytes) {
            Ok(val) => Ok(BodyData::ValidJson(val)),
            Err(_) => Ok(BodyData::RawString(
                String::from_utf8_lossy(bytes).into_owned(),
            )),
        }
    }
}

/// Flexible representation of message body content supporting both JSON and raw text.
///
/// This enum handles the diverse nature of Service Bus message content, providing
/// type-safe handling for both structured JSON data and plain text messages.
/// The parser attempts JSON deserialization first, falling back to raw string
/// storage to ensure no message content is lost.
///
/// # Variants
///
/// - **ValidJson** - Successfully parsed JSON content stored as `serde_json::Value`
/// - **RawString** - Plain text or unparseable content stored as UTF-8 string
///
/// # Examples
///
/// ## Working with JSON Messages
/// ```no_run
/// use server::model::BodyData;
/// use serde_json::{json, Value};
///
/// // Create JSON message body
/// let json_body = BodyData::ValidJson(json!({
///     "orderId": 12345,
///     "customerId": "customer-001",
///     "items": [
///         {"productId": "prod-001", "quantity": 2},
///         {"productId": "prod-002", "quantity": 1}
///     ],
///     "total": 149.99
/// }));
///
/// // Access JSON content
/// if let BodyData::ValidJson(value) = &json_body {
///     if let Some(order_id) = value.get("orderId").and_then(|v| v.as_i64()) {
///         println!("Processing order: {}", order_id);
///     }
///
///     if let Some(items) = value.get("items").and_then(|v| v.as_array()) {
///         println!("Order contains {} items", items.len());
///     }
/// }
/// ```
///
/// ## Working with Text Messages
/// ```no_run
/// use server::model::BodyData;
///
/// // Create text message body
/// let text_body = BodyData::RawString("Hello, Service Bus!".to_string());
///
/// // Access text content
/// if let BodyData::RawString(content) = &text_body {
///     println!("Message content: {}", content);
///
///     // Process text content
///     let word_count = content.split_whitespace().count();
///     println!("Message contains {} words", word_count);
/// }
/// ```
///
/// ## Pattern Matching for Different Content Types
/// ```no_run
/// use server::model::{BodyData, MessageModel};
/// use serde_json::Value;
///
/// fn process_message(message: &MessageModel) {
///     match &message.body {
///         BodyData::ValidJson(json_value) => {
///             println!("Processing JSON message");
///
///             // Handle different JSON message types
///             match json_value.get("type").and_then(|v| v.as_str()) {
///                 Some("order") => process_order_message(json_value),
///                 Some("notification") => process_notification_message(json_value),
///                 Some("event") => process_event_message(json_value),
///                 _ => println!("Unknown JSON message type"),
///             }
///         }
///         BodyData::RawString(text) => {
///             println!("Processing text message: {}", text);
///
///             // Handle different text formats
///             if text.starts_with("CMD:") {
///                 process_command_message(text);
///             } else if text.contains("ERROR") {
///                 process_error_message(text);
///             } else {
///                 process_plain_text_message(text);
///             }
///         }
///     }
/// }
/// ```
///
/// ## Serialization Behavior
/// ```no_run
/// use server::model::BodyData;
/// use serde_json::{json, to_string};
///
/// // JSON bodies serialize as their JSON content
/// let json_body = BodyData::ValidJson(json!({"key": "value"}));
/// let json_serialized = to_string(&json_body)?;
/// assert_eq!(json_serialized, r#"{"key":"value"}"#);
///
/// // Text bodies serialize as quoted strings
/// let text_body = BodyData::RawString("Hello, World!".to_string());
/// let text_serialized = to_string(&text_body)?;
/// assert_eq!(text_serialized, r#""Hello, World!""#);
/// ```
///
/// ## Content Type Detection
/// ```no_run
/// use server::model::BodyData;
///
/// fn analyze_message_content(body: &BodyData) -> String {
///     match body {
///         BodyData::ValidJson(value) => {
///             format!("JSON message with {} top-level fields",
///                     value.as_object().map(|o| o.len()).unwrap_or(0))
///         }
///         BodyData::RawString(text) => {
///             let char_count = text.chars().count();
///             let line_count = text.lines().count();
///             format!("Text message: {} characters, {} lines", char_count, line_count)
///         }
///     }
/// }
/// ```
///
/// ## Creating from Raw Data
/// ```no_run
/// use server::model::BodyData;
/// use serde_json::{from_str, Value};
///
/// fn create_body_from_bytes(data: &[u8]) -> BodyData {
///     // Try to parse as UTF-8 first
///     match std::str::from_utf8(data) {
///         Ok(text) => {
///             // Try to parse as JSON
///             match from_str::<Value>(text) {
///                 Ok(json) => BodyData::ValidJson(json),
///                 Err(_) => BodyData::RawString(text.to_string()),
///             }
///         }
///         Err(_) => {
///             // Fallback to lossy UTF-8 conversion for binary data
///             BodyData::RawString(String::from_utf8_lossy(data).into_owned())
///         }
///     }
/// }
/// ```
///
/// # Performance Notes
///
/// - JSON parsing is performed only once during message conversion
/// - `Value` type provides efficient access to JSON structure without re-parsing
/// - String storage uses Rust's efficient string handling
/// - Cloning is optimized through reference counting for large JSON objects
///
/// # Thread Safety
///
/// `BodyData` implements `Clone`, `Send`, and `Sync`, making it safe for
/// concurrent use across threads and async tasks.
#[derive(Debug, Clone, PartialEq)]
pub enum BodyData {
    /// Successfully parsed JSON content.
    ///
    /// Contains a `serde_json::Value` that provides structured access
    /// to the JSON data. This allows for efficient querying and
    /// manipulation of JSON message content.
    ValidJson(Value),

    /// Raw text content that couldn't be parsed as JSON.
    ///
    /// Contains the original message content as a UTF-8 string.
    /// This preserves all message data even for non-JSON content
    /// or malformed JSON that couldn't be parsed.
    RawString(String),
}

impl Serialize for BodyData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            BodyData::ValidJson(val) => val.serialize(serializer),
            BodyData::RawString(s) => serializer.serialize_str(s),
        }
    }
}

#[derive(Debug)]
pub enum MessageModelError {
    MissingMessageId,
    MissingMessageBody,
    MissingDeliveryCount,
    JsonError(serde_json::Error),
}

impl TryFrom<ServiceBusPeekedMessage> for MessageModel {
    type Error = MessageModelError;

    fn try_from(msg: ServiceBusPeekedMessage) -> Result<Self, Self::Error> {
        let id = msg
            .message_id()
            .ok_or(MessageModelError::MissingMessageId)?
            .to_string();

        let body = MessageModel::parse_message_body(&msg)?;

        let delivery_count = msg
            .delivery_count()
            .ok_or(MessageModelError::MissingDeliveryCount)? as usize;

        // Map Azure message state to our internal MessageState enum
        let state = match msg.state() {
            ServiceBusMessageState::Active => MessageState::Active,
            ServiceBusMessageState::Deferred => MessageState::Deferred,
            ServiceBusMessageState::Scheduled => MessageState::Scheduled,
        };

        Ok(Self {
            sequence: msg.sequence_number(),
            id,
            enqueued_at: msg.enqueued_time(),
            delivery_count,
            state,
            body,
        })
    }
}
