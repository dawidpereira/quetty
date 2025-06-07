use crate::components::common::Msg;
use crate::config;
use crate::theme::ThemeManager;
use tuirealm::{
    Component, Event, Frame, MockComponent, NoUserEvent,
    event::{Key, KeyEvent, KeyModifiers},
    props::BorderType,
    ratatui::{
        layout::{Alignment, Constraint, Layout, Rect},
        style::{Modifier, Style},
        text::{Line, Span, Text},
        widgets::{Block, Paragraph as RatatuiParagraph},
    },
};

pub struct HelpScreen {}

impl HelpScreen {
    pub fn new() -> Self {
        Self {}
    }
}

impl MockComponent for HelpScreen {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("  📖 Keyboard Shortcuts Help  ")
            .borders(tuirealm::ratatui::widgets::Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(ThemeManager::primary_accent()))
            .title_style(Style::default().fg(ThemeManager::title_accent()));

        // Create layout with header and scrollable content
        let chunks = Layout::default()
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .margin(1)
            .split(area);

        let keys = config::CONFIG.keys();

        // Define all key combinations to calculate consistent width
        let key_combinations = vec![
            format!("[{}]", keys.quit()),
            format!("[{}]", keys.help()),
            format!("[{}]", keys.theme()),
            "[Esc]".to_string(),
            format!("[↑] [{}]", keys.up()),
            format!("[↓] [{}]", keys.down()),
            format!("[Enter] [{}]", keys.queue_select()),
            "[PgUp] [PgDn]".to_string(),
            format!("[{}] [{}]", keys.next_page(), keys.alt_next_page()),
            format!("[{}] [{}]", keys.prev_page(), keys.alt_prev_page()),
            "[d]".to_string(),
            "[Enter]".to_string(),
            format!("[{}]", keys.toggle_selection()),
            format!("[Ctrl+{}]", keys.select_all_page()),
            "[Ctrl+Shift+A]".to_string(),
            format!(
                "[{}] [Ctrl+{}]",
                keys.delete_message(),
                keys.alt_delete_message()
            ),
            format!("[{}] [Ctrl+{}]", keys.send_to_dlq(), keys.send_to_dlq()),
            format!("[{}]", keys.resend_from_dlq()),
            format!("[{}]", keys.resend_and_delete_from_dlq()),
            "[←] [→]".to_string(),
            format!("[↑] [↓] [{}] [{}]", keys.up(), keys.down()),
        ];

        // Find the maximum width needed for key combinations
        let max_key_width = key_combinations.iter().map(|k| k.len()).max().unwrap_or(20);
        let padding_width = max_key_width + 4; // Add some extra padding

        let header_text = Text::from(vec![
            Line::from(vec![Span::styled(
                format!("Press [Esc] or [{}] to close this help screen", keys.help()),
                Style::default()
                    .fg(ThemeManager::shortcut_description())
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                "⚠️  DLQ operations are in development - use with caution",
                Style::default().fg(ThemeManager::status_warning()),
            )]),
        ]);

        let header_para = RatatuiParagraph::new(header_text)
            .alignment(Alignment::Center)
            .block(Block::default());

