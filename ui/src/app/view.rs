use crate::components::base_popup::PopupLayout;
use crate::components::common::{ComponentId, Msg};
use crate::error::AppError;
use tuirealm::ratatui::layout::{Constraint, Direction, Layout, Rect};
use tuirealm::{Application, Frame, NoUserEvent};

// Render the error popup centered on the screen using standardized sizing
pub fn view_error_popup(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
) -> Result<(), AppError> {
    let popup_area = PopupLayout::large(f.area());
    app.view(&ComponentId::ErrorPopup, f, popup_area);
    app.active(&ComponentId::ErrorPopup)
        .map_err(|e| AppError::Component(e.to_string()))?;
    Ok(())
}

// Render the confirmation popup centered on the screen using extra wide sizing
pub fn view_confirmation_popup(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
) -> Result<(), AppError> {
    let popup_area = PopupLayout::extra_wide(f.area());
    app.view(&ComponentId::ConfirmationPopup, f, popup_area);
    app.active(&ComponentId::ConfirmationPopup)
        .map_err(|e| AppError::Component(e.to_string()))?;
    Ok(())
}

// Render the success popup centered on the screen using standardized sizing
pub fn view_success_popup(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
) -> Result<(), AppError> {
    // Use a smaller popup area for success messages so they don't cover other popups
    let popup_area = PopupLayout::small(f.area());
    app.view(&ComponentId::SuccessPopup, f, popup_area);
    app.active(&ComponentId::SuccessPopup)
        .map_err(|e| AppError::Component(e.to_string()))?;
    Ok(())
}

// Render the number input popup centered on the screen using standardized sizing
pub fn view_number_input_popup(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
) -> Result<(), AppError> {
    let popup_area = PopupLayout::medium(f.area());
    app.view(&ComponentId::NumberInputPopup, f, popup_area);
    app.active(&ComponentId::NumberInputPopup)
        .map_err(|e| AppError::Component(e.to_string()))?;
    Ok(())
}

