use std::time::Duration;
use copypasta::{ClipboardContext, ClipboardProvider};
use tuirealm::event::NoUserEvent;
use tuirealm::props::{Alignment, Color, TextModifiers};
use tuirealm::ratatui::layout::{Constraint, Direction, Layout};
use tuirealm::terminal::{CrosstermTerminalAdapter, TerminalAdapter, TerminalBridge};
use tuirealm::{
    Application, EventListenerCfg, Update,
};

use crate::components::common::{ComponentId, Msg};
use crate::components::label::Label;
use crate::components::messages::Messages;
use crate::components::message_details::MessageDetails;

pub struct Model<T>
where
    T: TerminalAdapter,
{
    /// Application
    pub app: Application<ComponentId, Msg, NoUserEvent>,
    /// Indicates that the application must quit
    pub quit: bool,
    /// Tells whether to redraw interface
    pub redraw: bool,
    /// Used to draw to terminal
    pub terminal: TerminalBridge<T>,
}

impl Default for Model<CrosstermTerminalAdapter> {
    fn default() -> Self {
        Self {
            app: Self::init_app(),
            quit: false,
            redraw: true,
            terminal: TerminalBridge::init_crossterm().expect("Cannot initialize terminal"),
        }
    }
}

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn view(&mut self) {
        assert!(self
            .terminal
            .draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints(
                        [
                            Constraint::Length(1),
                            Constraint::Length(1), // Label
                            Constraint::Length(2),
                            Constraint::Length(16),// Messages
                        ]
                        .as_ref(),
                    )
                    .split(f.area());

                self.app.view(&ComponentId::Label, f, chunks[1]);

                let main_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(1)
                    .constraints(
                        [
                            Constraint::Length(49),// Messages
                            Constraint::Length(49),
                        ]
                        .as_ref(),
                    )
                    .split(chunks[3]);

                self.app.view(&ComponentId::Messages, f, main_chunks[0]);
                self.app.view(&ComponentId::MessageDetails, f, main_chunks[1]);
            })
            .is_ok());
    }

    fn init_app() -> Application<ComponentId, Msg, NoUserEvent> {
        // Setup application
        let mut app: Application<ComponentId, Msg, NoUserEvent> = Application::init(
            EventListenerCfg::default()
                .crossterm_input_listener(Duration::from_millis(20), 3)
                .poll_timeout(Duration::from_millis(10))
                .tick_interval(Duration::from_secs(1)),
        );

        // Mount components
        assert!(app
            .mount(
                ComponentId::Label,
                Box::new(
                    Label::default()
                        .text("Quetty, the cutest queue manager <3")
                        .alignment(Alignment::Center)
                        .background(Color::Reset)
                        .foreground(Color::Green)
                        .modifiers(TextModifiers::BOLD),
                ),
                Vec::default(),
            )
            .is_ok());

        assert!(app
            .mount(
                ComponentId::Messages,
                Box::new(Messages::new()),
                Vec::default(),
            )
            .is_ok());
        assert!(app
            .mount(
                ComponentId::MessageDetails,
                Box::new(MessageDetails::new()),
                Vec::default(),
            )
            .is_ok());
        assert!(app.active(&ComponentId::MessageDetails).is_ok());
        app
    }
}

impl<T> Update<Msg> for Model<T>
where
    T: TerminalAdapter,
{
    fn update(&mut self, msg: Option<Msg>) -> Option<Msg> {
        if let Some(msg) = msg {
            // Set redraw
            self.redraw = true;
            // Match message
            match msg {
                Msg::AppClose => {
                    self.quit = true; // Terminate
                    None
                },
                Msg::Submit(lines) => {
                    match ClipboardContext::new() {
                        Ok(mut ctx) => {
                            if let Err(e) = ctx.set_contents(lines.join("\n")) {
                                //TODO: Move to global error handler
                                println!("Error during copying data to clipboard: {}", e);
                            }
                        }
                        Err(e) => {
                            //TODO: Move to global error handler
                            println!("Failed to initialize clipboard context: {}", e);
                        }
                    }
                    None
                },

                _ => None
           }
        } else {
            None
        }
    }
}
