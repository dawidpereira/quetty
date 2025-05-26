use tuirealm::event::{Key, KeyEvent, KeyModifiers};
use tuirealm::props::{Alignment, BorderType, Color, Style};
use tuirealm::ratatui::layout::{Constraint, Layout, Rect};
use tuirealm::ratatui::text::{Line, Span, Text};
use tuirealm::ratatui::widgets::{Block, Paragraph, Row, Table};
use tuirealm::{Component, Event, Frame, MockComponent, NoUserEvent};

use super::common::Msg;

pub struct HelpScreen {}

impl HelpScreen {
    pub fn new() -> Self {
        Self {}
    }
}

impl MockComponent for HelpScreen {
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Create a layout with a table of keyboard shortcuts
        let block = Block::default()
            .title("Keyboard Shortcuts Help")
            .borders(tuirealm::ratatui::widgets::Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan));

        // Define the layout for the help content
        let chunks = Layout::default()
            .constraints([Constraint::Length(3), Constraint::Min(5)])
            .margin(1)
            .split(area);

        // Create a header section with general info
        let header_text = Text::from(vec![
            Line::from(vec![Span::styled(
                "Quetty Help",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(Span::raw("Press Esc or h to close this help screen")),
            Line::from(vec![Span::styled(
                "⚠️ DLQ message sending is in development - not for production use",
                Style::default().fg(Color::Red),
            )]),
        ]);

        let header_para = Paragraph::new(header_text)
            .alignment(Alignment::Center)
            .block(Block::default());

        // Define the keyboard shortcut data
        let shortcuts = vec![
            // Global shortcuts
            vec!["Global", "q", "Quit application"],
            vec!["Global", "h", "Toggle help screen"],
            // Navigation
            vec!["Navigation", "↑/k", "Move up"],
            vec!["Navigation", "↓/j", "Move down"],
            vec!["Navigation", "Enter/o", "Select item"],
            vec!["Navigation", "Esc", "Go back/cancel"],
            // Message list
            vec!["Messages", "PgUp/PgDown", "Scroll list"],
            vec!["Messages", "n/]", "Next page"],
            vec!["Messages", "p/[", "Previous page"],
            vec!["Messages", "d", "Toggle Dead Letter Queue"],
            vec![
                "Messages",
                "Ctrl+d",
                "Send message to DLQ (DEV - with confirmation)",
            ],
            vec!["Messages", "Enter", "View message details"],
            // Message details
            vec!["Details", "←/→", "Move cursor"],
            vec!["Details", "PgUp/PgDown", "Scroll content"],
        ];

        // Create rows for the table
        let rows = shortcuts.iter().map(|s| {
            let cells = s.iter().map(|c| Span::raw(*c));
            Row::new(cells)
        });

        // Create header row
        let header_cells = ["Context", "Key", "Description"]
            .iter()
            .map(|h| Span::styled(*h, Style::default().fg(Color::Yellow)));
        let header_row = Row::new(header_cells);

        // Create the table
        let table = Table::new(
            rows,
            [
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(60),
            ],
        )
        .header(header_row)
        .block(Block::default())
        .style(Style::default());

        // Render the components
        frame.render_widget(block, area);
        frame.render_widget(header_para, chunks[0]);
        frame.render_widget(table, chunks[1]);
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
