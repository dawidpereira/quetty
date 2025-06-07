use crate::app::model::Model;
use crate::components::common::{
    LoadingActivityMsg, MessageActivityMsg, Msg, QueueActivityMsg, QueueType,
};
use crate::config;
use crate::error::{AppError, AppResult};
use crate::theme::ThemeManager;
use server::bulk_operations::MessageIdentifier;
use server::model::MessageModel;
use tui_realm_stdlib::Table;
use tuirealm::Frame;
use tuirealm::command::{Cmd, CmdResult, Direction};
use tuirealm::event::{Key, KeyEvent, KeyModifiers};
use tuirealm::props::{Alignment, BorderType, Borders, Color, TableBuilder, TextSpan};
use tuirealm::ratatui::layout::Rect;
use tuirealm::ratatui::style::Style;
use tuirealm::ratatui::widgets::Paragraph;
use tuirealm::terminal::TerminalAdapter;
use tuirealm::{
    AttrValue, Attribute, Component, Event, MockComponent, NoUserEvent, State, StateValue,
};

#[derive(Debug, Clone)]
pub struct PaginationInfo {
    pub current_page: usize,
    pub total_pages_loaded: usize,
    pub total_messages_loaded: usize,
    pub current_page_size: usize,
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub queue_name: Option<String>,
    pub queue_type: QueueType,
    // Bulk selection info
    pub bulk_mode: bool,
    pub selected_count: usize,
}

pub struct Messages {
    component: Table,
    // Store data for direct rendering
    messages: Option<Vec<MessageModel>>,
    pagination_info: Option<PaginationInfo>,
    selected_messages: Vec<MessageIdentifier>,
    title: String,
    headers: Vec<String>,
    widths: Vec<u16>,
    is_focused: bool, // Track focus state for border styling
}

const CMD_RESULT_MESSAGE_SELECTED: &str = "MessageSelected";
const CMD_RESULT_MESSAGE_PREVIEW: &str = "MessagePreview";
const CMD_RESULT_QUEUE_UNSELECTED: &str = "QueueUnSelected";

impl Messages {
    pub fn new(messages: Option<&Vec<MessageModel>>) -> Self {
        Self::new_with_pagination(messages, None)
    }

    pub fn new_with_pagination(
        messages: Option<&Vec<MessageModel>>,
        pagination_info: Option<PaginationInfo>,
    ) -> Self {
        Self::new_with_pagination_and_selections(messages, pagination_info, Vec::new())
    }

    pub fn new_with_pagination_and_selections(
        messages: Option<&Vec<MessageModel>>,
        pagination_info: Option<PaginationInfo>,
        selected_messages: Vec<MessageIdentifier>,
    ) -> Self {
        Self::new_with_pagination_selections_and_focus(
            messages,
            pagination_info,
            selected_messages,
            false,
        )
    }

