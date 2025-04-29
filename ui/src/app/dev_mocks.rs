use chrono::{NaiveDateTime, TimeZone, Utc};
use uuid::Uuid;

use crate::models::models::{MessageDetailsModel, MessageModel};
pub fn mock_messages() -> Result<Vec<MessageModel>, Box<dyn std::error::Error>> {
    let raw_data = vec![
        (
            "1",
            "9d11fd83-b6d8-4c27-9cc1-ebb31d33bb97",
            "2025-04-24 14:00:00",
            "0",
        ),
        (
            "2",
            "b5ba303c-b125-4191-923d-ef2b3b698a7c",
            "2025-04-24 14:05:00",
            "0",
        ),
        (
            "3",
            "8ac51c0f-2d4e-492d-bc1f-b0e550273cc0",
            "2025-04-24 14:10:00",
            "0",
        ),
    ];

    let mut messages = Vec::new();

    for (seq_str, id_str, dt_str, count_str) in raw_data {
        let sequence = seq_str.parse::<i64>()?;
        let id = Uuid::parse_str(id_str)?;
        let naive_dt = NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%d %H:%M:%S")?;
        let enqueued_at = Utc.from_utc_datetime(&naive_dt);
        let delivery_count = count_str.parse::<usize>()?;

        messages.push(MessageModel::new(sequence, id, enqueued_at, delivery_count));
    }

    Ok(messages)
}

pub fn mock_message_details() -> Vec<MessageDetailsModel> {
    vec![
        MessageDetailsModel::new(
            Uuid::parse_str("9d11fd83-b6d8-4c27-9cc1-ebb31d33bb97").unwrap(),
            String::from(
                r#"{
  "event": "OrderPlaced",
  "timestamp": "2025-04-26T14:30:00+02:00",
  "user": {
    "id": "user_12345",
    "name": "John Doe",
    "email": "john.doe@example.com"
  },
  "order": {
    "order_id": "order_98765",
    "items": [
      {
        "product_id": "prod_001",
        "name": "Wireless Mouse",
        "quantity": 2,
        "price": 79.99
      },
      {
        "product_id": "prod_002",
        "name": "Mechanical Keyboard",
        "quantity": 1,
        "price": 349.50
      }
    ],
    "total": 509.48,
    "currency": "PLN",
    "status": "placed"
  }
}"#,
            ),
        ),
        MessageDetailsModel::new(
            Uuid::parse_str("b5ba303c-b125-4191-923d-ef2b3b698a7c").unwrap(),
            String::from(
                r#"{
  "event": "OrderPlaced",
  "timestamp": "2025-04-27T16:45:00+02:00",
  "user": {
    "id": "user_54321",
    "name": "Bob Johnson",
    "email": "bob.johnson@example.com"
  },
  "order": {
    "order_id": "order_67890",
    "items": [
      {
        "product_id": "prod_020",
        "name": "Smartwatch",
        "quantity": 1,
        "price": 899,00
      },
      {
        "product_id": "prod_021",
        "name": "Fitness Tracker Band",
        "quantity": 2,
        "price": 199,99
      }
    ],
    "total": 1 298,98,
    "currency": "PLN",
    "status": "placed"
  }
} "#,
            ),
        ),
        MessageDetailsModel::new(
            Uuid::parse_str("8ac51c0f-2d4e-492d-bc1f-b0e550273cc0").unwrap(),
            String::from(
                r#"{
  "event": "OrderPlaced",
  "timestamp": "2025-04-27T10:15:00+02:00",
  "user": {
    "id": "user_67890",
    "name": "Alice Smith",
    "email": "alice.smith@example.com"
  },
  "order": {
    "order_id": "order_12345",
    "items": [
      {
        "product_id": "prod_010",
        "name": "Bluetooth Headphones",
        "quantity": 1,
        "price": 299,99
      },
      {
        "product_id": "prod_011",
        "name": "USB-C Charger",
        "quantity": 3,
        "price": 49,90
      }
    ],
    "total": 449,69,
    "currency": "PLN",
    "status": "placed"
  }
} "#,
            ),
        ),
    ]
}
