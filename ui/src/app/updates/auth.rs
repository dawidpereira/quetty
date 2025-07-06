use crate::app::model::Model;
use crate::components::auth_popup::{AuthPopup, AuthPopupState};
use crate::components::common::{AuthActivityMsg, AzureDiscoveryMsg, ComponentId, Msg};
use crate::components::state::ComponentStateMount;
use crate::error::AppResult;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn update_auth(&mut self, msg: AuthActivityMsg) -> AppResult<Option<Msg>> {
        match msg {
            AuthActivityMsg::Login => {
                // Initiate login process
                if let Some(auth_service) = &self.auth_service {
                    let auth_service = auth_service.clone();
                    self.task_manager.execute_background(async move {
                        auth_service.initiate_authentication().await
                    });
                }
                Ok(None)
            }

            AuthActivityMsg::ShowDeviceCode {
                user_code,
                verification_url,
                message,
                expires_in,
            } => {
                // Remove loading indicator if shown
                if self.app.mounted(&ComponentId::LoadingIndicator) {
                    self.app
                        .umount(&ComponentId::LoadingIndicator)
                        .map_err(|e| crate::error::AppError::Component(e.to_string()))?;
                }

                // Calculate expiration time
                let expires_at =
                    Some(std::time::Instant::now() + std::time::Duration::from_secs(expires_in));

                // Show device code popup
                let popup = AuthPopup::new(AuthPopupState::ShowingDeviceCode {
                    user_code,
                    verification_url,
                    message,
                    expires_at,
                });

                if self.app.mounted(&ComponentId::AuthPopup) {
                    self.app
                        .umount(&ComponentId::AuthPopup)
                        .map_err(|e| crate::error::AppError::Component(e.to_string()))?;
                }

                self.app
                    .mount_with_state(ComponentId::AuthPopup, popup, Vec::default())?;

                self.app
                    .active(&ComponentId::AuthPopup)
                    .map_err(|e| crate::error::AppError::Component(e.to_string()))?;

                // Start a timer to refresh the auth popup every second
                let tx = self.state_manager.tx_to_main.clone();
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
                    loop {
                        interval.tick().await;
                        if tx.send(Msg::Tick).is_err() {
                            break;
                        }
                    }
                });

                Ok(None)
            }

            AuthActivityMsg::AuthenticationSuccess => {
                // Show success and close popup after a delay
                if self.app.mounted(&ComponentId::AuthPopup) {
                    // Remount with success state
                    self.app
                        .umount(&ComponentId::AuthPopup)
                        .map_err(|e| crate::error::AppError::Component(e.to_string()))?;
                    let popup = AuthPopup::new(AuthPopupState::Success);
                    self.app
                        .mount_with_state(ComponentId::AuthPopup, popup, Vec::default())?;
                }

                // Schedule popup removal after 2 seconds
                let tx = self.state_manager.tx_to_main.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    let _ = tx.send(Msg::AuthActivity(AuthActivityMsg::CancelAuthentication));
                });

                // Clear authentication flag
                self.state_manager.is_authenticating = false;

                // Check if we need to start Azure discovery
                let config = crate::config::get_config_or_panic();
                let has_connection_string = config.servicebus().connection_string().is_some();

                log::info!("Authentication successful! Checking next steps...");
                log::info!("Has connection string: {has_connection_string}");
                log::info!(
                    "Service bus manager initialized: {}",
                    self.service_bus_manager.is_some()
                );

                if !has_connection_string {
                    // No connection string configured, start discovery
                    log::info!("No connection string found, starting Azure discovery flow");
                    Ok(Some(Msg::AzureDiscovery(AzureDiscoveryMsg::StartDiscovery)))
                } else {
                    // Connection string available, load namespaces directly
                    log::info!("Connection string available, loading namespaces directly");
                    self.queue_manager.load_namespaces();
                    Ok(None)
                }
            }

            AuthActivityMsg::AuthenticationFailed(error) => {
                // Clear authentication flag
                self.state_manager.is_authenticating = false;

                // Show error in popup
                let popup = AuthPopup::new(AuthPopupState::Failed(error));

                if self.app.mounted(&ComponentId::AuthPopup) {
                    self.app
                        .umount(&ComponentId::AuthPopup)
                        .map_err(|e| crate::error::AppError::Component(e.to_string()))?;
                }

                self.app
                    .mount_with_state(ComponentId::AuthPopup, popup, Vec::default())?;

                self.app
                    .active(&ComponentId::AuthPopup)
                    .map_err(|e| crate::error::AppError::Component(e.to_string()))?;
                Ok(None)
            }

            AuthActivityMsg::CancelAuthentication => {
                // Close auth popup
                if self.app.mounted(&ComponentId::AuthPopup) {
                    self.app
                        .umount(&ComponentId::AuthPopup)
                        .map_err(|e| crate::error::AppError::Component(e.to_string()))?;

                    // Force redraw after unmounting auth popup
                    self.state_manager.set_redraw(true);
                }

                // If we're still in the authentication phase (not logged in anywhere), close the app
                if self.state_manager.is_authenticating {
                    log::info!("Authentication cancelled - closing application");
                    self.state_manager.is_authenticating = false;
                    return Ok(Some(Msg::AppClose));
                }

                // Otherwise, return focus to previous component
                self.app
                    .active(&ComponentId::Messages)
                    .map_err(|e| crate::error::AppError::Component(e.to_string()))?;
                Ok(None)
            }

            AuthActivityMsg::CopyDeviceCode => {
                // Debounce rapid copy requests
                let now = std::time::Instant::now();
                if let Some(last_copy) = self.state_manager.last_device_code_copy {
                    if now.duration_since(last_copy).as_millis() < 500 {
                        // Ignore if less than 500ms since last copy
                        return Ok(None);
                    }
                }
                self.state_manager.last_device_code_copy = Some(now);

                // Copy device code to clipboard
                if let Some(auth_service) = &self.auth_service {
                    let auth_service = auth_service.clone();
                    let tx = self.state_manager.tx_to_main.clone();
                    self.task_manager.execute_background(async move {
                        match auth_service.get_device_code_info().await {
                            Some(device_info) => {
                                // Use copypasta to copy to clipboard
                                use copypasta::{ClipboardContext, ClipboardProvider};
                                match ClipboardContext::new() {
                                    Ok(mut ctx) => {
                                        if let Err(e) =
                                            ctx.set_contents(device_info.user_code.clone())
                                        {
                                            let _ = tx.send(Msg::ShowError(format!(
                                                "Failed to copy device code: {e}"
                                            )));
                                        } else {
                                            log::info!("Device code copied to clipboard");
                                            let _ = tx
                                                .send(Msg::ShowSuccess("Code copied!".to_string()));
                                        }
                                    }
                                    Err(e) => {
                                        let _ = tx.send(Msg::ShowError(format!(
                                            "Failed to access clipboard: {e}"
                                        )));
                                    }
                                }
                            }
                            None => {
                                let _ =
                                    tx.send(Msg::ShowError("No device code available".to_string()));
                            }
                        }
                        Ok::<(), crate::error::AppError>(())
                    });
                }
                Ok(None)
            }

            AuthActivityMsg::OpenVerificationUrl => {
                // Open verification URL in browser
                if let Some(auth_service) = &self.auth_service {
                    let auth_service = auth_service.clone();
                    let tx = self.state_manager.tx_to_main.clone();
                    self.task_manager.execute_background(async move {
                        match auth_service.get_device_code_info().await {
                            Some(device_info) => {
                                // Use open crate to open URL in default browser
                                if let Err(e) = open::that(&device_info.verification_uri) {
                                    let _ = tx.send(Msg::ShowError(format!("Failed to open URL: {e}")));
                                } else {
                                    log::info!("Opened verification URL in browser");
                                    let _ = tx.send(Msg::ShowSuccess("Browser opened! Please enter the device code to authenticate.".to_string()));
                                }
                            }
                            None => {
                                let _ = tx.send(Msg::ShowError("No verification URL available".to_string()));
                            }
                        }
                        Ok::<(), crate::error::AppError>(())
                    });
                }
                Ok(None)
            }
        }
    }
}