    pub fn new_with_pagination_selections_and_focus(
        messages: Option<&Vec<MessageModel>>,
        pagination_info: Option<PaginationInfo>,
        selected_messages: Vec<MessageIdentifier>,
        is_focused: bool,
    ) -> Self {
        // Simplified title - just show queue info, no pagination details
        let title = if let Some(info) = &pagination_info {
            let queue_display = Self::format_queue_display(info);
            format!(" {} ", queue_display)
        } else {
            " Messages ".to_string()
        };

        let (headers, widths) = if pagination_info.as_ref().is_some_and(|info| info.bulk_mode) {
            // In bulk mode, add checkbox column with circular checkboxes
            (
                vec![
                    "●○".to_string(),
                    "Sequence".to_string(),
                    "Message ID".to_string(),
                    "Enqueued At".to_string(),
                    "Delivery Count".to_string(),
                ],
                vec![5, 9, 28, 24, 15],
            )
        } else {
            (
                vec![
                    "Sequence".to_string(),
                    "Message ID".to_string(),
                    "Enqueued At".to_string(),
                    "Delivery Count".to_string(),
                ],
                vec![10, 30, 25, 16],
            )
        };

        let component = {
            Table::default()
                .borders(
                    Borders::default()
                        .modifiers(BorderType::Rounded)
                        .color(ThemeManager::primary_accent()),
                )
                .background(Color::Reset)
                .foreground(ThemeManager::text_primary())
                .title(&title, Alignment::Center)
                .scroll(true)
                .highlighted_color(ThemeManager::selection_bg())
                .highlighted_str("► ")
                .rewind(false)
                .step(4)
                .row_height(1)
                .headers(&headers.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                .column_spacing(2)
                .widths(&widths)
                .table(Self::build_table_from_messages(
                    messages,
                    pagination_info.as_ref(),
                    &selected_messages,
                ))
        };

        Self {
            component,
            messages: messages.cloned(),
            pagination_info,
            selected_messages,
            title,
            headers,
            widths,
            is_focused,
        }
    }

    fn format_queue_display(info: &PaginationInfo) -> String {
        let queue_name = info.queue_name.as_deref().unwrap_or("Unknown Queue");
        match info.queue_type {
            QueueType::Main => format!("📬 Messages ({}) [Main → d:DLQ]", queue_name),
            QueueType::DeadLetter => {
                format!("💀 Dead Letter Queue ({}) [DLQ → d:Main]", queue_name)
            }
        }
    }

    fn format_bulk_info(info: &PaginationInfo) -> String {
        if info.bulk_mode && info.selected_count > 0 {
            format!("• {} selected", info.selected_count)
        } else if info.bulk_mode {
            "• Bulk mode".to_string()
        } else {
            "".to_string()
        }
    }

    fn format_navigation_hints(info: &PaginationInfo) -> String {
        let mut hints = Vec::new();

        if info.has_previous_page {
            hints.push("◀[p]");
        }
        if info.has_next_page {
            hints.push("[n]▶");
        }

        if hints.is_empty() {
            "• End of pages".to_string()
        } else {
            format!("• {}", hints.join(" "))
        }
    }

    fn format_pagination_status(info: &PaginationInfo) -> String {
        let bulk_info = Self::format_bulk_info(info);
        let navigation_hints = Self::format_navigation_hints(info);

        if info.total_messages_loaded == 0 {
            format!("No messages available {}", bulk_info)
        } else {
            format!(
                "Page {}/{} • {} total • {} on page {} {}",
                info.current_page + 1, // Display as 1-based
                info.total_pages_loaded.max(1),
                info.total_messages_loaded,
                info.current_page_size,
                navigation_hints,
                bulk_info
            )
        }
    }

    fn build_table_from_messages(
        messages: Option<&Vec<MessageModel>>,
        pagination_info: Option<&PaginationInfo>,
        selected_messages: &[MessageIdentifier],
    ) -> Vec<Vec<TextSpan>> {
        if let Some(messages) = messages {
            let mut builder = TableBuilder::default();
            let bulk_mode = pagination_info.is_some_and(|info| info.bulk_mode);

            for msg in messages {
                if bulk_mode {
                    // Add checkbox column in bulk mode with themed checkboxes
                    let message_id = MessageIdentifier::from_message(msg);
                    let checkbox_text = if selected_messages.contains(&message_id) {
                        "● " // Filled circle for checked
                    } else {
                        "○ " // Empty circle for unchecked
                    };
                    builder.add_col(TextSpan::from(checkbox_text));
                }

                builder
                    .add_col(TextSpan::from(msg.sequence.to_string()))
                    .add_col(TextSpan::from(msg.id.to_string()))
                    .add_col(TextSpan::from(msg.enqueued_at.to_string()))
                    .add_col(TextSpan::from(msg.delivery_count.to_string()))
                    .add_row();
            }
            return builder.build();
        }
        Vec::new()
    }

    /// Create a message identifier from index - this will send a message to get the actual message data
    fn create_toggle_message_selection(index: usize) -> Msg {
        Msg::MessageActivity(MessageActivityMsg::ToggleMessageSelectionByIndex(index))
    }
}

impl Component<Msg, NoUserEvent> for Messages {
    #[allow(clippy::too_many_lines)]
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let cmd_result = match ev {
            // Bulk selection key bindings
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) if c == config::CONFIG.keys().toggle_selection() => {
                // Toggle selection for current message
                let index = match self.state() {
                    tuirealm::State::One(StateValue::Usize(index)) => index,
                    _ => 0,
                };

                return Some(Self::create_toggle_message_selection(index));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::CONTROL,
            }) if c == config::CONFIG.keys().select_all_page() => {
                // Select all messages on current page
                return Some(Msg::MessageActivity(
                    MessageActivityMsg::SelectAllCurrentPage,
                ));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('A'),
                modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            }) => {
                // Select all loaded messages across all pages
                return Some(Msg::MessageActivity(
                    MessageActivityMsg::SelectAllLoadedMessages,
                ));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Esc,
                modifiers: KeyModifiers::NONE,
            }) => {
                // In bulk mode, clear selections. Otherwise, go back
                // We'll let the handler decide based on current state
                return Some(Msg::MessageActivity(MessageActivityMsg::ClearAllSelections));
            }

            // Enhanced existing operations for bulk mode
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::CONTROL,
            }) if c == config::CONFIG.keys().send_to_dlq() => {
                // Check if we should do bulk operation or single message operation
                // We'll send the bulk message and let the handler decide
                return Some(Msg::MessageActivity(
                    MessageActivityMsg::BulkSendSelectedToDLQ,
                ));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) if c == config::CONFIG.keys().send_to_dlq() => {
                // Single send to DLQ
                return Some(Msg::MessageActivity(
                    MessageActivityMsg::BulkSendSelectedToDLQ,
                ));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) if c == config::CONFIG.keys().resend_from_dlq() => {
                // Resend only (without deleting from DLQ)
                return Some(Msg::MessageActivity(
                    MessageActivityMsg::BulkResendSelectedFromDLQ(false),
                ));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::SHIFT,
            }) if c == config::CONFIG.keys().resend_and_delete_from_dlq() => {
                // Resend and delete from DLQ
                return Some(Msg::MessageActivity(
                    MessageActivityMsg::BulkResendSelectedFromDLQ(true),
                ));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Delete,
                modifiers: KeyModifiers::NONE,
            }) => {
                // Check if we should do bulk operation or single message operation
                return Some(Msg::MessageActivity(MessageActivityMsg::BulkDeleteSelected));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::CONTROL,
            }) if c == config::CONFIG.keys().alt_delete_message() => {
                // Bulk delete with Ctrl+X
                return Some(Msg::MessageActivity(MessageActivityMsg::BulkDeleteSelected));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) if c == config::CONFIG.keys().delete_message() => {
                // Single delete with X
                return Some(Msg::MessageActivity(MessageActivityMsg::BulkDeleteSelected));
            }

            // Navigation keys
            Event::Keyboard(KeyEvent {
                code: Key::Down,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.component.perform(Cmd::Move(Direction::Down));
                CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, self.state())
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) if c == config::CONFIG.keys().down() => {
                self.component.perform(Cmd::Move(Direction::Down));
                CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, self.state())
            }
            Event::Keyboard(KeyEvent {
                code: Key::Up,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.component.perform(Cmd::Move(Direction::Up));
                CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, self.state())
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) if c == config::CONFIG.keys().up() => {
                self.component.perform(Cmd::Move(Direction::Up));
                CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, self.state())
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageDown,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.component.perform(Cmd::Scroll(Direction::Down));
                CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, self.state())
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageUp,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.component.perform(Cmd::Scroll(Direction::Up));
                CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, self.state())
            }
            Event::Keyboard(KeyEvent {
                code: Key::Enter,
                modifiers: KeyModifiers::NONE,
            }) => CmdResult::Custom(CMD_RESULT_MESSAGE_SELECTED, self.state()),

            // Pagination
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) if c == config::CONFIG.keys().next_page() => {
                return Some(Msg::MessageActivity(MessageActivityMsg::NextPage));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) if c == config::CONFIG.keys().alt_next_page() => {
                return Some(Msg::MessageActivity(MessageActivityMsg::NextPage));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) if c == config::CONFIG.keys().prev_page() => {
                return Some(Msg::MessageActivity(MessageActivityMsg::PreviousPage));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) if c == config::CONFIG.keys().alt_prev_page() => {
                return Some(Msg::MessageActivity(MessageActivityMsg::PreviousPage));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('d'),
                modifiers: KeyModifiers::NONE,
            }) => return Some(Msg::QueueActivity(QueueActivityMsg::ToggleDeadLetterQueue)),

            _ => CmdResult::None,
        };

        match cmd_result {
            CmdResult::Custom(CMD_RESULT_MESSAGE_SELECTED, state) => match state {
                tuirealm::State::One(StateValue::Usize(index)) => {
                    Some(Msg::MessageActivity(MessageActivityMsg::EditMessage(index)))
                }
                _ => None,
            },
            CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, state) => match state {
                tuirealm::State::One(StateValue::Usize(index)) => Some(Msg::MessageActivity(
                    MessageActivityMsg::PreviewMessageDetails(index),
                )),
                _ => None,
            },
            CmdResult::Custom(CMD_RESULT_QUEUE_UNSELECTED, _) => {
                Some(Msg::QueueActivity(QueueActivityMsg::QueueUnselected))
            }
            _ => Some(Msg::ForceRedraw),
        }
    }
}

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn load_messages(&mut self) -> AppResult<()> {
        let taskpool = &self.taskpool;
        let tx_to_main = self.tx_to_main.clone();

        // Show loading indicator
        if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Start(
            "Loading messages...".to_string(),
        ))) {
            log::error!("Failed to send loading start message: {}", e);
        }

        let consumer = self.queue_state.consumer.clone().ok_or_else(|| {
            log::error!("No consumer available");
            AppError::State("No consumer available".to_string())
        })?;

        let tx_to_main_err = tx_to_main.clone();
        taskpool.execute(async move {
            let result = async {
                let mut consumer = consumer.lock().await;

                let messages = consumer
                    .peek_messages(config::CONFIG.max_messages(), None)
                    .await
                    .map_err(|e| {
                        log::error!("Failed to peek messages: {}", e);
                        AppError::ServiceBus(e.to_string())
                    })?;

                log::info!("Loaded {} messages", messages.len());

                // Stop loading indicator
                if let Err(e) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Stop)) {
                    log::error!("Failed to send loading stop message: {}", e);
                }

                // Send initial messages as new messages loaded
                if !messages.is_empty() {
                    tx_to_main
                        .send(Msg::MessageActivity(MessageActivityMsg::NewMessagesLoaded(
                            messages,
                        )))
                        .map_err(|e| {
                            log::error!("Failed to send new messages loaded message: {}", e);
                            AppError::Component(e.to_string())
                        })?;
                } else {
                    // No messages, but still need to update the view
                    tx_to_main
                        .send(Msg::MessageActivity(MessageActivityMsg::MessagesLoaded(
                            messages,
                        )))
                        .map_err(|e| {
                            log::error!("Failed to send messages loaded message: {}", e);
                            AppError::Component(e.to_string())
                        })?;
                }

                Ok::<(), AppError>(())
            }
            .await;
            if let Err(e) = result {
                log::error!("Error in message loading task: {}", e);

                // Stop loading indicator even if there was an error
                if let Err(err) = tx_to_main.send(Msg::LoadingActivity(LoadingActivityMsg::Stop)) {
                    log::error!("Failed to send loading stop message: {}", err);
                }

                // Send error message
                let _ = tx_to_main_err.send(Msg::Error(e));
            }
        });

        Ok(())
    }
}

