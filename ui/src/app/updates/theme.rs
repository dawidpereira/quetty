use crate::app::model::Model;
use crate::components::common::{Msg, ThemeActivityMsg};
use crate::theme::ThemeManager;
use crate::theme::types::ThemeConfig;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn update_theme(&mut self, msg: ThemeActivityMsg) -> Option<Msg> {
        match msg {
            ThemeActivityMsg::ThemeSelected(theme_name, flavor_name) => {
                self.handle_theme_selected(theme_name, flavor_name)
            }
            ThemeActivityMsg::ThemePickerClosed => self.handle_theme_picker_closed(),
        }
    }

    fn handle_theme_selected(&mut self, theme_name: String, flavor_name: String) -> Option<Msg> {
        log::info!("Switching to theme: {} ({})", theme_name, flavor_name);

        // Create theme config
        let theme_config = ThemeConfig {
            theme_name: theme_name.clone(),
            flavor_name: flavor_name.clone(),
        };

        // Switch the theme
        match ThemeManager::global().lock() {
            Ok(mut manager) => {
                if let Err(e) = manager.switch_theme_from_config(&theme_config) {
                    log::error!("Failed to switch theme: {}", e);
                    return Some(Msg::Error(e));
                }
            }
            Err(e) => {
                log::error!("Failed to acquire theme manager lock: {}", e);
                return Some(Msg::Error(crate::error::AppError::Component(format!(
                    "Failed to acquire theme manager lock: {}",
                    e
                ))));
            }
        }

        // Close the theme picker
        if let Err(e) = self.unmount_theme_picker() {
            log::error!("Failed to unmount theme picker: {}", e);
            return Some(Msg::Error(e));
        }

        // Force a complete redraw to apply the new theme
        self.redraw = true;

        log::info!(
            "Successfully switched to theme: {} ({})",
            theme_name,
            flavor_name
        );
        None
    }

    fn handle_theme_picker_closed(&mut self) -> Option<Msg> {
        log::debug!("Theme picker closed");

        if let Err(e) = self.unmount_theme_picker() {
            log::error!("Failed to unmount theme picker: {}", e);
            Some(Msg::Error(e))
        } else {
            None
        }
    }
}