// Render the page size popup centered on the screen using standardized sizing
pub fn view_page_size_popup(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
) -> Result<(), AppError> {
    let popup_area = PopupLayout::medium(f.area());
    app.view(&ComponentId::PageSizePopup, f, popup_area);
    app.active(&ComponentId::PageSizePopup)
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
    // Special handling for auth popup - it can coexist with success popup
    let auth_popup_mounted = app.mounted(&ComponentId::AuthPopup);
    let success_popup_mounted = app.mounted(&ComponentId::SuccessPopup);

    // When success popup is shown with auth popup, only show success popup
    // This prevents visual confusion from overlapping popups
    if auth_popup_mounted && success_popup_mounted {
        // Only render success popup when both are mounted
        view_success_popup(app, f)?;
        return Ok(());
    }

    // First, try to render the auth popup if it exists (highest priority for authentication flow)
    if auth_popup_mounted {
        return view_auth_popup(app, f);
    }

    // Then, try to render the page size popup if it exists
    if app.mounted(&ComponentId::PageSizePopup) {
        return view_page_size_popup(app, f);
    }

    // Then, try to render the number input popup if it exists
    if app.mounted(&ComponentId::NumberInputPopup) {
        return view_number_input_popup(app, f);
    }

    // Then, try to render the confirmation popup if it exists
    if app.mounted(&ComponentId::ConfirmationPopup) {
        return view_confirmation_popup(app, f);
    }

    // Then, try to render the success popup if it exists
    if success_popup_mounted {
        return view_success_popup(app, f);
    }

    // Then, try to render the error popup if it exists
    if app.mounted(&ComponentId::ErrorPopup) {
        return view_error_popup(app, f);
    }

    // Then, try to render the loading indicator if it exists
    if app.mounted(&ComponentId::LoadingIndicator) {
        let popup_area = PopupLayout::small(f.area());
        app.view(&ComponentId::LoadingIndicator, f, popup_area);
        return Ok(());
    }

    // Then, try to render Azure discovery pickers if they exist
    if app.mounted(&ComponentId::SubscriptionPicker) {
        let popup_area = PopupLayout::medium(f.area());
        app.view(&ComponentId::SubscriptionPicker, f, popup_area);
        app.active(&ComponentId::SubscriptionPicker)
            .map_err(|e| AppError::Component(e.to_string()))?;
        return Ok(());
    }

    if app.mounted(&ComponentId::ResourceGroupPicker) {
        let popup_area = PopupLayout::medium(f.area());
        app.view(&ComponentId::ResourceGroupPicker, f, popup_area);
        app.active(&ComponentId::ResourceGroupPicker)
            .map_err(|e| AppError::Component(e.to_string()))?;
        return Ok(());
    }

    // During Azure discovery, namespace picker is shown as a popup
    // Note: We don't check for namespace picker here anymore because it should only
    // be shown when explicitly set in the app state, not just because it's mounted

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

// View function for loading indicator using standardized sizing
pub fn view_loading(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
    _chunks: &[Rect],
) -> Result<(), AppError> {
    let popup_area = PopupLayout::small(f.area());
    app.view(&ComponentId::LoadingIndicator, f, popup_area);
    Ok(())
}

// View function for Azure discovery (subscription/resource group/namespace pickers)
pub fn view_azure_discovery(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
    _chunks: &[Rect],
) -> Result<(), AppError> {
    // Show whichever picker is currently mounted
    if app.mounted(&ComponentId::SubscriptionPicker) {
        let popup_area = PopupLayout::medium(f.area());
        app.view(&ComponentId::SubscriptionPicker, f, popup_area);
        app.active(&ComponentId::SubscriptionPicker)
            .map_err(|e| AppError::Component(e.to_string()))?;
    } else if app.mounted(&ComponentId::ResourceGroupPicker) {
        let popup_area = PopupLayout::medium(f.area());
        app.view(&ComponentId::ResourceGroupPicker, f, popup_area);
        app.active(&ComponentId::ResourceGroupPicker)
            .map_err(|e| AppError::Component(e.to_string()))?;
    } else if app.mounted(&ComponentId::NamespacePicker) {
        let popup_area = PopupLayout::medium(f.area());
        app.view(&ComponentId::NamespacePicker, f, popup_area);
        app.active(&ComponentId::NamespacePicker)
            .map_err(|e| AppError::Component(e.to_string()))?;
    }
    Ok(())
}

// View function for help screen using responsive sizing
pub fn view_help_screen(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
    _chunks: &[Rect],
) -> Result<(), AppError> {
    // Help screen needs nearly full screen, so use custom sizing
    let popup_area = PopupLayout::centered(f.area(), 90, 85);
    app.view(&ComponentId::HelpScreen, f, popup_area);
    app.active(&ComponentId::HelpScreen)
        .map_err(|e| AppError::Component(e.to_string()))?;
    Ok(())
}

// View function for theme picker using standardized sizing
pub fn view_theme_picker(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
    _chunks: &[Rect],
) -> Result<(), AppError> {
    let popup_area = PopupLayout::medium(f.area());
    app.view(&ComponentId::ThemePicker, f, popup_area);
    app.active(&ComponentId::ThemePicker)
        .map_err(|e| AppError::Component(e.to_string()))?;
    Ok(())
}

pub fn view_config_screen(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
    _chunks: &[Rect],
) -> Result<(), AppError> {
    app.view(&ComponentId::ConfigScreen, f, f.area());
    app.active(&ComponentId::ConfigScreen)
        .map_err(|e| AppError::Component(e.to_string()))?;
    Ok(())
}

// View function for auth popup using standardized sizing
pub fn view_auth_popup(
    app: &mut Application<ComponentId, Msg, NoUserEvent>,
    f: &mut Frame,
) -> Result<(), AppError> {
    let popup_area = PopupLayout::medium(f.area());
    app.view(&ComponentId::AuthPopup, f, popup_area);
    app.active(&ComponentId::AuthPopup)
        .map_err(|e| AppError::Component(e.to_string()))?;
    Ok(())
}
