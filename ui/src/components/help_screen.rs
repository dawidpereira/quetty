use crate::components::common::Msg;
use crate::config;
use tuirealm::{
    Component, Event, Frame, MockComponent, NoUserEvent,
    event::{Key, KeyEvent, KeyModifiers},
    props::{BorderType, Color},
    ratatui::{
        layout::{Constraint, Layout, Rect},
        style::Style,
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
            .title("📖 Keyboard Shortcuts Help")
            .borders(tuirealm::ratatui::widgets::Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan));

        // Create layout with header and scrollable content
        let chunks = Layout::default()
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .margin(1)
            .split(area);

        let keys = config::CONFIG.keys();
        let header_text = Text::from(vec![
            Line::from(vec![Span::styled(
                "Quetty - Azure Service Bus Queue Manager",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(Span::raw(format!(
                "Press [Esc] or [{}] to close this help screen",
                keys.help()
            ))),
            Line::from(vec![Span::styled(
                "⚠️  DLQ operations are in development - use with caution",
                Style::default().fg(Color::Red),
            )]),
        ]);

        let header_para = RatatuiParagraph::new(header_text)
            .alignment(tuirealm::ratatui::layout::Alignment::Center)
            .block(Block::default());

        // Help content with organized sections - using configured keys
        let help_content = Text::from(vec![
            // Global Actions
            Line::from(vec![Span::styled(
                "🌐 GLOBAL ACTIONS",
                Style::default().fg(Color::Green),
            )]),
            Line::from(""),
            Line::from(format!("  [{}]              Quit application", keys.quit())),
            Line::from(format!(
                "  [{}]              Toggle this help screen",
                keys.help()
            )),
            Line::from("  [Esc]            Go back / Cancel operation"),
            Line::from(""),
            // Navigation
            Line::from(vec![Span::styled(
                "🧭 NAVIGATION",
                Style::default().fg(Color::Green),
            )]),
            Line::from(""),
            Line::from(format!("  [↑] [{}]          Move up", keys.up())),
            Line::from(format!("  [↓] [{}]          Move down", keys.down())),
            Line::from(format!(
                "  [Enter] [{}]      Select / Open item",
                keys.queue_select()
            )),
            Line::from("  [PgUp] [PgDn]    Scroll page up/down"),
            Line::from(""),
            // Queue & Message Management
            Line::from(vec![Span::styled(
                "📋 QUEUE & MESSAGE MANAGEMENT",
                Style::default().fg(Color::Green),
            )]),
            Line::from(""),
            Line::from(format!(
                "  [{}] [{}]          Next page",
                keys.next_page(),
                keys.alt_next_page()
            )),
            Line::from(format!(
                "  [{}] [{}]         Previous page",
                keys.prev_page(),
                keys.alt_prev_page()
            )),
            Line::from("  [d]              Toggle between Main ↔ Dead Letter Queue"),
            Line::from("  [Enter]          View message details"),
            Line::from(""),
            // Bulk Selection
            Line::from(vec![Span::styled(
                "📦 BULK SELECTION MODE",
                Style::default().fg(Color::Green),
            )]),
            Line::from(""),
            Line::from(format!(
                "  [{}]          Toggle selection for current message",
                keys.toggle_selection()
            )),
            Line::from(format!(
                "  [Ctrl+{}]         Select all messages on current page",
                keys.select_all_page()
            )),
            Line::from("  [Ctrl+Shift+A]   Select all loaded messages (all pages)"),
            Line::from("  [Esc]            Clear selections / Exit bulk mode"),
            Line::from(""),
            // Message Operations
            Line::from(vec![Span::styled(
                "⚡ MESSAGE OPERATIONS",
                Style::default().fg(Color::Green),
            )]),
            Line::from(""),
            Line::from(format!(
                "  [{}] [Ctrl+{}]     Delete message(s) with confirmation",
                keys.delete_message(),
                keys.alt_delete_message()
            )),
            Line::from(format!(
                "  [{}] [Ctrl+{}]     Send message(s) to DLQ (⚠️ DEV)",
                keys.send_to_dlq(),
                keys.send_to_dlq()
            )),
            Line::from(format!(
                "  [{}]              Resend from DLQ to main queue (keep in DLQ)",
                keys.resend_from_dlq()
            )),
            Line::from(format!(
                "  [{}]              Resend and delete from DLQ (⚠️ DEV)",
                keys.resend_and_delete_from_dlq()
            )),
            Line::from(""),
            Line::from(vec![Span::styled(
                "💡 Note: Operations work on selected messages in bulk mode,",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(vec![Span::styled(
                "        or on current message when no selections exist.",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(""),
            // Message Details View
            Line::from(vec![Span::styled(
                "🔍 MESSAGE DETAILS VIEW",
                Style::default().fg(Color::Green),
            )]),
            Line::from(""),
            Line::from("  [←] [→]          Move cursor left/right"),
            Line::from(format!(
                "  [↑] [↓] [{}] [{}]  Scroll content up/down",
                keys.up(),
                keys.down()
            )),
            Line::from("  [PgUp] [PgDn]    Scroll content page up/down"),
            Line::from("  [Esc]            Return to message list"),
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
