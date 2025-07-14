use super::{AppState, Model};
use crate::components::common::ComponentId;
use crate::components::confirmation_popup::ConfirmationPopup;
use crate::components::error_popup::ErrorPopup;
use crate::components::global_key_watcher::GlobalKeyWatcher;
use crate::components::loading_indicator::LoadingIndicator;
use crate::components::number_input_popup::NumberInputPopup;
use crate::components::page_size_popup::PageSizePopup;
use crate::components::password_popup::PasswordPopup;
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
        log::debug!("Mounting loading indicator with message: {message}");

        // Unmount existing loading indicator if any
        if self.app.mounted(&ComponentId::LoadingIndicator) {
            if let Err(e) = self.app.umount(&ComponentId::LoadingIndicator) {
                log::error!("Failed to unmount loading indicator: {e}");
            }
        }

        // Mount with ComponentState pattern using extension trait
        self.app.mount_with_state(
            ComponentId::LoadingIndicator,
            LoadingIndicator::new(message, true),
            vec![
                Sub::new(SubEventClause::Tick, SubClause::Always),
                Sub::new(SubEventClause::Any, SubClause::Always),
            ],
        )?;

        self.app
            .active(&ComponentId::LoadingIndicator)
            .map_err(|e| AppError::Component(e.to_string()))?;

        self.set_redraw(true);

        log::debug!("Loading indicator mounted successfully");
        Ok(())
    }

    /// Mount error popup and give focus to it
    pub fn mount_error_popup(&mut self, error: &AppError) -> AppResult<()> {
        log::error!("Displaying error popup: {error}");

        self.app.remount_with_state(
            ComponentId::ErrorPopup,
            ErrorPopup::new(error),
            Vec::default(),
        )?;

        self.app
            .active(&ComponentId::ErrorPopup)
            .map_err(|e| AppError::Component(e.to_string()))?;

        self.set_redraw(true);

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
        self.set_redraw(true);
        Ok(())
    }

    /// Mount success popup and give focus to it
    pub fn mount_success_popup(&mut self, message: &str) -> AppResult<()> {
        log::info!("Displaying success popup: {message}");

        self.app.remount_with_state(
            ComponentId::SuccessPopup,
            SuccessPopup::new(message),
            Vec::default(),
        )?;

        // Only take focus if auth popup is not shown
        // This prevents keyboard input confusion when copying device code
        if !self.app.mounted(&ComponentId::AuthPopup) {
            self.app
                .active(&ComponentId::SuccessPopup)
                .map_err(|e| AppError::Component(e.to_string()))?;
        }

        self.set_redraw(true);

        Ok(())
    }

    /// Unmount success popup and return focus to previous component
    pub fn unmount_success_popup(&mut self) -> AppResult<()> {
        self.app
            .umount(&ComponentId::SuccessPopup)
            .map_err(|e| AppError::Component(e.to_string()))?;

        // Return to appropriate state
        self.activate_component_for_current_state()?;
        self.set_redraw(true);
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

    pub fn mount_page_size_popup(&mut self) -> AppResult<()> {
        self.app.remount_with_state(
            ComponentId::PageSizePopup,
            PageSizePopup::new(),
            Vec::default(),
        )?;

        self.app
            .active(&ComponentId::PageSizePopup)
            .map_err(|e| AppError::Component(e.to_string()))?;

        Ok(())
    }

    pub fn unmount_confirmation_popup(&mut self) -> AppResult<()> {
        self.app
            .umount(&ComponentId::ConfirmationPopup)
            .map_err(|e| AppError::Component(e.to_string()))?;

        // Return to appropriate state
        self.activate_component_for_current_state()?;
        self.set_redraw(true);
        Ok(())
    }

    pub fn unmount_number_input_popup(&mut self) -> AppResult<()> {
        self.app
            .umount(&ComponentId::NumberInputPopup)
            .map_err(|e| AppError::Component(e.to_string()))?;

        // Return to appropriate state
        self.activate_component_for_current_state()?;
        self.set_redraw(true);
        Ok(())
    }

    pub fn unmount_page_size_popup(&mut self) -> AppResult<()> {
        self.app
            .umount(&ComponentId::PageSizePopup)
            .map_err(|e| AppError::Component(e.to_string()))?;

        // Return to appropriate state
        self.activate_component_for_current_state()?;
        self.set_redraw(true);
        Ok(())
    }

    pub fn mount_theme_picker(&mut self) -> AppResult<()> {
        // Store the current state so we can return to it
        self.state_manager.previous_state = Some(self.state_manager.app_state.clone());

        // Mount theme picker with ComponentState pattern using extension trait
        self.app.remount_with_state(
            ComponentId::ThemePicker,
            ThemePicker::new(),
            Vec::default(),
        )?;

        self.app
            .active(&ComponentId::ThemePicker)
            .map_err(|e| AppError::Component(e.to_string()))?;

        self.state_manager.app_state = AppState::ThemePicker;
        self.set_redraw(true);

        Ok(())
    }

    pub fn unmount_theme_picker(&mut self) -> AppResult<()> {
        self.app
            .umount(&ComponentId::ThemePicker)
            .map_err(|e| AppError::Component(e.to_string()))?;

        // Return to previous state
        if let Some(prev_state) = self.state_manager.previous_state.take() {
            self.state_manager.app_state = prev_state;
        } else {
            self.state_manager.app_state = AppState::NamespacePicker;
        }

        // Return to appropriate component based on state
        self.activate_component_for_current_state()?;
        self.set_redraw(true);
        Ok(())
    }

    pub fn mount_config_screen(&mut self) -> AppResult<()> {
        use crate::components::config_screen::ConfigScreen;

        // Check if config screen is already mounted and active
        if self.app.mounted(&ComponentId::ConfigScreen)
            && self.state_manager.app_state == AppState::ConfigScreen
        {
            log::debug!("ConfigScreen already mounted and active, skipping");
            return Ok(());
        }

        // Store the current state so we can return to it
        self.state_manager.previous_state = Some(self.state_manager.app_state.clone());

        // Mount config screen with ComponentState pattern using extension trait
        self.app.remount_with_state(
            ComponentId::ConfigScreen,
            ConfigScreen::new(),
            vec![Sub::new(SubEventClause::Any, SubClause::Always)],
        )?;

        self.app
            .active(&ComponentId::ConfigScreen)
            .map_err(|e| AppError::Component(e.to_string()))?;

        self.state_manager.app_state = AppState::ConfigScreen;
        self.set_redraw(true);

        Ok(())
    }

    pub fn unmount_config_screen(&mut self) -> AppResult<()> {
        // Check if config screen is actually mounted before trying to unmount
        if !self.app.mounted(&ComponentId::ConfigScreen) {
            log::debug!("ConfigScreen not mounted, skipping unmount");
        } else {
            self.app
                .umount(&ComponentId::ConfigScreen)
                .map_err(|e| AppError::Component(e.to_string()))?;
        }

        // Return to previous state
        if let Some(prev_state) = self.state_manager.previous_state.take() {
            self.state_manager.app_state = prev_state;
        } else {
            self.state_manager.app_state = AppState::NamespacePicker;
        }

        // Return to appropriate component based on state
        self.activate_component_for_current_state()?;
        self.set_redraw(true);
        Ok(())
    }

    pub fn mount_password_popup(&mut self, error_message: Option<String>) -> AppResult<()> {
        self.mount_password_popup_with_methods(error_message, None)
    }

    pub fn mount_password_popup_with_methods(
        &mut self,
        error_message: Option<String>,
        encrypted_methods: Option<Vec<String>>,
    ) -> AppResult<()> {
        use crate::config;

        // Store the current state so we can return to it
        self.state_manager.previous_state = Some(self.state_manager.app_state.clone());

        // Get encrypted methods from config if not provided
        let methods = encrypted_methods.unwrap_or_else(|| {
            let config = config::get_config_or_panic();
            config.get_encrypted_auth_methods()
        });

        let popup = match (error_message, methods.is_empty()) {
            (Some(error), false) => PasswordPopup::with_error_and_methods(error, methods),
            (Some(error), true) => PasswordPopup::with_error(error),
            (None, false) => PasswordPopup::with_encrypted_methods(methods),
            (None, true) => PasswordPopup::new(),
        };

        // Mount password popup with ComponentState pattern using extension trait
        self.app.remount_with_state(
            ComponentId::PasswordPopup,
            popup,
            vec![Sub::new(SubEventClause::Any, SubClause::Always)],
        )?;

        self.app
            .active(&ComponentId::PasswordPopup)
            .map_err(|e| AppError::Component(e.to_string()))?;

        self.state_manager.app_state = AppState::PasswordPopup;
        self.set_redraw(true);

        // Disable global shortcuts while password popup is active
        self.set_editing_message(true);
        if let Err(e) = self.update_global_key_watcher_editing_state() {
            self.error_reporter.report_key_watcher_error(e);
        }

        Ok(())
    }

    pub fn unmount_password_popup(&mut self) -> AppResult<()> {
        // Check if password popup is actually mounted before trying to unmount
        if !self.app.mounted(&ComponentId::PasswordPopup) {
            log::debug!("PasswordPopup not mounted, skipping unmount");
        } else {
            self.app
                .umount(&ComponentId::PasswordPopup)
                .map_err(|e| AppError::Component(e.to_string()))?;
        }

        // Return to previous state
        if let Some(prev_state) = self.state_manager.previous_state.take() {
            self.state_manager.app_state = prev_state;
        } else {
            self.state_manager.app_state = AppState::NamespacePicker;
        }

        // Return to appropriate component based on state
        self.activate_component_for_current_state()?;
        self.set_redraw(true);

        // Re-enable global shortcuts after password popup is unmounted
        self.set_editing_message(false);
        if let Err(e) = self.update_global_key_watcher_editing_state() {
            self.error_reporter.report_key_watcher_error(e);
        }

        Ok(())
    }

    /// Update the GlobalKeyWatcher's editing state
    pub fn update_global_key_watcher_editing_state(&mut self) -> AppResult<()> {
        self.app
            .remount(
                ComponentId::GlobalKeyWatcher,
                Box::new(GlobalKeyWatcher::new(self.state_manager.is_editing_message)),
                vec![Sub::new(SubEventClause::Any, SubClause::Always)],
            )
            .map_err(|e| AppError::Component(e.to_string()))?;

        Ok(())
    }

    /// Helper method to activate the appropriate component for the current state
    fn activate_component_for_current_state(&mut self) -> AppResult<()> {
        // Check if auth popup is still mounted - it has priority
        if self.app.mounted(&ComponentId::AuthPopup) {
            self.app
                .active(&ComponentId::AuthPopup)
                .map_err(|e| AppError::Component(e.to_string()))?;
            return Ok(());
        }

        match self.state_manager.app_state {
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
                self.state_manager.app_state = AppState::NamespacePicker;
                self.app
                    .active(&ComponentId::NamespacePicker)
                    .map_err(|e| AppError::Component(e.to_string()))?;
            }
            AppState::ConfigScreen => {
                self.app
                    .active(&ComponentId::ConfigScreen)
                    .map_err(|e| AppError::Component(e.to_string()))?;
            }
            AppState::PasswordPopup => {
                self.app
                    .active(&ComponentId::PasswordPopup)
                    .map_err(|e| AppError::Component(e.to_string()))?;
            }
            AppState::AzureDiscovery => {
                // Stay in discovery mode - the active component will be managed by discovery flow
            }
        }
        Ok(())
    }
}
