use std::time::{Duration, Instant};
use tui_realm_stdlib::Label;
use tuirealm::{
    Component, Event, MockComponent,
    event::NoUserEvent,
    props::{Alignment, AttrValue, Attribute, Color},
};

use crate::components::common::Msg;

// Simple animation frames for loading indicator
const SPINNER_FRAMES: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
const FRAME_DURATION: Duration = Duration::from_millis(100);

#[derive(MockComponent)]
pub struct LoadingIndicator {
    component: Label,
    message: String,
    frame_index: usize,
    last_frame_time: Instant,
}

impl LoadingIndicator {
    pub fn new(message: &str, _indeterminate: bool) -> Self {
        let mut component = Label::default();

        // Set the initial text with animation frame
        let display_text = format!("{} {}", SPINNER_FRAMES[0], message);
        component.attr(Attribute::Text, AttrValue::String(display_text));

        // Set text color
        component.attr(Attribute::Foreground, AttrValue::Color(Color::LightBlue));

        // Set text alignment to center
        component.attr(
            Attribute::Alignment,
            AttrValue::Alignment(Alignment::Center),
        );

        // Don't set TextModifiers/TextProps as they're causing issues

        log::debug!("Created new LoadingIndicator with message: {}", message);

        Self {
            component,
            message: message.to_string(),
            frame_index: 0,
            last_frame_time: Instant::now(),
        }
    }

    // Update the animation frame
    fn update_animation(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_frame_time) >= FRAME_DURATION {
            // Move to next frame
            self.frame_index = (self.frame_index + 1) % SPINNER_FRAMES.len();
            self.last_frame_time = now;

            // Update the text with new animation frame
            let display_text = format!("{} {}", SPINNER_FRAMES[self.frame_index], self.message);
            self.component
                .attr(Attribute::Text, AttrValue::String(display_text));


        }
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
            _ => None,
        }
    }
}