impl MockComponent for Messages {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Get the current selection from the internal component
        let table_state = self.component.state();
        let selected_index = match table_state {
            tuirealm::State::One(StateValue::Usize(index)) => index,
            _ => 0,
        };

        // Build the table rows for ratatui
        let mut rows = Vec::new();
        if let Some(ref messages) = self.messages {
            let bulk_mode = self
                .pagination_info
                .as_ref()
                .is_some_and(|info| info.bulk_mode);

            for (i, msg) in messages.iter().enumerate() {
                let mut cells = Vec::new();

                if bulk_mode {
                    // Add checkbox column in bulk mode
                    let message_id = MessageIdentifier::from_message(msg);
                    let checkbox_text = if self.selected_messages.contains(&message_id) {
                        "●"
                    } else {
                        "○"
                    };
                    cells.push(tuirealm::ratatui::widgets::Cell::from(checkbox_text));
                }

                // Add the message data cells with proper theming
                cells.push(
                    tuirealm::ratatui::widgets::Cell::from(msg.sequence.to_string()).style(
                        tuirealm::ratatui::style::Style::default()
                            .fg(ThemeManager::message_sequence()),
                    ),
                );
                cells.push(
                    tuirealm::ratatui::widgets::Cell::from(msg.id.to_string()).style(
                        tuirealm::ratatui::style::Style::default().fg(ThemeManager::message_id()),
                    ),
                );
                cells.push(
                    tuirealm::ratatui::widgets::Cell::from(msg.enqueued_at.to_string()).style(
                        tuirealm::ratatui::style::Style::default()
                            .fg(ThemeManager::message_timestamp()),
                    ),
                );
                cells.push(
                    tuirealm::ratatui::widgets::Cell::from(msg.delivery_count.to_string()).style(
                        tuirealm::ratatui::style::Style::default()
                            .fg(ThemeManager::message_delivery_count()),
                    ),
                );

                let mut row = tuirealm::ratatui::widgets::Row::new(cells);

                // Apply selection highlighting
                if i == selected_index {
                    row = row.style(
                        tuirealm::ratatui::style::Style::default()
                            .bg(ThemeManager::selection_bg())
                            .fg(ThemeManager::selection_fg()),
                    );
                }

                rows.push(row);
            }
        }

