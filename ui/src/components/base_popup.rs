use crate::theme::ThemeManager;
use tuirealm::ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

/// Common popup styling and layout patterns
pub struct PopupStyle {
    pub border_color: Color,
    pub title_color: Color,
    pub text_color: Color,
    pub muted_color: Color,
}

impl Default for PopupStyle {
    fn default() -> Self {
        Self {
            border_color: ThemeManager::primary_accent(),
            title_color: ThemeManager::title_accent(),
            text_color: ThemeManager::text_primary(),
            muted_color: ThemeManager::text_muted(),
        }
    }
}

impl PopupStyle {
    pub fn error() -> Self {
        Self {
            border_color: ThemeManager::status_error(),
            title_color: ThemeManager::status_error(),
            ..Self::default()
        }
    }

    pub fn success() -> Self {
        Self {
            border_color: ThemeManager::status_success(),
            title_color: ThemeManager::status_success(),
            ..Self::default()
        }
    }
}

/// Base popup builder for consistent popup creation
pub struct PopupBuilder {
    title: String,
    style: PopupStyle,
    content_lines: Vec<Line<'static>>,
    instructions: Option<String>,
}

impl PopupBuilder {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            style: PopupStyle::default(),
            content_lines: Vec::new(),
            instructions: None,
        }
    }

    pub fn error(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            style: PopupStyle::error(),
            content_lines: Vec::new(),
            instructions: None,
        }
    }

    pub fn success(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            style: PopupStyle::success(),
            content_lines: Vec::new(),
            instructions: None,
        }
    }

    pub fn add_text(mut self, text: impl Into<String>) -> Self {
        self.content_lines.push(Line::from(text.into()));
        self
    }

    pub fn add_error_text(mut self, text: impl Into<String>) -> Self {
        self.content_lines.push(Line::from(Span::styled(
            text.into(),
            Style::default().fg(ThemeManager::status_error()),
        )));
        self
    }

    pub fn add_empty_line(mut self) -> Self {
        self.content_lines.push(Line::from(""));
        self
    }

    pub fn add_line(mut self, spans: Vec<Span<'static>>) -> Self {
        self.content_lines.push(Line::from(spans));
        self
    }

    pub fn add_multiline_text(mut self, text: impl Into<String>) -> Self {
        for line in text.into().lines() {
            self.content_lines.push(Line::from(line.to_string()));
        }
        // Add spacing after multiline text for better visual separation
        self.content_lines.push(Line::from(""));
        self
    }

    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Add colorful confirmation instructions with styled Yes/No buttons
    pub fn with_confirmation_instructions(mut self, yes_key: &str, no_key: &str) -> Self {
        // Create a line with colorful Yes/No buttons similar to number input popup
        let instruction_line = Line::from(vec![
            Span::styled(
                format!("[{}]", yes_key.to_uppercase()),
                Style::default()
                    .fg(ThemeManager::status_success())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Yes    "),
            Span::styled(
                format!("[{}]", no_key.to_uppercase()),
                Style::default()
                    .fg(ThemeManager::status_error())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" No"),
        ]);

        // Add more spacing before the buttons for better visual separation
        self.content_lines.push(Line::from(""));
        self.content_lines.push(Line::from(""));
        self.content_lines.push(instruction_line);
        self
    }

    /// Create a block widget with custom title (without content)
    /// Useful when you need popup styling with a different title
    pub fn create_block_with_title(self, title: impl Into<String>) -> Block<'static> {
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.style.border_color))
            .title(title.into())
            .title_alignment(Alignment::Center)
            .title_style(
                Style::default()
                    .fg(self.style.title_color)
                    .add_modifier(Modifier::BOLD),
            )
    }

    /// Create a block widget with conditional styling based on focus state
    /// Useful for table containers and other components that change appearance when focused
    pub fn create_conditional_block(
        self,
        title: impl Into<String>,
        is_focused: bool,
        focused_color: Color,
        unfocused_color: Color,
    ) -> Block<'static> {
        let border_color = if is_focused {
            focused_color
        } else {
            unfocused_color
        };

        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(title.into())
            .title_alignment(Alignment::Center)
            .title_style(
                Style::default()
                    .fg(self.style.title_color)
                    .add_modifier(Modifier::BOLD),
            )
    }

    pub fn render(self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.style.border_color))
            .title(format!(" {} ", self.title))
            .title_alignment(Alignment::Center)
            .title_style(
                Style::default()
                    .fg(self.style.title_color)
                    .add_modifier(Modifier::BOLD),
            );

        let mut all_lines = Vec::new();

        // Add more empty lines at top for better spacing
        all_lines.push(Line::from(""));
        all_lines.push(Line::from(""));

        // Add content lines
        all_lines.extend(self.content_lines);

        // Add instructions at bottom if provided
        if let Some(instructions) = self.instructions {
            all_lines.push(Line::from(""));
            all_lines.push(Line::from(Span::styled(
                instructions,
                Style::default().fg(self.style.muted_color),
            )));
        }

        let paragraph = Paragraph::new(all_lines)
            .block(block)
            .style(Style::default().fg(self.style.text_color))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, area);
    }
}

/// Common popup sizing utilities
pub struct PopupLayout;

impl PopupLayout {
    /// Calculate centered popup area with given percentage of screen
    pub fn centered(area: Rect, width_percent: u16, height_percent: u16) -> Rect {
        let popup_width = (area.width * width_percent) / 100;
        let popup_height = (area.height * height_percent) / 100;

        let x = (area.width.saturating_sub(popup_width)) / 2;
        let y = (area.height.saturating_sub(popup_height)) / 2;

        Rect {
            x: area.x + x,
            y: area.y + y,
            width: popup_width,
            height: popup_height,
        }
    }

    /// Calculate small popup (40% width, 30% height)
    pub fn small(area: Rect) -> Rect {
        Self::centered(area, 40, 30)
    }

    /// Calculate medium popup (60% width, 50% height)
    pub fn medium(area: Rect) -> Rect {
        Self::centered(area, 60, 50)
    }

    /// Calculate large popup (80% width, 70% height)
    pub fn large(area: Rect) -> Rect {
        Self::centered(area, 80, 70)
    }

    /// Calculate extra wide popup for confirmations (90% width, 60% height)
    pub fn extra_wide(area: Rect) -> Rect {
        Self::centered(area, 90, 60)
    }
}
