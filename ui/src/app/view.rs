use crate::components::common::{ComponentId, Msg};
use crate::error::AppError;
use tuirealm::ratatui::layout::{Constraint, Direction, Layout, Rect};
use tuirealm::{Application, Frame, NoUserEvent};

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
