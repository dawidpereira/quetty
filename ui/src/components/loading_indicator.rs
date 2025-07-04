use crate::components::common::{LoadingActivityMsg, Msg};
use crate::components::state::ComponentState;
use crate::config;
use crate::theme::ThemeManager;
use std::time::{Duration, Instant};
use tui_realm_stdlib::Label;
use tuirealm::{
    Component, Event, MockComponent,
    event::{Key, KeyEvent, NoUserEvent},
    props::{Alignment, AttrValue, Attribute},
};

// Simple animation frames for loading indicator
const SPINNER_FRAMES: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

#[derive(MockComponent)]
pub struct LoadingIndicator {
    component: Label,
    message: String,
    progress_message: Option<String>,
    frame_index: usize,
    last_frame_time: Instant,
    is_mounted: bool,
    show_cancel_button: bool,
    operation_id: Option<String>,
}

impl LoadingIndicator {
    pub fn new(message: &str, _indeterminate: bool) -> Self {
        let mut component = Label::default();

        // Set the initial text with animation frame
        let display_text = format!("{} {}", SPINNER_FRAMES[0], message);
        component.attr(Attribute::Text, AttrValue::String(display_text));

        // Set text color
        component.attr(
            Attribute::Foreground,
            AttrValue::Color(ThemeManager::status_loading()),
        );

        // Set text alignment to center
        component.attr(
            Attribute::Alignment,
            AttrValue::Alignment(Alignment::Center),
        );

        // Don't set TextModifiers/TextProps as they're causing issues

        log::debug!("Created new LoadingIndicator with message: {message}");

        Self {
            component,
            message: message.to_string(),
            progress_message: None,
            frame_index: 0,
            last_frame_time: Instant::now(),
            is_mounted: false,
            show_cancel_button: false,
            operation_id: None,
        }
    }

    /// Update progress message
    pub fn update_progress(&mut self, progress: String) {
        self.progress_message = Some(progress);
        self.update_display_text();
    }

    /// Show cancel button for operation
    pub fn show_cancel_button(&mut self, operation_id: String) {
        self.show_cancel_button = true;
        self.operation_id = Some(operation_id);
        self.update_display_text();
    }

    // Update the animation frame
    fn update_animation(&mut self) {
        let now = Instant::now();
        let frame_duration = Duration::from_millis(
            config::get_config_or_panic()
                .ui()
                .loading_frame_duration_ms(),
        );
        if now.duration_since(self.last_frame_time) >= frame_duration {
            // Move to next frame
            self.frame_index = (self.frame_index + 1) % SPINNER_FRAMES.len();
            self.last_frame_time = now;
            self.update_display_text();
        }
    }

    /// Update the display text with current state
    fn update_display_text(&mut self) {
        let spinner = SPINNER_FRAMES[self.frame_index];

        let display_text = if self.show_cancel_button {
            // Clean format with empty line between message and cancel instruction
            format!("{} {}\n\nPress 'c' to cancel", spinner, self.message)
        } else if let Some(ref progress) = self.progress_message {
            // Show progress only when no cancel button (for non-cancellable operations)
            format!("{} {} • {}", spinner, self.message, progress)
        } else {
            format!("{} {}", spinner, self.message)
        };

        self.component
            .attr(Attribute::Text, AttrValue::String(display_text));
    }
}

impl Component<Msg, NoUserEvent> for LoadingIndicator {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        match ev {
            Event::Tick => {
                // Update animation on tick
                self.update_animation();
                Some(Msg::ForceRedraw)
            }
            Event::Keyboard(KeyEvent {
                code: Key::Char('c'),
                ..
            }) if self.show_cancel_button => {
                log::info!("User requested cancellation via 'c' key");
                Some(Msg::LoadingActivity(LoadingActivityMsg::Cancel))
            }
            _ => None,
        }
    }
}

impl ComponentState for LoadingIndicator {
    fn mount(&mut self) -> crate::error::AppResult<()> {
        log::debug!("Mounting LoadingIndicator component");

        if self.is_mounted {
            log::warn!("LoadingIndicator is already mounted");
            return Ok(());
        }

        // Reset animation state
        self.frame_index = 0;
        self.last_frame_time = Instant::now();

        // Update initial display
        self.update_display_text();

        self.is_mounted = true;
        log::debug!("LoadingIndicator component mounted successfully");
        Ok(())
    }
}

impl Drop for LoadingIndicator {
    fn drop(&mut self) {
        log::debug!("Dropping LoadingIndicator component");
        self.is_mounted = false;
        log::debug!("LoadingIndicator component dropped");
    }
}
