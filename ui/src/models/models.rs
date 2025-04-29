use chrono::{DateTime, Utc};
use uuid::Uuid;

pub struct MessageDetailsModel {
    pub id: Uuid,
    pub message: String
}

impl MessageDetailsModel {
    pub fn new(id: Uuid, message: String) -> Self{
        Self{ id, message }
    } 
}

pub struct MessageModel{
    pub sequence: i64,
    pub id: Uuid,
    pub enqueued_at: DateTime<Utc>,
    pub delivery_count: usize
}

impl MessageModel {
    pub fn new(sequence: i64, id: Uuid, enqueued_at: DateTime<Utc>, delivery_count: usize) -> Self{
        Self{ sequence, id, enqueued_at, delivery_count }
    }
}
