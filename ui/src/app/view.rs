use crate::components::common::{ComponentId, Msg};
use crate::error::AppError;
use tuirealm::ratatui::layout::{Constraint, Direction, Layout, Rect};
use tuirealm::{Application, Frame, NoUserEvent};

// Render the error popup centered on the screen
pub fn view_error_popup(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
) -> Result<(), AppError> {
    // Create a centered box for the error popup
    let popup_width = 60;
    let popup_height = 10;
    let area = f.area();

    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(
        popup_x,
        popup_y,
        popup_width.min(area.width),
        popup_height.min(area.height),
    );

    app.view(&ComponentId::ErrorPopup, f, popup_area);

    // Make sure the popup has focus
    app.active(&ComponentId::ErrorPopup)
        .map_err(|e| AppError::Component(e.to_string()))?;

    Ok(())
}

// Higher-order function to wrap view functions with error popup handling
pub fn with_error_popup<F>(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
    chunks: &[Rect],
    view_fn: F,
) -> Result<(), AppError>
where
    F: FnOnce(
        &mut Application<ComponentId, Msg, NoUserEvent>,
        &mut Frame,
        &[Rect],
    ) -> Result<(), AppError>,
{
    // First, try to render the error popup if it exists
    if app.mounted(&ComponentId::ErrorPopup) {
        return view_error_popup(app, f);
    }

    // If no error popup, proceed with the original view function
    view_fn(app, f, chunks)
}

// View functions - direct implementations without wrappers

pub fn view_namespace_picker(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
    chunks: &[Rect],
) -> Result<(), AppError> {
    app.view(&ComponentId::NamespacePicker, f, chunks[3]);
    app.active(&ComponentId::NamespacePicker)
        .map_err(|e| AppError::Component(e.to_string()))?;
    Ok(())
}

pub fn view_queue_picker(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
    chunks: &[Rect],
) -> Result<(), AppError> {
    app.view(&ComponentId::QueuePicker, f, chunks[3]);
    app.active(&ComponentId::QueuePicker)
        .map_err(|e| AppError::Component(e.to_string()))?;
    Ok(())
}

pub fn view_message_picker(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
    chunks: &[Rect],
) -> Result<(), AppError> {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints(
            [
                Constraint::Min(49), // Messages
                Constraint::Min(49),
            ]
            .as_ref(),
        )
        .split(chunks[3]);
    app.view(&ComponentId::Messages, f, main_chunks[0]);
    app.view(&ComponentId::MessageDetails, f, main_chunks[1]);
    app.active(&ComponentId::Messages)
        .map_err(|e| AppError::Component(e.to_string()))?;
    Ok(())
}

pub fn view_message_details(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
    chunks: &[Rect],
) -> Result<(), AppError> {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints(
            [
                Constraint::Min(49), // Messages
                Constraint::Min(49),
            ]
            .as_ref(),
        )
        .split(chunks[3]);
    app.view(&ComponentId::Messages, f, main_chunks[0]);
    app.view(&ComponentId::MessageDetails, f, main_chunks[1]);
    app.active(&ComponentId::MessageDetails)
        .map_err(|e| AppError::Component(e.to_string()))?;
    Ok(())
}

// View function for loading indicator
pub fn view_loading(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
    _chunks: &[Rect],
) -> Result<(), AppError> {
    // Create a centered area for the loading indicator
    let area = f.area();
    let popup_width = 60;
    let popup_height = 5;

    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(
        popup_x,
        popup_y,
        popup_width.min(area.width),
        popup_height.min(area.height),
    );

    // Draw the loading indicator in the popup area
    app.view(&ComponentId::LoadingIndicator, f, popup_area);

    Ok(())
}

// View function for help bar
pub fn view_help_bar(
    f: &mut Frame,
    chunks: &[Rect],
    active_component: &ComponentId,
) -> Result<(), AppError> {
    // Create a temporary help bar with the active component
    let mut help_bar = crate::components::help_bar::HelpBar::new();

    // Directly render the help bar with the active component
    help_bar.view_with_active(f, chunks[4], active_component);

    Ok(())
}
