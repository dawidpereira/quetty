use server::consumer::Consumer;

/// Finds the target message by receiving messages and searching by ID
pub async fn find_target_message(
    consumer: &mut Consumer,
    message_id: &str,
    message_sequence: i64,
) -> Result<azservicebus::ServiceBusReceivedMessage, crate::error::AppError> {
    log::debug!(
        "Looking for message with ID {} and sequence {}",
        message_id,
        message_sequence
    );

    let mut attempts = 0;
    let dlq_config = crate::config::CONFIG.dlq();
    let max_attempts = dlq_config.max_attempts();
    let receive_timeout_secs = dlq_config
        .receive_timeout_secs()
        .min(dlq_config.receive_timeout_cap_secs());
    let mut target_message = None;
    let mut other_messages = Vec::new();

    while attempts < max_attempts && target_message.is_none() {
        log::debug!("Attempt {} to find target message", attempts + 1);

        // Add timeout to prevent hanging indefinitely
        let received_messages = match tokio::time::timeout(
            std::time::Duration::from_secs(receive_timeout_secs),
            consumer.receive_messages(dlq_config.batch_size()),
        )
        .await
        {
            Ok(Ok(messages)) => messages,
            Ok(Err(e)) => {
                log::error!("Failed to receive messages: {}", e);
                return Err(crate::error::AppError::ServiceBus(e.to_string()));
            }
            Err(_) => {
                log::error!(
                    "Timeout while receiving messages after {} seconds",
                    receive_timeout_secs
                );
                return Err(crate::error::AppError::ServiceBus(format!(
                    "Timeout while receiving messages after {} seconds",
                    receive_timeout_secs
                )));
            }
        };

        if received_messages.is_empty() {
            log::warn!(
                "No more messages available to receive on attempt {}",
                attempts + 1
            );
            // If no messages are available, wait a bit before retrying
            tokio::time::sleep(std::time::Duration::from_millis(
                dlq_config.retry_delay_ms(),
            ))
            .await;
            attempts += 1;
            continue;
        }

        log::debug!(
            "Received {} messages on attempt {}",
            received_messages.len(),
            attempts + 1
        );

        for msg in received_messages {
            if let Some(msg_id) = msg.message_id() {
                log::debug!(
                    "Checking message ID: {} (looking for: {}), sequence: {} (looking for: {})",
                    msg_id,
                    message_id,
                    msg.sequence_number(),
                    message_sequence
                );

                if msg_id == message_id && msg.sequence_number() == message_sequence {
                    log::info!(
                        "Found target message with ID {} and sequence {}",
                        message_id,
                        message_sequence
                    );
                    target_message = Some(msg);
                    break;
                }
            } else {
                log::debug!("Message has no ID, sequence: {}", msg.sequence_number());
            }
            other_messages.push(msg);
        }

        attempts += 1;
    }

    // Abandon all the other messages we received but don't want to process
    if !other_messages.is_empty() {
        log::debug!("Abandoning {} other messages", other_messages.len());
        abandon_other_messages(consumer, other_messages).await;
    }

    // Return the target message or error
    target_message.ok_or_else(|| {
        log::error!(
            "Could not find message with ID {} and sequence {} after {} attempts",
            message_id,
            message_sequence,
            attempts
        );
        crate::error::AppError::ServiceBus(format!(
            "Could not find message with ID {} and sequence {} in received messages after {} attempts",
            message_id, message_sequence, attempts
        ))
    })
}

/// Abandons messages that were received but are not the target
pub async fn abandon_other_messages(
    consumer: &mut Consumer,
    other_messages: Vec<azservicebus::ServiceBusReceivedMessage>,
) {
    for msg in other_messages {
        if let Err(e) = consumer.abandon_message(&msg).await {
            let msg_id = msg
                .message_id()
                .map(|id| id.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            log::warn!("Failed to abandon message {}: {}", msg_id, e);
        }
    }
} 