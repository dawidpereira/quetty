use crate::app::model::{AppState, Model};
use crate::components::common::{MessageActivityMsg, Msg};
use std::sync::Arc;
use tokio::sync::Mutex;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn update_messages(&mut self, msg: MessageActivityMsg) -> Option<Msg> {
        match msg {
            MessageActivityMsg::EditMessage(index) => {
                if let Err(e) = self.remount_message_details(index) {
                    return Some(Msg::Error(e));
                }
                self.app_state = AppState::MessageDetails;
                Some(Msg::ForceRedraw)
            }
            MessageActivityMsg::CancelEditMessage => {
                self.app_state = AppState::MessagePicker;
                None
            }
            MessageActivityMsg::MessagesLoaded(messages) => {
                self.messages = Some(messages);
                if let Err(e) = self.remount_messages() {
                    return Some(Msg::Error(e));
                }
                if let Err(e) = self.remount_message_details(0) {
                    return Some(Msg::Error(e));
                }
                self.app_state = AppState::MessagePicker;
                None
            }
            MessageActivityMsg::ConsumerCreated(consumer) => {
                self.consumer = Some(Arc::new(Mutex::new(consumer)));
                // Reset pagination state for new consumer
                self.current_page = 0;
                self.has_next_page = false;
                self.has_previous_page = false;
                self.all_loaded_messages.clear();
                self.total_pages_loaded = 0;
                self.last_loaded_sequence = None;
                if let Err(e) = self.load_messages() {
                    return Some(Msg::Error(e));
                }
                None
            }
            MessageActivityMsg::PreviewMessageDetails(index) => {
                if let Err(e) = self.remount_message_details(index) {
                    return Some(Msg::Error(e));
                }
                None
            }
            MessageActivityMsg::NextPage => {
                if self.has_next_page {
                    if let Err(e) = self.handle_next_page() {
                        return Some(Msg::Error(e));
                    }
                }
                None
            }
            MessageActivityMsg::PreviousPage => {
                if self.has_previous_page {
                    if let Err(e) = self.handle_previous_page() {
                        return Some(Msg::Error(e));
                    }
                }
                None
            }
            MessageActivityMsg::NewMessagesLoaded(new_messages) => {
                let is_initial_load = self.all_loaded_messages.is_empty();

                // Add new messages to our store
                self.all_loaded_messages.extend(new_messages);
                self.total_pages_loaded += 1;

                // Update last loaded sequence
                if let Some(last_msg) = self.all_loaded_messages.last() {
                    self.last_loaded_sequence = Some(last_msg.sequence);
                }

                // If this is not the initial load, advance to the new page
                if !is_initial_load {
                    self.current_page += 1;
                }

                // Update the current page view
                if let Err(e) = self.update_current_page_view() {
                    return Some(Msg::Error(e));
                }

                // Ensure we're in the right state and have message details
                if self.app_state != AppState::MessagePicker {
                    self.app_state = AppState::MessagePicker;
                }

                // Initialize message details if we have messages
                if !self.all_loaded_messages.is_empty() {
                    if let Err(e) = self.remount_message_details(0) {
                        return Some(Msg::Error(e));
                    }
                }

                None
            }
            MessageActivityMsg::PageChanged => {
                // Just update the view with current page
                if let Err(e) = self.update_current_page_view() {
                    return Some(Msg::Error(e));
                }
                None
            }
            MessageActivityMsg::PaginationStateUpdated {
                has_next,
                has_previous,
                current_page,
                total_pages_loaded,
            } => {
                self.has_next_page = has_next;
                self.has_previous_page = has_previous;
                self.current_page = current_page;
                self.total_pages_loaded = total_pages_loaded;
                None
            }
        }
    }

    pub fn handle_next_page(&mut self) -> crate::error::AppResult<()> {
        log::debug!(
            "Handle next page - current: {}, total_loaded: {}",
            self.current_page,
            self.total_pages_loaded
        );

        let next_page = self.current_page + 1;

        // Check if we already have this page loaded
        if next_page < self.total_pages_loaded {
            // We have this page in memory, just switch to it
            log::debug!("Page {} already loaded, switching view", next_page);
            self.current_page = next_page;
            self.update_pagination_state();

            // Send page changed message
            if let Err(e) = self
                .tx_to_main
                .send(crate::components::common::Msg::MessageActivity(
                    crate::components::common::MessageActivityMsg::PageChanged,
                ))
            {
                log::error!("Failed to send page changed message: {}", e);
            }
        } else {
            // Need to load new messages from API
            log::debug!("Loading new page {} from API", next_page);
            self.load_new_messages_from_api()?;
        }

        Ok(())
    }

    pub fn handle_previous_page(&mut self) -> crate::error::AppResult<()> {
        log::debug!("Handle previous page - current: {}", self.current_page);

        if self.current_page > 0 {
            self.current_page -= 1;
            self.update_pagination_state();

            // Send page changed message
            if let Err(e) = self
                .tx_to_main
                .send(crate::components::common::Msg::MessageActivity(
                    crate::components::common::MessageActivityMsg::PageChanged,
                ))
            {
                log::error!("Failed to send page changed message: {}", e);
            }
        }

        Ok(())
    }

    fn update_pagination_state(&mut self) {
        self.has_previous_page = self.current_page > 0;
        // We have next page if we've loaded more pages than current + 1, or if we might have more to load
        self.has_next_page = self.current_page + 1 < self.total_pages_loaded
            || (self.total_pages_loaded > 0
                && self.all_loaded_messages.len() % crate::config::CONFIG.max_messages() as usize
                    == 0);

        log::debug!(
            "Updated pagination state: current={}, total_loaded={}, has_prev={}, has_next={}",
            self.current_page,
            self.total_pages_loaded,
            self.has_previous_page,
            self.has_next_page
        );
    }

    fn update_current_page_view(&mut self) -> crate::error::AppResult<()> {
        let page_size = crate::config::CONFIG.max_messages() as usize;
        let start_idx = self.current_page * page_size;
        let end_idx = std::cmp::min(start_idx + page_size, self.all_loaded_messages.len());

        let current_page_messages = if start_idx < self.all_loaded_messages.len() {
            self.all_loaded_messages[start_idx..end_idx].to_vec()
        } else {
            Vec::new()
        };

        log::debug!(
            "Updating view for page {}: showing messages {}-{} of {}",
            self.current_page,
            start_idx,
            end_idx,
            self.all_loaded_messages.len()
        );

        self.messages = Some(current_page_messages.clone());

        // Update pagination state
        self.update_pagination_state();

        // Send pagination state update
        if let Err(e) = self
            .tx_to_main
            .send(crate::components::common::Msg::MessageActivity(
                crate::components::common::MessageActivityMsg::PaginationStateUpdated {
                    has_next: self.has_next_page,
                    has_previous: self.has_previous_page,
                    current_page: self.current_page,
                    total_pages_loaded: self.total_pages_loaded,
                },
            ))
        {
            log::error!("Failed to send pagination state update: {}", e);
        }

        // Remount messages component with new data
        self.remount_messages()?;

        Ok(())
    }

    fn load_new_messages_from_api(&mut self) -> crate::error::AppResult<()> {
        log::debug!(
            "Loading new messages from API, last_sequence: {:?}",
            self.last_loaded_sequence
        );

        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        // Show loading indicator
        if let Err(e) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
            crate::components::common::LoadingActivityMsg::Start(
                "Loading more messages...".to_string(),
            ),
        )) {
            log::error!("Failed to send loading start message: {}", e);
        }

        let consumer = self.consumer.clone().ok_or_else(|| {
            log::error!("No consumer available");
            crate::error::AppError::State("No consumer available".to_string())
        })?;

        let tx_to_main_err = tx_to_main.clone();
        let from_sequence = self.last_loaded_sequence.map(|seq| seq + 1);

        taskpool.execute(async move {
            let result = async {
                log::debug!("Acquiring consumer lock");
                let mut consumer = consumer.lock().await;
                log::debug!("Peeking messages with sequence: {:?}", from_sequence);

                let messages = consumer
                    .peek_messages(crate::config::CONFIG.max_messages(), from_sequence)
                    .await
                    .map_err(|e| {
                        log::error!("Failed to peek messages: {}", e);
                        crate::error::AppError::ServiceBus(e.to_string())
                    })?;

                log::info!("Loaded {} new messages from API", messages.len());

                // Stop loading indicator
                if let Err(e) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
                    crate::components::common::LoadingActivityMsg::Stop,
                )) {
                    log::error!("Failed to send loading stop message: {}", e);
                }

                // Send new messages
                if !messages.is_empty() {
                    tx_to_main
                        .send(crate::components::common::Msg::MessageActivity(
                            crate::components::common::MessageActivityMsg::NewMessagesLoaded(
                                messages,
                            ),
                        ))
                        .map_err(|e| {
                            log::error!("Failed to send new messages loaded message: {}", e);
                            crate::error::AppError::Component(e.to_string())
                        })?;
                } else {
                    log::debug!("No new messages available");
                    // Still need to update the view to reflect that we tried to load
                    tx_to_main
                        .send(crate::components::common::Msg::MessageActivity(
                            crate::components::common::MessageActivityMsg::PageChanged,
                        ))
                        .map_err(|e| {
                            log::error!("Failed to send page changed message: {}", e);
                            crate::error::AppError::Component(e.to_string())
                        })?;
                }

                Ok::<(), crate::error::AppError>(())
            }
            .await;

            if let Err(e) = result {
                log::error!("Error in message loading task: {}", e);

                // Stop loading indicator even if there was an error
                if let Err(err) = tx_to_main.send(crate::components::common::Msg::LoadingActivity(
                    crate::components::common::LoadingActivityMsg::Stop,
                )) {
                    log::error!("Failed to send loading stop message: {}", err);
                }

                // Send error message
                let _ = tx_to_main_err.send(crate::components::common::Msg::Error(e));
            }
        });

        Ok(())
    }
}

