use crate::components::common::{MessageActivityMsg, Msg, QueueActivityMsg, QueueType};
use crate::components::messages::rendering::{
    build_table_from_messages, calculate_responsive_layout, format_delivery_count_responsive,
    get_state_color, get_state_display,
};
use crate::components::messages::selection::{
    create_toggle_message_selection, format_pagination_status, format_queue_display,
};
use crate::config;
use crate::theme::ThemeManager;
use server::bulk_operations::MessageIdentifier;
use server::model::MessageModel;
use tui_realm_stdlib::Table;
use tuirealm::command::{Cmd, CmdResult, Direction};
use tuirealm::event::{Key, KeyEvent, KeyModifiers};
use tuirealm::props::{Alignment, BorderType, Borders, Color, Style};
use tuirealm::ratatui::layout::{Alignment as RatatuiAlignment, Constraint, Rect};
use tuirealm::ratatui::style::{Color as RatatuiColor, Modifier, Style as RatatuiStyle};
use tuirealm::ratatui::widgets::{
    Block, BorderType as RatatuiBorderType, Borders as RatatuiBorders, Cell, Paragraph, Row,
    Table as RatatuiTable, TableState,
};
use tuirealm::{
    AttrValue, Attribute, Component, Event, Frame, MockComponent, NoUserEvent, State, StateValue,
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
    is_focused: bool,    // Track focus state for border styling
    narrow_layout: bool, // Track if we're using narrow layout for responsive formatting
}

const CMD_RESULT_MESSAGE_SELECTED: &str = "MessageSelected";
const CMD_RESULT_MESSAGE_PREVIEW: &str = "MessagePreview";
const CMD_RESULT_QUEUE_UNSELECTED: &str = "QueueUnSelected";

