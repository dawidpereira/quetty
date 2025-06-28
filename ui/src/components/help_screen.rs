use crate::components::base_popup::PopupBuilder;
use crate::components::common::Msg;
use crate::components::help::{HelpContent, HelpRenderer};
use crate::config;
use tuirealm::{
    Component, Event, Frame, MockComponent, NoUserEvent,
    event::{Key, KeyEvent},
    ratatui::layout::{Alignment, Rect},
};

/// Help screen component that displays keyboard shortcuts and usage information.
///
/// This component provides a full-screen help interface using the PopupBuilder
/// pattern for consistent styling and theming.
///
/// # Usage
///
/// ```rust
/// use quetty::components::help_screen::HelpScreen;
///
/// let help_screen = HelpScreen::new();
/// ```
///
/// # Events
///
/// - `KeyEvent::Esc` - Closes the help screen and returns to main interface
///
/// # Features
///
/// - **Responsive layout** - Automatically adjusts to terminal size
/// - **Two-column display** - Organized shortcuts grouped by category
/// - **Consistent theming** - Uses PopupBuilder for standardized appearance
/// - **Configuration-driven** - Shortcuts automatically reflect user's key bindings
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
        // Use PopupBuilder for consistent styling
        let popup_content = PopupBuilder::new("Help Screen");

        // Get help content from configuration
        let keys = config::get_config_or_panic().keys();
        let help_content = HelpContent::from_config(keys);

        // Layout the screen areas
        let (header_area, left_area, right_area) = self.renderer.layout_help_screen(area);

        // Render the popup block using PopupBuilder
        let block = popup_content.create_block_with_title("  📖 Keyboard Shortcuts Help  ");

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