        // Create the table headers with proper theming
        let header_cells: Vec<tuirealm::ratatui::widgets::Cell> = self
            .headers
            .iter()
            .map(|h| {
                tuirealm::ratatui::widgets::Cell::from(h.as_str()).style(
                    tuirealm::ratatui::style::Style::default()
                        .fg(ThemeManager::header_accent()) // Always yellow to match line numbers
                        .add_modifier(tuirealm::ratatui::style::Modifier::BOLD),
                )
            })
            .collect();

        let header = tuirealm::ratatui::widgets::Row::new(header_cells).height(1);

        // Create the table widget with proper theming
        let table = tuirealm::ratatui::widgets::Table::new(
            rows,
            &self
                .widths
                .iter()
                .map(|&w| tuirealm::ratatui::layout::Constraint::Length(w))
                .collect::<Vec<_>>(),
        )
        .header(header)
        .block(
            tuirealm::ratatui::widgets::Block::default()
                .borders(tuirealm::ratatui::widgets::Borders::ALL)
                .border_type(tuirealm::ratatui::widgets::BorderType::Rounded)
                .border_style(
                    tuirealm::ratatui::style::Style::default().fg(if self.is_focused {
                        ThemeManager::primary_accent() // Teal when focused
                    } else {
                        tuirealm::ratatui::style::Color::White // White when not focused
                    }),
                )
                .title(self.title.as_str())
                .title_alignment(tuirealm::ratatui::layout::Alignment::Center)
                .title_style(
                    tuirealm::ratatui::style::Style::default()
                        .fg(ThemeManager::title_accent()) // Use pink to match message details title
                        .add_modifier(tuirealm::ratatui::style::Modifier::BOLD),
                ),
        )
        .column_spacing(2)
        .row_highlight_style(
            tuirealm::ratatui::style::Style::default()
                .bg(ThemeManager::selection_bg())
                .fg(ThemeManager::selection_fg())
                .add_modifier(tuirealm::ratatui::style::Modifier::BOLD),
        )
        .highlight_symbol("► ");