/// Get current index from table state
fn get_current_index_from_state(state: &State) -> usize {
    match state {
        State::One(StateValue::Usize(index)) => *index,
        _ => 0,
    }
}

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
            let queue_display = format_queue_display(info);
            format!(" {} ", queue_display)
        } else {
            " Messages ".to_string()
        };

        let (headers, widths, use_narrow_layout) = calculate_responsive_layout(
            120, // Default width, will be recalculated in view()
            pagination_info.as_ref().is_some_and(|info| info.bulk_mode),
        );

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
                .table(build_table_from_messages(
                    messages,
                    pagination_info.as_ref(),
                    &selected_messages,
                    &widths,
                    use_narrow_layout,
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
            narrow_layout: use_narrow_layout,
        }
    }

    /// Get the current selection index
    pub fn get_current_index(&self) -> usize {
        get_current_index_from_state(&self.component.state())
    }

    /// Get the number of messages currently available
    pub fn get_message_count(&self) -> usize {
        self.messages.as_ref().map_or(0, |msgs| msgs.len())
    }

    /// Move selection down with bounds checking
    pub fn move_down(&mut self) {
        let current = self.get_current_index();
        let max_index = self.get_message_count().saturating_sub(1);

        if current < max_index {
            self.component.perform(Cmd::Move(Direction::Down));
        }
    }

    /// Move selection up with bounds checking
    pub fn move_up(&mut self) {
        let current = self.get_current_index();
        if current > 0 {
            self.component.perform(Cmd::Move(Direction::Up));
        }
    }

    /// Page down with bounds checking
    pub fn page_down(&mut self) {
        let current = self.get_current_index();
        let max_index = self.get_message_count().saturating_sub(1);

        if current < max_index {
            self.component.perform(Cmd::Scroll(Direction::Down));
            // Ensure we don't go beyond the last item
            let new_index = self.get_current_index();
            if new_index > max_index {
                // Reset to the last valid position
                let moves_back = new_index - max_index;
                for _ in 0..moves_back {
                    self.component.perform(Cmd::Move(Direction::Up));
                }
            }
        }
    }

    /// Page up with bounds checking
    pub fn page_up(&mut self) {
        self.component.perform(Cmd::Scroll(Direction::Up));
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
                    State::One(StateValue::Usize(index)) => index,
                    _ => 0,
                };

                return Some(create_toggle_message_selection(index));
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

            // Enhanced existing operations for bulk mode - context-aware send/resend
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::CONTROL,
            }) if c == config::CONFIG.keys().send_to_dlq() => {
                // Context-aware operation based on current queue type
                if let Some(pagination_info) = &self.pagination_info {
                    match pagination_info.queue_type {
                        QueueType::Main => {
                            // In main queue: send to DLQ
                            return Some(Msg::MessageActivity(
                                MessageActivityMsg::BulkSendSelectedToDLQ,
                            ));
                        }
                        QueueType::DeadLetter => {
                            // In DLQ: resend to main queue (keep in DLQ)
                            return Some(Msg::MessageActivity(
                                MessageActivityMsg::BulkResendSelectedFromDLQ(false),
                            ));
                        }
                    }
                } else {
                    // Fallback to send to DLQ if no pagination info available
                    return Some(Msg::MessageActivity(
                        MessageActivityMsg::BulkSendSelectedToDLQ,
                    ));
                }
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) if c == config::CONFIG.keys().send_to_dlq() => {
                // Context-aware operation based on current queue type
                if let Some(pagination_info) = &self.pagination_info {
                    match pagination_info.queue_type {
                        QueueType::Main => {
                            // In main queue: send to DLQ
                            return Some(Msg::MessageActivity(
                                MessageActivityMsg::BulkSendSelectedToDLQ,
                            ));
                        }
                        QueueType::DeadLetter => {
                            // In DLQ: resend to main queue (keep in DLQ)
                            return Some(Msg::MessageActivity(
                                MessageActivityMsg::BulkResendSelectedFromDLQ(false),
                            ));
                        }
                    }
                } else {
                    // Fallback to send to DLQ if no pagination info available
                    return Some(Msg::MessageActivity(
                        MessageActivityMsg::BulkSendSelectedToDLQ,
                    ));
                }
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
                self.move_down();
                CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, self.state())
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) if c == config::CONFIG.keys().down() => {
                self.move_down();
                CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, self.state())
            }
            Event::Keyboard(KeyEvent {
                code: Key::Up,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.move_up();
                CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, self.state())
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) if c == config::CONFIG.keys().up() => {
                self.move_up();
                CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, self.state())
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageDown,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.page_down();
                CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, self.state())
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageUp,
                modifiers: KeyModifiers::NONE,
            }) => {
                self.page_up();
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

            // Global navigation
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) if c == config::CONFIG.keys().help() => {
                return Some(Msg::ToggleHelpScreen);
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) if c == config::CONFIG.keys().quit() => return Some(Msg::AppClose),
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::CONTROL,
            }) if c == config::CONFIG.keys().quit() => return Some(Msg::AppClose),
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) if c == config::CONFIG.keys().quit() => {
                return Some(Msg::QueueActivity(QueueActivityMsg::QueueUnselected));
            }

            // Queue toggle
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) if c == config::CONFIG.keys().toggle_dlq() => {
                return Some(Msg::QueueActivity(QueueActivityMsg::ToggleDeadLetterQueue));
            }

            // Message composition
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) if c == config::CONFIG.keys().compose_multiple() => {
                return Some(Msg::MessageActivity(
                    MessageActivityMsg::SetMessageRepeatCount,
                ));
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::CONTROL,
            }) if c == config::CONFIG.keys().compose_single() => {
                return Some(Msg::MessageActivity(MessageActivityMsg::ComposeNewMessage));
            }

            _ => CmdResult::None,
        };

        match cmd_result {
            CmdResult::Custom(
                CMD_RESULT_MESSAGE_SELECTED,
                State::One(StateValue::Usize(index)),
            ) => Some(Msg::MessageActivity(MessageActivityMsg::EditMessage(index))),
            CmdResult::Custom(CMD_RESULT_MESSAGE_PREVIEW, State::One(StateValue::Usize(index))) => {
                Some(Msg::MessageActivity(
                    MessageActivityMsg::PreviewMessageDetails(index),
                ))
            }
            CmdResult::Custom(CMD_RESULT_QUEUE_UNSELECTED, _) => {
                Some(Msg::QueueActivity(QueueActivityMsg::QueueUnselected))
            }
            _ => Some(Msg::ForceRedraw),
        }
    }
}

impl MockComponent for Messages {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Recalculate responsive layout based on actual available width
        let bulk_mode = self
            .pagination_info
            .as_ref()
            .is_some_and(|info| info.bulk_mode);

        let (headers, widths, narrow_layout) = calculate_responsive_layout(area.width, bulk_mode);

