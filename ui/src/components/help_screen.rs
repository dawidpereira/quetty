use crate::components::common::Msg;
use crate::components::help::{HelpContent, HelpRenderer};
use crate::config;
use crate::theme::ThemeManager;
use tuirealm::{
    Component, Event, Frame, MockComponent, NoUserEvent,
    event::{Key, KeyEvent},
    props::BorderType,
    ratatui::{
        layout::{Alignment, Rect},
        style::Style,
        widgets::Block,
    },
};

pub struct HelpScreen {
    renderer: HelpRenderer,
}

impl HelpScreen {
    pub fn new() -> Self {
        Self {
            renderer: HelpRenderer::new(),
        }
    }
}

impl Default for HelpScreen {
    fn default() -> Self {
        Self::new()
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

        // Get help content from configuration
        let keys = config::get_config_or_panic().keys();
        let help_content = HelpContent::from_config(keys);

        // Layout the screen areas
        let (header_area, left_area, right_area) = self.renderer.layout_help_screen(area);

        // Render header with instructions and warnings
        let header_text = self.renderer.render_header(&help_content);
        let header_para = self
            .renderer
            .create_paragraph(header_text, Alignment::Center);

        // Render help content split into two columns
        let (left_content, right_content) = self.renderer.render_help_content(&help_content);
        let left_para = self
            .renderer
            .create_paragraph(left_content, Alignment::Left);
        let right_para = self
            .renderer
            .create_paragraph(right_content, Alignment::Left);

        // Render all components
        frame.render_widget(block, area);
        frame.render_widget(header_para, header_area);
        frame.render_widget(left_para, left_area);
        frame.render_widget(right_para, right_area);
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
            _ => None,
        }
    }
}
