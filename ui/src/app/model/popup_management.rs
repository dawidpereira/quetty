use super::{AppState, Model};
use crate::components::common::ComponentId;
use crate::components::confirmation_popup::ConfirmationPopup;
use crate::components::error_popup::ErrorPopup;
use crate::components::global_key_watcher::GlobalKeyWatcher;
use crate::components::loading_indicator::LoadingIndicator;
use crate::components::number_input_popup::NumberInputPopup;
use crate::components::state::ComponentStateMount;
use crate::components::success_popup::SuccessPopup;
use crate::components::theme_picker::ThemePicker;
use crate::error::{AppError, AppResult};
use tuirealm::terminal::TerminalAdapter;
use tuirealm::{Sub, SubClause, SubEventClause};

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn mount_loading_indicator(&mut self, message: &str) -> AppResult<()> {
        log::debug!("Mounting loading indicator with message: {}", message);

        // Unmount existing loading indicator if any
        if self.app.mounted(&ComponentId::LoadingIndicator) {
            if let Err(e) = self.app.umount(&ComponentId::LoadingIndicator) {
                log::error!("Failed to unmount loading indicator: {}", e);
            }
        }

        // Mount with ComponentState pattern using extension trait
        self.app.mount_with_state(
            ComponentId::LoadingIndicator,
            LoadingIndicator::new(message, true),
            vec![Sub::new(SubEventClause::Tick, SubClause::Always)],
        )?;

        log::debug!("Loading indicator mounted successfully");
        Ok(())
    }

    /// Mount error popup and give focus to it
    pub fn mount_error_popup(&mut self, error: &AppError) -> AppResult<()> {
        log::error!("Displaying error popup: {}", error);

        self.app.remount_with_state(
            ComponentId::ErrorPopup,
            ErrorPopup::new(error),
            Vec::default(),
        )?;

        self.app
            .active(&ComponentId::ErrorPopup)
            .map_err(|e| AppError::Component(e.to_string()))?;

        self.redraw = true;

        Ok(())
    }

    /// Unmount error popup and return focus to previous component
    pub fn unmount_error_popup(&mut self) -> AppResult<()> {
        // Note: We can't access the component directly through TUI realm's API
        // to call unmount(), but the ComponentState pattern ensures components
        // are properly initialized when mounted.

        self.app
            .umount(&ComponentId::ErrorPopup)
            .map_err(|e| AppError::Component(e.to_string()))?;

        // Return to appropriate state
        self.activate_component_for_current_state()?;
        self.redraw = true;
        Ok(())
    }

    /// Mount success popup and give focus to it
    pub fn mount_success_popup(&mut self, message: &str) -> AppResult<()> {
        log::info!("Displaying success popup: {}", message);

        self.app.remount_with_state(
            ComponentId::SuccessPopup,
            SuccessPopup::new(message),
            Vec::default(),
        )?;

        self.app
            .active(&ComponentId::SuccessPopup)
            .map_err(|e| AppError::Component(e.to_string()))?;

        self.redraw = true;

        Ok(())
    }

    /// Unmount success popup and return focus to previous component
    pub fn unmount_success_popup(&mut self) -> AppResult<()> {
        self.app
            .umount(&ComponentId::SuccessPopup)
            .map_err(|e| AppError::Component(e.to_string()))?;

        // Return to appropriate state
        self.activate_component_for_current_state()?;
        self.redraw = true;
        Ok(())
    }

    pub fn mount_confirmation_popup(&mut self, title: &str, message: &str) -> AppResult<()> {
        self.app.remount_with_state(
            ComponentId::ConfirmationPopup,
            ConfirmationPopup::new(title, message),
            Vec::default(),
        )?;

        self.app
            .active(&ComponentId::ConfirmationPopup)
            .map_err(|e| AppError::Component(e.to_string()))?;

        Ok(())
    }

    pub fn mount_number_input_popup(
        &mut self,
        title: String,
        message: String,
        min_value: usize,
        max_value: usize,
    ) -> AppResult<()> {
        self.app.remount_with_state(
            ComponentId::NumberInputPopup,
            NumberInputPopup::new(title, message, min_value, max_value),
            Vec::default(),
        )?;

        self.app
            .active(&ComponentId::NumberInputPopup)
            .map_err(|e| AppError::Component(e.to_string()))?;

        Ok(())
    }

    pub fn unmount_confirmation_popup(&mut self) -> AppResult<()> {
        self.app
            .umount(&ComponentId::ConfirmationPopup)
            .map_err(|e| AppError::Component(e.to_string()))?;

        // Return to appropriate state
        self.activate_component_for_current_state()?;
        self.redraw = true;
        Ok(())
    }

    pub fn unmount_number_input_popup(&mut self) -> AppResult<()> {
        self.app
            .umount(&ComponentId::NumberInputPopup)
            .map_err(|e| AppError::Component(e.to_string()))?;

        // Return to appropriate state
        self.activate_component_for_current_state()?;
        self.redraw = true;
        Ok(())
    }

    pub fn mount_theme_picker(&mut self) -> AppResult<()> {
        // Store the current state so we can return to it
        self.previous_state = Some(self.app_state.clone());

        // Mount theme picker with ComponentState pattern using extension trait
        self.app.remount_with_state(
            ComponentId::ThemePicker,
            ThemePicker::new(),
            Vec::default(),
        )?;

        self.app
            .active(&ComponentId::ThemePicker)
            .map_err(|e| AppError::Component(e.to_string()))?;

        self.app_state = AppState::ThemePicker;
        self.redraw = true;

        Ok(())
    }

    pub fn unmount_theme_picker(&mut self) -> AppResult<()> {
        self.app
            .umount(&ComponentId::ThemePicker)
            .map_err(|e| AppError::Component(e.to_string()))?;

        // Return to previous state
        if let Some(prev_state) = self.previous_state.take() {
            self.app_state = prev_state;
        } else {
            self.app_state = AppState::NamespacePicker;
        }

        // Return to appropriate component based on state
        self.activate_component_for_current_state()?;
        self.redraw = true;
        Ok(())
    }

    /// Update the GlobalKeyWatcher's editing state
    pub fn update_global_key_watcher_editing_state(&mut self) -> AppResult<()> {
        self.app
            .remount(
                ComponentId::GlobalKeyWatcher,
                Box::new(GlobalKeyWatcher::new(self.is_editing_message)),
                vec![Sub::new(SubEventClause::Any, SubClause::Always)],
            )
            .map_err(|e| AppError::Component(e.to_string()))?;

        Ok(())
    }

    /// Helper method to activate the appropriate component for the current state
    fn activate_component_for_current_state(&mut self) -> AppResult<()> {
        match self.app_state {
            AppState::NamespacePicker => {
                self.app
                    .active(&ComponentId::NamespacePicker)
                    .map_err(|e| AppError::Component(e.to_string()))?;
            }
            AppState::QueuePicker => {
                self.app
                    .active(&ComponentId::QueuePicker)
                    .map_err(|e| AppError::Component(e.to_string()))?;
            }
            AppState::MessagePicker => {
                self.app
                    .active(&ComponentId::Messages)
                    .map_err(|e| AppError::Component(e.to_string()))?;
            }
            AppState::MessageDetails => {
                self.app
                    .active(&ComponentId::MessageDetails)
                    .map_err(|e| AppError::Component(e.to_string()))?;
            }
            AppState::Loading => {
                // If we were showing a loading indicator, just continue showing it
                // No need to activate any specific component
                // The loading indicator will be updated or closed by its own message flow
            }
            AppState::HelpScreen => {
                self.app
                    .active(&ComponentId::HelpScreen)
                    .map_err(|e| AppError::Component(e.to_string()))?;
            }
            AppState::ThemePicker => {
                // This shouldn't happen, but just in case
                self.app_state = AppState::NamespacePicker;
                self.app
                    .active(&ComponentId::NamespacePicker)
                    .map_err(|e| AppError::Component(e.to_string()))?;
            }
        }
        Ok(())
    }
}
