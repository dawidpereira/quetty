use crate::components::base_popup::PopupBuilder;
use crate::components::common::{Msg, QueueType};
use crate::components::messages::rendering::{
    build_table_from_messages, calculate_responsive_layout, format_delivery_count_responsive,
    get_state_color, get_state_display,
};
use crate::components::messages::selection::{format_pagination_status, format_queue_display};
use crate::theme::ThemeManager;
use quetty_server::bulk_operations::MessageIdentifier;
use quetty_server::model::MessageModel;
use tui_realm_stdlib::Table;
use tuirealm::command::{Cmd, CmdResult};
use tuirealm::event::{Key, KeyEvent};
use tuirealm::props::{Alignment, BorderType, Borders, Color, Style};
use tuirealm::ratatui::layout::{Alignment as RatatuiAlignment, Constraint, Rect};
use tuirealm::ratatui::style::{Color as RatatuiColor, Modifier, Style as RatatuiStyle};
use tuirealm::ratatui::widgets::{Cell, Paragraph, Row, Table as RatatuiTable, TableState};
use tuirealm::{
    AttrValue, Attribute, Component, Event, Frame, MockComponent, NoUserEvent, State, StateValue,
};

/// Information about message list pagination and queue state.
///
/// Contains all the necessary information for displaying pagination controls,
/// bulk selection status, and queue statistics in the message list component.
#[derive(Debug, Clone)]
pub struct PaginationInfo {
    /// Current page number (0-based)
    pub current_page: usize,
    /// Total number of pages loaded so far
    pub total_pages_loaded: usize,
    /// Total number of messages loaded
    pub total_messages_loaded: usize,
    /// Number of messages per page
    pub current_page_size: usize,
    /// Whether there are more pages available
    pub has_next_page: bool,
    /// Whether there are previous pages available
    pub has_previous_page: bool,
    /// Name of the current queue
    pub queue_name: Option<String>,
    /// Type of the current queue (Main or DeadLetter)
    pub queue_type: QueueType,
    /// Whether bulk selection mode is active
    pub bulk_mode: bool,
    /// Number of messages currently selected
    pub selected_count: usize,
    /// Total number of messages in the queue (if available)
    pub queue_total_messages: Option<u64>,
    /// Age of queue statistics in seconds (if available)
    pub queue_stats_age_seconds: Option<i64>,
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

pub const CMD_RESULT_MESSAGE_SELECTED: &str = "MessageSelected";
pub const CMD_RESULT_MESSAGE_PREVIEW: &str = "MessagePreview";
pub const CMD_RESULT_QUEUE_UNSELECTED: &str = "QueueUnSelected";

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
            format!(" {queue_display} ")
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
                .headers(headers.iter().map(|s| s.as_str()).collect::<Vec<_>>())
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
    pub fn current_index(&self) -> usize {
        get_current_index_from_state(&self.component.state())
    }

    /// Get the number of messages currently available
    pub fn message_count(&self) -> usize {
        self.messages.as_ref().map_or(0, |msgs| msgs.len())
    }

    /// Get pagination info for external access
    pub fn pagination_info(&self) -> &Option<PaginationInfo> {
        &self.pagination_info
    }

    /// Get component for internal operations (used by navigation module)
    pub(super) fn component_mut(&mut self) -> &mut Table {
        &mut self.component
    }

    /// Get component for read-only access
    pub(super) fn component(&self) -> &Table {
        &self.component
    }

    /// Get messages for internal operations
    pub(super) fn messages(&self) -> &Option<Vec<MessageModel>> {
        &self.messages
    }

    /// Get selected messages for internal operations
    pub(super) fn selected_messages(&self) -> &Vec<MessageIdentifier> {
        &self.selected_messages
    }

    /// Get headers for internal operations
    pub(super) fn headers(&self) -> &Vec<String> {
        &self.headers
    }

    /// Set headers for internal operations
    pub(super) fn set_headers(&mut self, headers: Vec<String>) {
        self.headers = headers;
    }

    /// Get widths for internal operations
    pub(super) fn widths(&self) -> &Vec<u16> {
        &self.widths
    }