        // Update stored layout info
        self.headers = headers;
        self.widths = widths.clone();
        self.narrow_layout = narrow_layout;

        // Get the current selection from the internal component
        let table_state = self.component.state();
        let selected_index = get_current_index_from_state(&table_state);

        let mut rows = Vec::new();
        if let Some(ref messages) = self.messages {
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
                    cells.push(Cell::from(checkbox_text));
                }

                // Add the message data cells with proper theming
                cells.push(
                    Cell::from(msg.sequence.to_string())
                        .style(RatatuiStyle::default().fg(ThemeManager::message_sequence())),
                );
                cells.push(
                    Cell::from(msg.id.to_string())
                        .style(RatatuiStyle::default().fg(ThemeManager::message_id())),
                );
                cells.push(
                    Cell::from(msg.enqueued_at.to_string())
                        .style(RatatuiStyle::default().fg(ThemeManager::message_timestamp())),
                );
                cells.push(
                    Cell::from(get_state_display(&msg.state))
                        .style(RatatuiStyle::default().fg(get_state_color(&msg.state))),
                );

                let delivery_width = widths[if bulk_mode { 5 } else { 4 }];
                cells.push(
                    Cell::from(format_delivery_count_responsive(
                        msg.delivery_count,
                        delivery_width as usize,
                        narrow_layout,
                    ))
                    .style(RatatuiStyle::default().fg(ThemeManager::message_delivery_count())),
                );

                let mut row = Row::new(cells);

                // Apply selection highlighting
                if i == selected_index {
                    row = row.style(
                        RatatuiStyle::default()
                            .bg(ThemeManager::selection_bg())
                            .fg(ThemeManager::selection_fg()),
                    );
                }

                rows.push(row);
            }
        }

        // Create the table headers with proper theming
        let header_cells: Vec<Cell> = self
            .headers
            .iter()
            .map(|h| {
                Cell::from(h.as_str()).style(
                    RatatuiStyle::default()
                        .fg(ThemeManager::header_accent()) // Always yellow to match line numbers
                        .add_modifier(Modifier::BOLD),
                )
            })
            .collect();

        let header = Row::new(header_cells).height(1);

        // Create the table widget with proper theming
        let table = RatatuiTable::new(
            rows,
            &self
                .widths
                .iter()
                .map(|&w| Constraint::Length(w))
                .collect::<Vec<_>>(),
        )
        .header(header)
        .block(
            Block::default()
                .borders(RatatuiBorders::ALL)
                .border_type(RatatuiBorderType::Rounded)
                .border_style(RatatuiStyle::default().fg(if self.is_focused {
                    ThemeManager::primary_accent() // Teal when focused
                } else {
                    RatatuiColor::White // White when not focused
                }))
                .title(self.title.as_str())
                .title_alignment(RatatuiAlignment::Center)
                .title_style(
                    RatatuiStyle::default()
                        .fg(ThemeManager::title_accent()) // Use pink to match message details title
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .column_spacing(2)
        .row_highlight_style(
            RatatuiStyle::default()
                .bg(ThemeManager::selection_bg())
                .fg(ThemeManager::selection_fg())
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("► ");

        // Create table state for selection
        let mut table_state = TableState::default();
        table_state.select(Some(selected_index));

        // Render the table
        frame.render_stateful_widget(table, area, &mut table_state);

        // Create status bar overlay at the bottom with pagination info
        if let Some(ref info) = self.pagination_info {
            let status_text = format_pagination_status(info);
            let status_bar = Paragraph::new(status_text)
                .style(
                    Style::default().fg(if self.is_focused {
                        ThemeManager::primary_accent() // Teal text when focused
                    } else {
                        RatatuiColor::White // White text when not focused
                    }), // No background - clean and transparent
                )
                .alignment(RatatuiAlignment::Center);

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
                    let target_position = position as usize;
                    let max_index = self.get_message_count().saturating_sub(1);

                    // Ensure target position is within bounds
                    let bounded_position = target_position.min(max_index);

                    // Reset to beginning first
                    let current = self.get_current_index();
                    for _ in 0..current {
                        self.move_up();
                    }

                    // Move to target position using bounds-checked movement
                    for _ in 0..bounded_position {
                        self.move_down();
                    }

                    log::debug!(
                        "Moved cursor to position: {} (requested: {})",
                        bounded_position,
                        target_position
                    );
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
