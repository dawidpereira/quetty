use super::content::{HelpContent, HelpSection, Shortcut};
use crate::theme::ThemeManager;
use tuirealm::ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph as RatatuiParagraph},
};

/// Utility for rendering help content with consistent styling
pub struct HelpRenderer {
    key_width: usize,
}

impl HelpRenderer {
    pub fn new() -> Self {
        Self { key_width: 20 }
    }

    /// Render the complete help content split into two columns
    pub fn render_help_content<'a>(&self, content: &'a HelpContent) -> (Text<'a>, Text<'a>) {
        let sections = &content.sections;
        let mid_point = sections.len().div_ceil(2);

        let left_sections = &sections[0..mid_point];
        let right_sections = &sections[mid_point..];

        let left_content = self.render_sections(left_sections);
        let right_content = self.render_sections(right_sections);

        (left_content, right_content)
    }

    /// Render header content (instructions and warnings)
    pub fn render_header<'a>(&self, content: &'a HelpContent) -> Text<'a> {
        Text::from(vec![
            Line::from(vec![Span::styled(
                &content.header_message,
                Style::default()
                    .fg(ThemeManager::shortcut_description())
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                &content.warning_message,
                Style::default().fg(ThemeManager::status_warning()),
            )]),
        ])
    }

    /// Render multiple sections into a single Text
    fn render_sections<'a>(&self, sections: &'a [HelpSection]) -> Text<'a> {
        let mut lines = Vec::new();

        for section in sections {
            // Handle note section specially
            if section.title == "Note" {
                lines.push(Line::from(vec![Span::styled(
                    "Note: Operations work on selected messages in bulk mode,",
                    Style::default().fg(ThemeManager::status_info()),
                )]));
                lines.push(Line::from(vec![Span::styled(
                    "      or on current message when no selections exist.",
                    Style::default().fg(ThemeManager::status_info()),
                )]));
                lines.push(Line::from(""));
                continue;
            }

            // Add section title (skip if empty)
            if !section.title.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    format!("{} {}", section.icon, section.title),
                    Style::default()
                        .fg(ThemeManager::help_section_title())
                        .add_modifier(Modifier::BOLD),
                )]));
                lines.push(Line::from(""));
            }

            // Add shortcuts for this section
            for shortcut in &section.shortcuts {
                lines.push(self.render_shortcut(shortcut));
            }

            // Add spacing between sections
            if !section.title.is_empty() {
                lines.push(Line::from(""));
            }
        }

        Text::from(lines)
    }

    /// Render a single shortcut with consistent formatting
    fn render_shortcut<'a>(&self, shortcut: &'a Shortcut) -> Line<'a> {
        let key_text = if shortcut.keys.len() == 1 {
            shortcut.keys[0].clone()
        } else {
            shortcut.keys.join(" ")
        };

        // Handle special case for note lines (no description)
        if shortcut.description.is_empty() {
            return Line::from(vec![Span::styled(
                key_text,
                Style::default().fg(ThemeManager::status_info()),
            )]);
        }

        Line::from(vec![
            Span::styled(
                format!("  {:width$}", key_text, width = self.key_width),
                Style::default()
                    .fg(ThemeManager::shortcut_key())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &shortcut.description,
                Style::default().fg(ThemeManager::shortcut_description()),
            ),
        ])
    }

    /// Create a styled paragraph with the given text and alignment
    pub fn create_paragraph<'a>(
        &self,
        text: Text<'a>,
        alignment: Alignment,
    ) -> RatatuiParagraph<'a> {
        RatatuiParagraph::new(text)
            .alignment(alignment)
            .block(Block::default())
    }

    /// Split area into header and content sections
    pub fn layout_help_screen(&self, area: Rect) -> (Rect, Rect, Rect) {
        let chunks = Layout::default()
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .margin(1)
            .split(area);

        let columns = Layout::default()
            .direction(tuirealm::ratatui::layout::Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);

        (chunks[0], columns[0], columns[1])
    }
}

impl Default for HelpRenderer {
    fn default() -> Self {
        Self::new()
    }
}