    /// Set widths for internal operations
    pub(super) fn set_widths(&mut self, widths: Vec<u16>) {
        self.widths = widths;
    }

    /// Get title for internal operations
    pub(super) fn title(&self) -> &String {
        &self.title
    }

    /// Get focus state for internal operations
    pub(super) fn is_focused(&self) -> bool {
        self.is_focused
    }

    /// Set narrow layout flag for internal operations
    pub(super) fn set_narrow_layout(&mut self, narrow_layout: bool) {
        self.narrow_layout = narrow_layout;
    }
}

impl Component<Msg, NoUserEvent> for Messages {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => {
                // If no messages are selected, and we are in bulk mode, exit bulk mode
                if self
                    .pagination_info
                    .as_ref()
                    .is_some_and(|info| info.bulk_mode)
                    && self.selected_messages.is_empty()
                {
                    return Some(Msg::MessageActivity(
                        crate::components::common::MessageActivityMsg::ClearAllSelections,
                    ));
                }
                // If no messages are selected, and we are not in bulk mode, exit queue
                if self.selected_messages.is_empty() {
                    return Some(Msg::QueueActivity(
                        crate::components::common::QueueActivityMsg::ExitQueueConfirmation,
                    ));
                }
                // Otherwise, delegate to event handling module
                super::event_handling::handle_event(self, ev)
            }
            _ => super::event_handling::handle_event(self, ev),
        }
    }
}

impl MockComponent for Messages {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Recalculate responsive layout based on actual available width
        let bulk_mode = self
            .pagination_info()
            .as_ref()
            .is_some_and(|info| info.bulk_mode);

        let (headers, widths, narrow_layout) = calculate_responsive_layout(area.width, bulk_mode);

        // Update stored layout info
        self.set_headers(headers);
        self.set_widths(widths.clone());
        self.set_narrow_layout(narrow_layout);

        // Get the current selection from the internal component
        let table_state = self.component().state();
        let selected_index = get_current_index_from_state(&table_state);

        let mut rows = Vec::new();
        if let Some(messages) = self.messages() {
            for (i, msg) in messages.iter().enumerate() {
                let mut cells = Vec::new();

                if bulk_mode {
                    // Add checkbox column in bulk mode
                    let message_id = MessageIdentifier::from_message(msg);
                    let checkbox_text = if self.selected_messages().contains(&message_id) {
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
            .headers()
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
                .widths()
                .iter()
                .map(|&w| Constraint::Length(w))
                .collect::<Vec<_>>(),
        )
        .header(header)
        .block(
            // Use PopupBuilder for consistent styling with conditional focus state
            PopupBuilder::new("Messages Table").create_conditional_block(
                self.title().as_str(),
                self.is_focused(),
                ThemeManager::primary_accent(), // Teal when focused
                RatatuiColor::White,            // White when not focused
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
        if let Some(info) = self.pagination_info() {
            let status_text = format_pagination_status(info);
            let status_bar = Paragraph::new(status_text)
                .style(
                    Style::default().fg(if self.is_focused() {
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
        self.component().query(attr)
    }

    fn attr(&mut self, attr: Attribute, value: AttrValue) {
        match attr {
            Attribute::Custom("cursor_position") => {
                if let AttrValue::Number(position) = value {
                    log::debug!("Received cursor position attribute: {position}");
                    let target_position = position as usize;
                    let max_index = self.message_count().saturating_sub(1);

                    // Ensure target position is within bounds
                    let bounded_position = target_position.min(max_index);

                    // Reset to beginning first
                    let current = self.current_index();
                    for _ in 0..current {
                        self.move_up();
                    }

                    // Move to target position using bounds-checked movement
                    for _ in 0..bounded_position {
                        self.move_down();
                    }

                    log::debug!(
                        "Moved cursor to position: {bounded_position} (requested: {target_position})"
                    );
                }
            }
            _ => {
                self.component_mut().attr(attr, value);
            }
        }
    }

    fn state(&self) -> State {
        self.component().state()
    }

    fn perform(&mut self, cmd: Cmd) -> CmdResult {
        self.component_mut().perform(cmd)
    }
}
