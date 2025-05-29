use crate::app::model::Model;
use crate::components::common::Msg;
use server::consumer::Consumer;
use server::producer::{Producer, ServiceBusClientProducerExt};
use std::sync::Arc;
use std::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn handle_send_message_to_dlq(&mut self, index: usize) -> Option<Msg> {
        // ⚠️ WARNING: DLQ message sending is in development and not recommended for production use

        // Validate the request
        let message = match self.validate_dlq_request(index) {
            Ok(msg) => msg,
            Err(error_msg) => return Some(error_msg),
        };

        // Get required resources
        let consumer = match self.get_consumer_for_dlq() {
            Ok(consumer) => consumer,
            Err(error_msg) => return Some(error_msg),
        };

        // Start the DLQ operation
        self.start_dlq_operation(message, consumer);
        None
    }

    /// Validates that the DLQ request is valid and returns the target message
    fn validate_dlq_request(&self, index: usize) -> Result<server::model::MessageModel, Msg> {
        // Get the message at the specified index
        let message = if let Some(messages) = &self.queue_state.messages {
            if let Some(msg) = messages.get(index) {
                msg.clone()
            } else {
                log::error!("Message index {} out of bounds", index);
                return Err(Msg::Error(crate::error::AppError::State(
                    "Message index out of bounds".to_string(),
                )));
            }
        } else {
            log::error!("No messages available");
            return Err(Msg::Error(crate::error::AppError::State(
                "No messages available".to_string(),
            )));
        };

        // Only allow sending to DLQ from main queue (not from DLQ itself)
        if self.queue_state.current_queue_type != crate::components::common::QueueType::Main {
            log::warn!("Cannot send message to DLQ from dead letter queue");
            return Err(Msg::Error(crate::error::AppError::State(
                "Cannot send message to DLQ from dead letter queue".to_string(),
            )));
        }

        Ok(message)
    }

    /// Gets the consumer for DLQ operations
    fn get_consumer_for_dlq(&self) -> Result<Arc<Mutex<Consumer>>, Msg> {
        match self.queue_state.consumer.clone() {
            Some(consumer) => Ok(consumer),
            None => {
                log::error!("No consumer available");
                Err(Msg::Error(crate::error::AppError::State(
                    "No consumer available".to_string(),
                )))
            }
        }
    }

    /// Starts the DLQ operation in a background task
    fn start_dlq_operation(
        &self,
        message: server::model::MessageModel,
        consumer: Arc<Mutex<Consumer>>,
    ) {
        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        // Show loading indicator
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Start(
                "Sending message to dead letter queue...".to_string(),
            ),
        )) {
            log::error!("Failed to send loading start message: {}", e);
        }

        let tx_to_main_err = tx_to_main.clone();
        let message_id = message.id.clone();
        let message_sequence = message.sequence;

        taskpool.execute(async move {
            let result =
                Self::execute_dlq_operation(consumer, message_id.clone(), message_sequence).await;

            match result {
                Ok(()) => {
                    Self::handle_dlq_success(&tx_to_main, &message_id, message_sequence);
                }
                Err(e) => {
                    Self::handle_dlq_error(&tx_to_main, &tx_to_main_err, e);
                }
            }
        });
    }

    /// Executes the DLQ operation: find and dead letter the target message
    async fn execute_dlq_operation(
        consumer: Arc<Mutex<Consumer>>,
        message_id: String,
        message_sequence: i64,
    ) -> Result<(), crate::error::AppError> {
        let mut consumer = consumer.lock().await;

        // Find the target message using shared utility
        let target_msg = super::utils::find_target_message(&mut consumer, &message_id, message_sequence).await?;

        // Send the message to dead letter queue
        log::info!("Sending message {} to dead letter queue", message_id);
        consumer
            .dead_letter_message(
                &target_msg,
                Some("Manual dead letter".to_string()),
                Some("Message manually sent to DLQ via Ctrl+D".to_string()),
            )
            .await
            .map_err(|e| {
                log::error!("Failed to dead letter message: {}", e);
                crate::error::AppError::ServiceBus(e.to_string())
            })?;

        log::info!(
            "Successfully sent message {} to dead letter queue",
            message_id
        );

        Ok(())
    }

    /// Handles successful DLQ operation
    fn handle_dlq_success(
        tx_to_main: &Sender<crate::components::common::Msg>,
        message_id: &str,
        message_sequence: i64,
    ) {
        log::info!(
            "DLQ operation completed successfully for message {} (sequence {})",
            message_id,
            message_sequence
        );

        // Stop loading indicator
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Stop,
        )) {
            log::error!("Failed to send loading stop message: {}", e);
        }

        // Remove the message from local state instead of reloading from server
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::MessageActivity(
            crate::components::common::MessageActivityMsg::RemoveMessageFromState(
                message_id.to_string(),
                message_sequence,
            ),
        )) {
            log::error!("Failed to send remove message from state message: {}", e);
        }
    }

    /// Handles DLQ operation errors
    fn handle_dlq_error(
        tx_to_main: &Sender<crate::components::common::Msg>,
        tx_to_main_err: &Sender<crate::components::common::Msg>,
        error: crate::error::AppError,
    ) {
        log::error!("Error in DLQ operation: {}", error);

        // Stop loading indicator
        if let Err(err) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Stop,
        )) {
            log::error!("Failed to send loading stop message: {}", err);
        }

        // Send error message
        let _ = tx_to_main_err.send(crate::components::common::Msg::Error(error));
    }

    pub fn handle_resend_message_from_dlq(&mut self, index: usize) -> Option<Msg> {
        // ⚠️ WARNING: DLQ message resending is in development and not recommended for production use

        // Validate the request
        let message = match self.validate_resend_request(index) {
            Ok(msg) => msg,
            Err(error_msg) => return Some(error_msg),
        };

        // Get required resources
        let consumer = match self.get_consumer_for_dlq() {
            Ok(consumer) => consumer,
            Err(error_msg) => return Some(error_msg),
        };

        // Start the resend operation
        self.start_resend_operation(message, consumer)
    }

    /// Validates that the resend request is valid and returns the target message
    fn validate_resend_request(&self, index: usize) -> Result<server::model::MessageModel, Msg> {
        // Get the message at the specified index
        let message = if let Some(messages) = &self.queue_state.messages {
            if let Some(msg) = messages.get(index) {
                msg.clone()
            } else {
                log::error!("Message index {} out of bounds", index);
                return Err(Msg::Error(crate::error::AppError::State(
                    "Message index out of bounds".to_string(),
                )));
            }
        } else {
            log::error!("No messages available");
            return Err(Msg::Error(crate::error::AppError::State(
                "No messages available".to_string(),
            )));
        };

        // Only allow resending from DLQ (not from main queue)
        if self.queue_state.current_queue_type != crate::components::common::QueueType::DeadLetter {
            log::warn!("Cannot resend message from main queue - only from dead letter queue");
            return Err(Msg::Error(crate::error::AppError::State(
                "Cannot resend message from main queue - only from dead letter queue".to_string(),
            )));
        }

        Ok(message)
    }

    /// Starts the resend operation in a background task
    fn start_resend_operation(
        &self,
        message: server::model::MessageModel,
        consumer: Arc<Mutex<Consumer>>,
    ) -> Option<Msg> {
        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        // Show loading indicator
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Start(
                "Resending message from dead letter queue...".to_string(),
            ),
        )) {
            log::error!("Failed to send loading start message: {}", e);
        }

        let tx_to_main_err = tx_to_main.clone();
        let message_id = message.id.clone();
        let message_sequence = message.sequence;

        // Get the main queue name and service bus client for resending
        let main_queue_name = match self.get_main_queue_name_from_current_dlq() {
            Ok(name) => name,
            Err(e) => {
                log::error!("Failed to get main queue name: {}", e);
                return Some(Msg::Error(e));
            }
        };
        let service_bus_client = self.service_bus_client.clone();

        log::info!(
            "Starting resend operation for message {} (sequence {}) from DLQ to queue {}",
            message_id,
            message_sequence,
            main_queue_name
        );

        let task = async move {
            log::debug!("Executing resend operation in background task");

            // Add overall timeout to the entire resend operation
            let dlq_config = crate::config::CONFIG.dlq();
            let overall_timeout_secs = (dlq_config.receive_timeout_secs()
                + dlq_config.send_timeout_secs())
            .min(dlq_config.overall_timeout_cap_secs());
            log::debug!(
                "Using overall timeout of {} seconds for resend operation",
                overall_timeout_secs
            );

            let result = tokio::time::timeout(
                std::time::Duration::from_secs(overall_timeout_secs),
                Self::execute_resend_operation(
                    consumer,
                    message_id.clone(),
                    message_sequence,
                    main_queue_name,
                    service_bus_client,
                ),
            )
            .await;

            match result {
                Ok(Ok(())) => {
                    log::info!(
                        "Resend operation completed successfully for message {}",
                        message_id
                    );
                    Self::handle_resend_success(&tx_to_main, &message_id, message_sequence);
                }
                Ok(Err(e)) => {
                    log::error!("Failed to resend message {}: {}", message_id, e);
                    Self::handle_resend_error(&tx_to_main, &tx_to_main_err, e);
                }
                Err(_) => {
                    log::error!(
                        "Overall timeout for resend operation after {} seconds",
                        overall_timeout_secs
                    );
                    let timeout_error = crate::error::AppError::ServiceBus(format!(
                        "Resend operation timed out after {} seconds",
                        overall_timeout_secs
                    ));
                    Self::handle_resend_error(&tx_to_main, &tx_to_main_err, timeout_error);
                }
            }
        };

        taskpool.execute(task);

        None
    }

    /// Executes the resend operation: receive message from DLQ, send to main queue, complete DLQ message
    async fn execute_resend_operation(
        consumer: Arc<Mutex<Consumer>>,
        message_id: String,
        message_sequence: i64,
        main_queue_name: String,
        service_bus_client: Arc<
            Mutex<azservicebus::ServiceBusClient<azservicebus::core::BasicRetryPolicy>>,
        >,
    ) -> Result<(), crate::error::AppError> {
        log::debug!("Acquiring consumer lock for resend operation");
        let mut consumer = consumer.lock().await;

        // Find the target message in DLQ using shared utility
        log::debug!("Searching for target message in DLQ");
        let target_msg = super::utils::find_target_message(&mut consumer, &message_id, message_sequence).await?;

        // Get the message body and properties for resending
        log::debug!("Extracting message body for resending");
        let message_body = target_msg.body().map_err(|e| {
            log::error!("Failed to get message body: {}", e);
            crate::error::AppError::ServiceBus(e.to_string())
        })?;

        // Create a new message with the same content using Producer helper
        log::debug!("Creating new message with {} bytes", message_body.len());
        let new_message = Producer::create_message(message_body.to_vec());

        // Send the message to the main queue
        log::info!(
            "Resending message {} from DLQ to main queue {}",
            message_id,
            main_queue_name
        );
        Self::send_message_to_main_queue(&main_queue_name, new_message, service_bus_client).await?;

        // Complete the original DLQ message to remove it from DLQ
        log::info!(
            "Completing DLQ message {} to remove it from dead letter queue",
            message_id
        );
        consumer.complete_message(&target_msg).await.map_err(|e| {
            log::error!("Failed to complete DLQ message: {}", e);
            crate::error::AppError::ServiceBus(e.to_string())
        })?;

        log::info!(
            "Successfully resent message {} from dead letter queue to main queue",
            message_id
        );

        Ok(())
    }

    /// Get the main queue name from the current DLQ queue name
    fn get_main_queue_name_from_current_dlq(&self) -> Result<String, crate::error::AppError> {
        if let Some(current_queue_name) = &self.queue_state.current_queue_name {
            if self.queue_state.current_queue_type
                == crate::components::common::QueueType::DeadLetter
            {
                // Remove the /$deadletterqueue suffix to get the main queue name
                if let Some(main_name) = current_queue_name.strip_suffix("/$deadletterqueue") {
                    Ok(main_name.to_string())
                } else {
                    log::error!(
                        "Current queue name '{}' doesn't have expected DLQ suffix '/$deadletterqueue'",
                        current_queue_name
                    );
                    Err(crate::error::AppError::State(
                        "Current queue name doesn't have expected DLQ suffix".to_string(),
                    ))
                }
            } else {
                log::error!(
                    "Cannot resend from main queue - current queue type is {:?}",
                    self.queue_state.current_queue_type
                );
                Err(crate::error::AppError::State(
                    "Cannot resend from main queue - only from dead letter queue".to_string(),
                ))
            }
        } else {
            log::error!("No current queue name available");
            Err(crate::error::AppError::State(
                "No current queue name available".to_string(),
            ))
        }
    }

    /// Send a message to the main queue using Producer
    async fn send_message_to_main_queue(
        queue_name: &str,
        message: azservicebus::ServiceBusMessage,
        service_bus_client: Arc<
            Mutex<azservicebus::ServiceBusClient<azservicebus::core::BasicRetryPolicy>>,
        >,
    ) -> Result<(), crate::error::AppError> {
        // Use configurable timeout with cap to avoid hanging - Azure Service Bus might have internal timeouts
        let dlq_config = crate::config::CONFIG.dlq();
        let send_timeout_secs = dlq_config
            .send_timeout_secs()
            .min(dlq_config.send_timeout_cap_secs());

        log::debug!(
            "Creating producer for queue: {} (timeout: {}s)",
            queue_name,
            send_timeout_secs
        );

        // Add timeout to the entire send operation
        let send_result =
            tokio::time::timeout(std::time::Duration::from_secs(send_timeout_secs), async {
                log::debug!("Acquiring service bus client lock for sending");
                // Acquire the service bus client lock
                let mut client = service_bus_client.lock().await;

                log::debug!("Creating producer for queue: {}", queue_name);
                // Create a producer for the main queue
                let mut producer = client
                    .create_producer_for_queue(
                        queue_name,
                        azservicebus::ServiceBusSenderOptions::default(),
                    )
                    .await
                    .map_err(|e| {
                        log::error!("Failed to create producer for queue {}: {}", queue_name, e);
                        crate::error::AppError::ServiceBus(e.to_string())
                    })?;

                log::debug!("Sending message to queue: {}", queue_name);

                // Send the message using the producer
                producer.send_message(message).await.map_err(|e| {
                    log::error!("Failed to send message to queue {}: {}", queue_name, e);
                    crate::error::AppError::ServiceBus(e.to_string())
                })?;

                log::debug!("Message sent successfully, disposing producer");

                // Dispose the producer
                producer.dispose().await.map_err(|e| {
                    log::warn!("Failed to dispose producer for queue {}: {}", queue_name, e);
                    crate::error::AppError::ServiceBus(e.to_string())
                })?;

                log::debug!("Producer disposed successfully");
                Ok::<(), crate::error::AppError>(())
            })
            .await;

        match send_result {
            Ok(Ok(())) => {
                log::info!("Successfully sent message to queue: {}", queue_name);
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => {
                log::error!(
                    "Timeout while sending message to queue {} after {} seconds",
                    queue_name,
                    send_timeout_secs
                );
                Err(crate::error::AppError::ServiceBus(format!(
                    "Timeout while sending message to queue {} after {} seconds",
                    queue_name, send_timeout_secs
                )))
            }
        }
    }

    /// Handles successful resend operation
    fn handle_resend_success(
        tx_to_main: &Sender<crate::components::common::Msg>,
        message_id: &str,
        message_sequence: i64,
    ) {
        log::info!(
            "Resend operation completed successfully for message {} (sequence {})",
            message_id,
            message_sequence
        );

        // Stop loading indicator
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Stop,
        )) {
            log::error!("Failed to send loading stop message: {}", e);
        }

        // Remove the message from local state since it's been resent
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::MessageActivity(
            crate::components::common::MessageActivityMsg::RemoveMessageFromState(
                message_id.to_string(),
                message_sequence,
            ),
        )) {
            log::error!("Failed to send remove message from state message: {}", e);
        }
    }

    /// Handles resend operation errors
    fn handle_resend_error(
        tx_to_main: &Sender<crate::components::common::Msg>,
        tx_to_main_err: &Sender<crate::components::common::Msg>,
        error: crate::error::AppError,
    ) {
        log::error!("Error in resend operation: {}", error);

        // Stop loading indicator
        if let Err(err) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Stop,
        )) {
            log::error!("Failed to send loading stop message: {}", err);
        }

        // Send error message
        let _ = tx_to_main_err.send(crate::components::common::Msg::Error(error));
    }
} 