use azservicebus::prelude::ServiceBusPeekedMessage;
use azservicebus::primitives::service_bus_message_state::ServiceBusMessageState;
use azure_core::date::OffsetDateTime;
use serde::Serialize;
use serde::ser::Serializer;
use serde_json::Value;
use std::convert::TryFrom;

/// Represents a Service Bus message with all its metadata and content.
///
/// This struct contains all the information about a message in an Azure Service Bus queue,
/// including its sequence number, ID, timestamps, delivery information, state, and body content.
///
/// # Examples
///
/// ```no_run
/// use server::model::{MessageModel, MessageState, BodyData};
/// use azure_core::date::OffsetDateTime;
///
/// let message = MessageModel::new(
///     12345,
///     "message-id-123".to_string(),
///     OffsetDateTime::now_utc(),
///     1,
///     MessageState::Active,
///     BodyData::RawString("Hello, world!".to_string()),
/// );
/// ```
#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct MessageModel {
    /// The sequence number assigned by Service Bus
    pub sequence: i64,
    /// Unique identifier for the message
    pub id: String,
    /// When the message was enqueued in Service Bus
    #[serde(with = "azure_core::date::iso8601")]
    pub enqueued_at: OffsetDateTime,
    /// Number of times the message has been delivered
    pub delivery_count: usize,
    /// Current state of the message
    pub state: MessageState,
    /// The message body content
    pub body: BodyData,
}

/// Represents the possible states of a Service Bus message.
///
/// This enum maps to the Azure Service Bus message states and includes
/// additional states that messages can be in during processing.
#[derive(Serialize, Clone, PartialEq, Debug, Default)]
pub enum MessageState {
    /// Message is active and available for processing
    #[default]
    Active,
    /// Message has been deferred for later processing
    Deferred,
    /// Message is scheduled for future delivery
    Scheduled,
    /// Message has been moved to the dead letter queue
    DeadLettered,
    /// Message has been successfully processed and completed
    Completed,
    /// Message processing was abandoned and returned to the queue
    Abandoned,
}

impl MessageModel {
    /// Creates a new MessageModel with the specified parameters.
    ///
    /// # Arguments
    ///
    /// * `sequence` - The sequence number from Service Bus
    /// * `id` - Unique identifier for the message
    /// * `enqueued_at` - When the message was enqueued
    /// * `delivery_count` - Number of delivery attempts
    /// * `state` - Current state of the message
    /// * `body` - The message body content
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

    /// Converts a collection of Azure Service Bus messages to MessageModel instances.
    ///
    /// This method filters out any messages that fail to convert, returning only
    /// the successfully converted messages. Failed conversions are silently ignored.
    ///
    /// # Arguments
    ///
    /// * `messages` - Vector of ServiceBusPeekedMessage instances to convert
    ///
    /// # Returns
    ///
    /// Vector of successfully converted MessageModel instances
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

    /// Parses the message body from a ServiceBusPeekedMessage.
    ///
    /// Attempts to parse the body as JSON first. If that fails, treats it as raw string data.
    ///
    /// # Arguments
    ///
    /// * `msg` - The Service Bus message to parse
    ///
    /// # Returns
    ///
    /// `Ok(BodyData)` containing either valid JSON or raw string data,
    /// or `Err(MessageModelError)` if the body cannot be retrieved
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

/// Represents the body content of a Service Bus message.
///
/// Message bodies can be either valid JSON that can be parsed and displayed
/// in a structured format, or raw string content that should be displayed as-is.
#[derive(Debug, Clone, PartialEq)]
pub enum BodyData {
    /// Message body contains valid JSON data
    ValidJson(Value),
    /// Message body contains raw string data (including invalid JSON)
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

/// Errors that can occur when converting Azure Service Bus messages to MessageModel.
#[derive(Debug)]
pub enum MessageModelError {
    /// The message is missing a required message ID
    MissingMessageId,
    /// The message is missing a body
    MissingMessageBody,
    /// The message is missing delivery count information
    MissingDeliveryCount,
    /// JSON parsing error when processing message body
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