        // Create table state for selection
        let mut table_state = tuirealm::ratatui::widgets::TableState::default();
        table_state.select(Some(selected_index));

        // Render the table
        frame.render_stateful_widget(table, area, &mut table_state);

        // Create status bar overlay at the bottom with pagination info
        if let Some(ref info) = self.pagination_info {
            let status_text = Self::format_pagination_status(info);
            let status_bar = Paragraph::new(status_text)
                .style(
                    Style::default().fg(if self.is_focused {
                        ThemeManager::primary_accent() // Teal text when focused
                    } else {
                        tuirealm::ratatui::style::Color::White // White text when not focused
                    }), // No background - clean and transparent
                )
                .alignment(tuirealm::ratatui::layout::Alignment::Center);

            // Position status bar at the exact same height as table bottom border
            let status_area = Rect {
                x: area.x,
                y: area.y + area.height.saturating_sub(1),
                width: area.width,
                height: 1,
            };

            frame.render_widget(status_bar, status_area);
        }
    }

    fn query(&self, attr: Attribute) -> Option<AttrValue> {
        self.component.query(attr)
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        match attr {
            Attribute::Custom("cursor_position") => {
                if let AttrValue::Number(position) = value {
                    log::debug!("Received cursor position attribute: {}", position);
                    // Try to set the cursor position using the Table component's perform method
                    let target_position = position as usize;

                    // Move cursor to target position by performing multiple Down movements
                    // This is a workaround since we can't directly set cursor position
                    for _ in 0..target_position {
                        self.component.perform(Cmd::Move(Direction::Down));
                    }

                    log::debug!("Attempted to move cursor to position: {}", target_position);
                }
            }
            _ => {
                self.component.attr(attr, value);
            }
        }
    }

    fn state(&self) -> State {
        self.component.state()
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        self.component.perform(cmd)
    }
}