        // Help content with organized sections - using configured keys
        let help_content = Text::from(vec![
            // Global Actions
            Line::from(vec![Span::styled(
                "🌐 GLOBAL ACTIONS",
                Style::default()
                    .fg(ThemeManager::help_section_title())
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!(
                        "  {:width$}",
                        format!("[{}]", keys.quit()),
                        width = padding_width
                    ),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Quit application",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!(
                        "  {:width$}",
                        format!("[{}]", keys.help()),
                        width = padding_width
                    ),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Toggle this help screen",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!(
                        "  {:width$}",
                        format!("[{}]", keys.theme()),
                        width = padding_width
                    ),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Open theme picker",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("  {:width$}", "[Esc]", width = padding_width),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Go back / Cancel operation",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(""),
            // Navigation
            Line::from(vec![Span::styled(
                "🧭 NAVIGATION",
                Style::default()
                    .fg(ThemeManager::help_section_title())
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!(
                        "  {:width$}",
                        format!("[↑] [{}]", keys.up()),
                        width = padding_width
                    ),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Move up",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!(
                        "  {:width$}",
                        format!("[↓] [{}]", keys.down()),
                        width = padding_width
                    ),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Move down",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!(
                        "  {:width$}",
                        format!("[Enter] [{}]", keys.queue_select()),
                        width = padding_width
                    ),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Select / Open item",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("  {:width$}", "[PgUp] [PgDn]", width = padding_width),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Scroll page up/down",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(""),
            // Queue & Message Management
            Line::from(vec![Span::styled(
                "📋 QUEUE & MESSAGE MANAGEMENT",
                Style::default()
                    .fg(ThemeManager::help_section_title())
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!(
                        "  {:width$}",
                        format!("[{}] [{}]", keys.next_page(), keys.alt_next_page()),
                        width = padding_width
                    ),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Next page",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!(
                        "  {:width$}",
                        format!("[{}] [{}]", keys.prev_page(), keys.alt_prev_page()),
                        width = padding_width
                    ),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Previous page",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("  {:width$}", "[d]", width = padding_width),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Toggle between Main ↔ Dead Letter Queue",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("  {:width$}", "[Enter]", width = padding_width),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "View message details",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(""),
            // Bulk Selection
            Line::from(vec![Span::styled(
                "📦 BULK SELECTION MODE",
                Style::default()
                    .fg(ThemeManager::help_section_title())
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!(
                        "  {:width$}",
                        format!("[{}]", keys.toggle_selection()),
                        width = padding_width
                    ),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Toggle selection for current message",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!(
                        "  {:width$}",
                        format!("[Ctrl+{}]", keys.select_all_page()),
                        width = padding_width
                    ),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Select all messages on current page",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("  {:width$}", "[Ctrl+Shift+A]", width = padding_width),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Select all loaded messages (all pages)",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("  {:width$}", "[Esc]", width = padding_width),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Clear selections / Exit bulk mode",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(""),
            // Message Operations
            Line::from(vec![Span::styled(
                "⚡ MESSAGE OPERATIONS",
                Style::default()
                    .fg(ThemeManager::help_section_title())
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!(
                        "  {:width$}",
                        format!(
                            "[{}] [Ctrl+{}]",
                            keys.delete_message(),
                            keys.alt_delete_message()
                        ),
                        width = padding_width
                    ),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Delete message(s) with confirmation",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!(
                        "  {:width$}",
                        format!("[{}] [Ctrl+{}]", keys.send_to_dlq(), keys.send_to_dlq()),
                        width = padding_width
                    ),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Send message(s) to DLQ (⚠️ DEV)",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!(
                        "  {:width$}",
                        format!("[{}]", keys.resend_from_dlq()),
                        width = padding_width
                    ),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Resend from DLQ to main queue (keep in DLQ)",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!(
                        "  {:width$}",
                        format!("[{}]", keys.resend_and_delete_from_dlq()),
                        width = padding_width
                    ),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Resend and delete from DLQ (⚠️ DEV)",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "💡 Note: Operations work on selected messages in bulk mode,",
                Style::default().fg(ThemeManager::status_info()),
            )]),
            Line::from(vec![Span::styled(
                "        or on current message when no selections exist.",
                Style::default().fg(ThemeManager::status_info()),
            )]),
            Line::from(""),
            // Message Details View
            Line::from(vec![Span::styled(
                "🔍 MESSAGE DETAILS VIEW",
                Style::default()
                    .fg(ThemeManager::help_section_title())
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!("  {:width$}", "[←] [→]", width = padding_width),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Move cursor left/right",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!(
                        "  {:width$}",
                        format!("[↑] [↓] [{}] [{}]", keys.up(), keys.down()),
                        width = padding_width
                    ),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Scroll content up/down",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("  {:width$}", "[PgUp] [PgDn]", width = padding_width),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Scroll content page up/down",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("  {:width$}", "[Esc]", width = padding_width),
                    Style::default()
                        .fg(ThemeManager::shortcut_key())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Return to message list",
                    Style::default().fg(ThemeManager::shortcut_description()),
                ),
            ]),
        ]);

        let content_para = RatatuiParagraph::new(help_content)
            .block(Block::default())
            .wrap(tuirealm::ratatui::widgets::Wrap { trim: true });

        // Render components
        frame.render_widget(block, area);
        frame.render_widget(header_para, chunks[0]);
        frame.render_widget(content_para, chunks[1]);
    }

    fn query(&self, _attr: tuirealm::Attribute) -> Option<tuirealm::AttrValue> {
        None
    }

    fn attr(&mut self, _attr: tuirealm::Attribute, _value: tuirealm::AttrValue) {}

    fn state(&self) -> tuirealm::State {
        tuirealm::State::None
    }

    fn perform(&mut self, _cmd: tuirealm::command::Cmd) -> tuirealm::command::CmdResult {
        tuirealm::command::CmdResult::None
    }
}

impl Component<Msg, NoUserEvent> for HelpScreen {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        match ev {
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => Some(Msg::ToggleHelpScreen),
            Event::Keyboard(KeyEvent {
                code: Key::Char(c),
                modifiers: KeyModifiers::NONE,
            }) => {
                let keys = config::CONFIG.keys();
                if c == keys.help() {
                    Some(Msg::ToggleHelpScreen)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
