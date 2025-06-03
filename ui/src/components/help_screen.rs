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

use crate::components::common::Msg;

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

        // Header section
        let header_text = Text::from(vec![
            Line::from(vec![Span::styled(
                "Quetty - Azure Service Bus Queue Manager",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(Span::raw("Press [Esc] or [h] to close this help screen")),
            Line::from(vec![Span::styled(
                "⚠️  DLQ operations are in development - use with caution",
                Style::default().fg(Color::Red),
            )]),
        ]);

        let header_para = RatatuiParagraph::new(header_text)
            .alignment(tuirealm::ratatui::layout::Alignment::Center)
            .block(Block::default());

        // Help content with organized sections
        let help_content = Text::from(vec![
            // Global Actions
            Line::from(vec![Span::styled(
                "🌐 GLOBAL ACTIONS",
                Style::default().fg(Color::Green),
            )]),
            Line::from(""),
            Line::from("  [q]              Quit application"),
            Line::from("  [h]              Toggle this help screen"),
            Line::from("  [Esc]            Go back / Cancel operation"),
            Line::from(""),
            // Navigation
            Line::from(vec![Span::styled(
                "🧭 NAVIGATION",
                Style::default().fg(Color::Green),
            )]),
            Line::from(""),
            Line::from("  [↑] [k]          Move up"),
            Line::from("  [↓] [j]          Move down"),
            Line::from("  [Enter] [o]      Select / Open item"),
            Line::from("  [PgUp] [PgDn]    Scroll page up/down"),
            Line::from(""),
            // Queue & Message Management
            Line::from(vec![Span::styled(
                "📋 QUEUE & MESSAGE MANAGEMENT",
                Style::default().fg(Color::Green),
            )]),
            Line::from(""),
            Line::from("  [n] ']'          Next page"),
            Line::from("  [p] '['         Previous page"),
            Line::from("  [d]              Toggle between Main ↔ Dead Letter Queue"),
            Line::from("  [Enter]          View message details"),
            Line::from(""),
            // Bulk Selection
            Line::from(vec![Span::styled(
                "📦 BULK SELECTION MODE",
                Style::default().fg(Color::Green),
            )]),
            Line::from(""),
            Line::from("  [Space]          Toggle selection for current message"),
            Line::from("  [Ctrl+A]         Select all messages on current page"),
            Line::from("  [Ctrl+Shift+A]   Select all loaded messages (all pages)"),
            Line::from("  [Esc]            Clear selections / Exit bulk mode"),
            Line::from(""),
            // Message Operations
            Line::from(vec![Span::styled(
                "⚡ MESSAGE OPERATIONS",
                Style::default().fg(Color::Green),
            )]),
            Line::from(""),
            Line::from("  [Delete] [Ctrl+X] Delete message(s) with confirmation"),
            Line::from("  [Ctrl+D]         Send message(s) to DLQ (⚠️ DEV)"),
            Line::from("  [r]              Resend from DLQ to main queue (⚠️ DEV)"),
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
            Line::from("  [↑] [↓] [k] [j]  Scroll content up/down"),
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
                code: Key::Char('h'),
                modifiers: KeyModifiers::NONE,
            }) => Some(Msg::ToggleHelpScreen),
            _ => None,
        }
    }
}
