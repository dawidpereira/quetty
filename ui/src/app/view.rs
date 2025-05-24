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

pub fn view_namespace_picker(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
    chunks: &[Rect],
) -> Result<(), AppError> {
    // First, try to render the error popup if it exists
    if app.mounted(&ComponentId::ErrorPopup) {
        return view_error_popup(app, f);
    }

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
    // First, try to render the error popup if it exists
    if app.mounted(&ComponentId::ErrorPopup) {
        return view_error_popup(app, f);
    }

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
    // First, try to render the error popup if it exists
    if app.mounted(&ComponentId::ErrorPopup) {
        return view_error_popup(app, f);
    }

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
    // First, try to render the error popup if it exists
    if app.mounted(&ComponentId::ErrorPopup) {
        return view_error_popup(app, f);
    }
    
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
