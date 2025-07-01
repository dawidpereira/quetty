use azservicebus::prelude::ServiceBusPeekedMessage;
use azservicebus::primitives::service_bus_message_state::ServiceBusMessageState;
use azure_core::date::OffsetDateTime;
use serde::Serialize;
use serde::ser::Serializer;
use serde_json::Value;
use std::convert::TryFrom;

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct MessageModel {
    pub sequence: i64,
    pub id: String,
    #[serde(with = "azure_core::date::iso8601")]
    pub enqueued_at: OffsetDateTime,
    pub delivery_count: usize,
    pub state: MessageState,
    pub body: BodyData,
}

#[derive(Serialize, Clone, PartialEq, Debug, Default)]
pub enum MessageState {
    #[default]
    Active,
    Deferred,
    Scheduled,
    DeadLettered,
    Completed,
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

#[derive(Debug, Clone, PartialEq)]
pub enum BodyData {
    ValidJson(Value),
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
