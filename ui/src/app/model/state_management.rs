use super::{AppState, Model};
use crate::app::view::*;
use crate::components::common::ComponentId;
use crate::components::help_bar::HelpBar;
use crate::error::AppResult;
use tuirealm::ratatui::layout::{Constraint, Direction, Layout};
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn view(&mut self) -> AppResult<()> {
        let mut view_result: AppResult<()> = Ok(());

        // Extract values before the closure to avoid borrowing issues
        let current_app_state = self.state_manager.app_state.clone();
        let active_component = match current_app_state {
            AppState::NamespacePicker => ComponentId::NamespacePicker,
            AppState::QueuePicker => ComponentId::QueuePicker,
            AppState::MessagePicker => ComponentId::Messages,
            AppState::MessageDetails => ComponentId::MessageDetails,
            AppState::Loading => ComponentId::LoadingIndicator,
            AppState::HelpScreen => ComponentId::HelpScreen,
            AppState::ThemePicker => ComponentId::ThemePicker,
        };

        // Update active component before drawing
        self.set_active_component(active_component.clone());

        // Get queue state data for help bar before closure
        let queue_state = &self.queue_manager.queue_state;
        let (queue_type, bulk_mode, selected_count) = if active_component == ComponentId::Messages {
            (
                Some(queue_state.current_queue_type.clone()),
                Some(queue_state.bulk_selection.selection_mode),
                Some(queue_state.bulk_selection.selection_count()),
            )
        } else {
            (None, None, None)
        };

        let _ = self.terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(1),
                        Constraint::Length(1), // Label
                        Constraint::Length(2),
                        Constraint::Min(16),   // Main area
                        Constraint::Length(1), // Help bar
                    ]
                    .as_ref(),
                )
                .split(f.area());

            self.app.view(&ComponentId::TextLabel, f, chunks[1]);

            // Apply the view based on the app state, with error popup handling
            view_result = match current_app_state {
                AppState::NamespacePicker => {
                    with_popup(&mut self.app, f, &chunks, view_namespace_picker)
                }
                AppState::QueuePicker => with_popup(&mut self.app, f, &chunks, view_queue_picker),
                AppState::MessagePicker => {
                    with_popup(&mut self.app, f, &chunks, view_message_picker)
                }
                AppState::MessageDetails => {
                    with_popup(&mut self.app, f, &chunks, view_message_details)
                }
                AppState::Loading => with_popup(&mut self.app, f, &chunks, view_loading),
                AppState::HelpScreen => with_popup(&mut self.app, f, &chunks, view_help_screen),
                AppState::ThemePicker => view_theme_picker(&mut self.app, f, &chunks),
            };

            // View help bar (if not showing any popup) with active component
            if !self.app.mounted(&ComponentId::ErrorPopup)
                && !self.app.mounted(&ComponentId::SuccessPopup)
                && !self.app.mounted(&ComponentId::ConfirmationPopup)
                && !self.app.mounted(&ComponentId::NumberInputPopup)
                && !self.app.mounted(&ComponentId::PageSizePopup)
                && !self.app.mounted(&ComponentId::ThemePicker)
            {
                // Create a temporary help bar with the active component
                let mut help_bar = HelpBar::new();

                help_bar.view_with_active_and_queue_type(
                    f,
                    chunks[4],
                    &active_component,
                    queue_type.as_ref(),
                    bulk_mode,
                    selected_count,
                );
            }
        });

        view_result
    }
}
