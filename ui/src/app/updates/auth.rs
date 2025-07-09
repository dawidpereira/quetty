use crate::app::model::Model;
use crate::components::auth_popup::{AuthPopup, AuthPopupState};
use crate::components::common::{AuthActivityMsg, AzureDiscoveryMsg, ComponentId, Msg};
use crate::components::state::ComponentStateMount;
use crate::error::AppResult;
use crate::utils::auth::AuthUtils;
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
                let auth_method = &config.azure_ad().auth_method;
                let has_connection_string = config.servicebus().has_connection_string();

                log::info!("Authentication successful! Checking next steps...");
                log::info!(
                    "Auth method: {} ({})",
                    auth_method,
                    AuthUtils::auth_method_description(config)
                );
                log::info!("Has connection string: {has_connection_string}");
                log::info!(
                    "Service bus manager initialized: {}",
                    self.service_bus_manager.is_some()
                );

                // For device code flow, check if we have all required configuration first
                if AuthUtils::is_device_code_auth(config) {
                    let azure_ad_config = config.azure_ad();
                    if azure_ad_config.has_subscription_id()
                        && azure_ad_config.has_resource_group()
                        && azure_ad_config.has_namespace()
                    {
                        log::info!(
                            "Device code authentication - all Azure configuration already available, skipping discovery"
                        );
                        // All required Azure config is present, fetch connection string directly
                        let subscription_id = azure_ad_config
                            .subscription_id()
                            .expect("subscription_id should be present");
                        let resource_group = azure_ad_config
                            .resource_group()
                            .expect("resource_group should be present");
                        let namespace = azure_ad_config
                            .namespace()
                            .expect("namespace should be present");

                        // Set the selected values in state manager so discovery finalization works properly
                        self.state_manager.update_azure_selection(
                            Some(subscription_id.to_string()),
                            Some(resource_group.to_string()),
                            Some(namespace.to_string()),
                        );

                        Ok(Some(Msg::AzureDiscovery(
                            AzureDiscoveryMsg::FetchingConnectionString {
                                subscription_id: subscription_id.to_string(),
                                resource_group: resource_group.to_string(),
                                namespace: namespace.to_string(),
                            },
                        )))
                    } else {
                        log::info!("Device code authentication - starting Azure discovery flow");
                        Ok(Some(Msg::AzureDiscovery(AzureDiscoveryMsg::StartDiscovery)))
                    }
                } else if AuthUtils::is_connection_string_auth(config) {
                    // Connection string auth should have been handled differently
                    log::warn!(
                        "Connection string auth reached device code success handler - this shouldn't happen"
                    );
                    self.queue_manager.load_namespaces();
                    Ok(None)
                } else if config.servicebus().has_connection_string() {
                    // Other auth methods with connection string available
                    log::info!("Connection string available, loading namespaces directly");
                    self.queue_manager.load_namespaces();
                    Ok(None)
                } else if AuthUtils::supports_discovery(config) {
                    // Other auth methods without connection string - start discovery
                    log::info!(
                        "No connection string found, starting Azure discovery flow for {}",
                        AuthUtils::auth_method_description(config)
                    );
                    Ok(Some(Msg::AzureDiscovery(AzureDiscoveryMsg::StartDiscovery)))
                } else {
                    log::warn!(
                        "Auth method {} does not support automatic discovery",
                        AuthUtils::auth_method_description(config)
                    );
                    self.queue_manager.load_namespaces();
                    Ok(None)
                }
            }

            AuthActivityMsg::AuthenticationFailed(error) => {
                // Clear authentication flag
                self.state_manager.is_authenticating = false;

                // Check if the error is due to incomplete configuration
                if error.contains("client ID")
                    || error.contains("tenant ID")
                    || error.contains("Invalid authentication request")
                {
                    // Configuration issue - open config screen instead of showing error popup
                    log::info!(
                        "Authentication failed due to configuration issue, opening config screen"
                    );
                    return Ok(Some(Msg::ToggleConfigScreen));
                }

                // Show error in popup for other types of authentication failures
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

            AuthActivityMsg::TokenRefreshFailed(error) => {
                log::error!("Token refresh failed: {error}");

                // Check if it's a critical error that requires re-authentication
                if error.contains("expired") || error.contains("Invalid refresh token") {
                    // Clear authentication state
                    if let Some(auth_service) = &self.auth_service {
                        let auth_state = auth_service.auth_state_manager();
                        tokio::spawn(async move {
                            auth_state.logout().await;
                        });
                    }

                    // Show error popup with re-authentication prompt
                    let error_msg = format!(
                        "Authentication token refresh failed: {error}. Please log in again."
                    );

                    // Mount error popup
                    self.mount_error_popup(&crate::error::AppError::Auth(error_msg.clone()))?;

                    // Trigger re-authentication after a short delay
                    let tx = self.state_manager.tx_to_main.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        let _ = tx.send(Msg::AuthActivity(AuthActivityMsg::Login));
                    });
                } else {
                    // Non-critical error, just log it
                    log::warn!("Non-critical token refresh error: {error}");
                }

                Ok(None)
            }

            AuthActivityMsg::CreateServiceBusManager => {
                log::info!("Creating Service Bus manager with connection string");

                // For connection string auth, we need to create the Service Bus manager
                // and proceed directly to namespace/queue selection
                let config = crate::config::get_config_or_panic();

                match config.servicebus().connection_string() {
                    Ok(Some(connection_string)) => {
                        // Spawn task to create Service Bus manager
                        let tx = self.state_manager.tx_to_main.clone();
                        let http_client = self.http_client.clone();
                        let connection_string = connection_string.to_string();

                        self.task_manager.execute_background(async move {
                        use azservicebus::{ServiceBusClient as AzureServiceBusClient, ServiceBusClientOptions};
                        use server::service_bus_manager::ServiceBusManager;
                        use std::sync::Arc;
                        use tokio::sync::Mutex;

                        match AzureServiceBusClient::new_from_connection_string(
                            &connection_string,
                            ServiceBusClientOptions::default(),
                        ).await {
                            Ok(azure_service_bus_client) => {
                                log::info!("Service Bus client created successfully");

                                // Create ServiceBusManager
                                let config = crate::config::get_config_or_panic();
                                let azure_ad_config = config.azure_ad();
                                let statistics_config = server::service_bus_manager::azure_management_client::StatisticsConfig::new(
                                    config.queue_stats_display_enabled(),
                                    config.queue_stats_cache_ttl_seconds(),
                                    config.queue_stats_use_management_api(),
                                );
                                let batch_config = config.batch();

                                let service_bus_manager = Arc::new(Mutex::new(ServiceBusManager::new(
                                    Arc::new(Mutex::new(azure_service_bus_client)),
                                    http_client,
                                    azure_ad_config.clone(),
                                    statistics_config,
                                    batch_config.clone(),
                                    connection_string,
                                )));

                                // Send the service bus manager to the model
                                let _ = tx.send(Msg::SetServiceBusManager(service_bus_manager));

                                // Send success message after manager is set
                                let _ = tx.send(Msg::AuthActivity(AuthActivityMsg::AuthenticationSuccess));

                                Ok(())
                            }
                            Err(e) => {
                                log::error!("Failed to create Service Bus client: {e}");
                                let _ = tx.send(Msg::AuthActivity(AuthActivityMsg::AuthenticationFailed(
                                    format!("Failed to connect to Service Bus: {e}")
                                )));
                                Err(crate::error::AppError::ServiceBus(e.to_string()))
                            }
                        }
                    });
                    }
                    Ok(None) => {
                        log::error!(
                            "No connection string available for Service Bus manager creation"
                        );
                        return Ok(Some(Msg::AuthActivity(
                            AuthActivityMsg::AuthenticationFailed(
                                "No connection string configured".to_string(),
                            ),
                        )));
                    }
                    Err(e) => {
                        log::error!("Failed to decrypt connection string: {e}");
                        return Ok(Some(Msg::AuthActivity(
                            AuthActivityMsg::AuthenticationFailed(format!(
                                "Connection string decryption failed: {e}"
                            )),
                        )));
                    }
                }

                Ok(None)
            }
        }
    }
}
