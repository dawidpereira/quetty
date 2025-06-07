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

// Render the confirmation popup centered on the screen
pub fn view_confirmation_popup(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
) -> Result<(), AppError> {
    // Create a centered box for the confirmation popup with better sizing
    let popup_width = 90; // Increased width for longer messages
    let popup_height = 12; // Increased height for multi-line messages
    let area = f.area();

    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(
        popup_x,
        popup_y,
        popup_width.min(area.width),
        popup_height.min(area.height),
    );

    app.view(&ComponentId::ConfirmationPopup, f, popup_area);

    // Make sure the popup has focus
    app.active(&ComponentId::ConfirmationPopup)
        .map_err(|e| AppError::Component(e.to_string()))?;

    Ok(())
}

// Render the success popup centered on the screen
pub fn view_success_popup(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
) -> Result<(), AppError> {
    // Create a centered box for the success popup
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

    app.view(&ComponentId::SuccessPopup, f, popup_area);

    // Make sure the popup has focus
    app.active(&ComponentId::SuccessPopup)
        .map_err(|e| AppError::Component(e.to_string()))?;

    Ok(())
}

// Higher-order function to wrap view functions with popup handling
pub fn with_popup<F>(
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
    // First, try to render the confirmation popup if it exists
    if app.mounted(&ComponentId::ConfirmationPopup) {
        return view_confirmation_popup(app, f);
    }

    // Then, try to render the success popup if it exists
    if app.mounted(&ComponentId::SuccessPopup) {
        return view_success_popup(app, f);
    }

    // Then, try to render the error popup if it exists
    if app.mounted(&ComponentId::ErrorPopup) {
        return view_error_popup(app, f);
    }

    // If no popups, proceed with the original view function
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

// View function for help screen
pub fn view_help_screen(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
    _chunks: &[Rect],
) -> Result<(), AppError> {
    // Create a centered box for the help screen
    let area = f.area();
    let popup_width = area.width.saturating_sub(10);
    let popup_height = area.height.saturating_sub(6);

    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(
        popup_x,
        popup_y,
        popup_width.min(area.width),
        popup_height.min(area.height),
    );

    app.view(&ComponentId::HelpScreen, f, popup_area);
    app.active(&ComponentId::HelpScreen)
        .map_err(|e| AppError::Component(e.to_string()))?;

    Ok(())
}

// View function for theme picker
pub fn view_theme_picker(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
    _chunks: &[Rect],
) -> Result<(), AppError> {
    // Create a centered box for the theme picker
    let popup_width = 60;
    let popup_height = 20;
    let area = f.area();

    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(
        popup_x,
        popup_y,
        popup_width.min(area.width),
        popup_height.min(area.height),
    );

    app.view(&ComponentId::ThemePicker, f, popup_area);
    app.active(&ComponentId::ThemePicker)
        .map_err(|e| AppError::Component(e.to_string()))?;
    Ok(())
}
