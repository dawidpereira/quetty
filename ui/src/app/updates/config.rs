use crate::app::model::Model;
use crate::components::common::{ConfigActivityMsg, ConfigUpdateData, Msg};
use crate::error::AppResult;
use std::env;
use std::fs;
use tuirealm::terminal::TerminalAdapter;

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    pub fn update_config(&mut self, msg: ConfigActivityMsg) -> AppResult<Option<Msg>> {
        match msg {
            ConfigActivityMsg::Save(config_data) => self.handle_config_save(config_data),
            ConfigActivityMsg::ConfirmAndProceed(config_data) => {
                self.handle_config_confirm_and_proceed(config_data)
            }
            ConfigActivityMsg::Cancel => self.handle_config_cancel(),
            ConfigActivityMsg::FieldChanged { .. } => {
                // Field changes are handled directly in the component
                Ok(None)
            }
            ConfigActivityMsg::AuthMethodChanged(_) => {
                // Auth method changes are handled directly in the component
                Ok(None)
            }
        }
    }

    fn handle_config_save(&mut self, config_data: ConfigUpdateData) -> AppResult<Option<Msg>> {
        log::info!(
            "Saving configuration with auth method: {}",
            config_data.auth_method
        );

        log::debug!(
            "ConfigUpdateData: tenant_id={:?}, client_id={:?}, connection_string={:?}",
            config_data
                .tenant_id
                .as_ref()
                .map(|s| if s.is_empty() { "<empty>" } else { "<set>" }),
            config_data
                .client_id
                .as_ref()
                .map(|s| if s.is_empty() { "<empty>" } else { "<set>" }),
            config_data
                .connection_string
                .as_ref()
                .map(|s| if s.is_empty() { "<empty>" } else { "<set>" })
        );

        // Save sensitive data to environment variables (which will be written to .env)
        unsafe {
            if let Some(tenant_id) = &config_data.tenant_id {
                env::set_var("AZURE_AD__TENANT_ID", tenant_id);
            }
            if let Some(client_id) = &config_data.client_id {
                env::set_var("AZURE_AD__CLIENT_ID", client_id);
            }
            if let Some(client_secret) = &config_data.client_secret {
                env::set_var("AZURE_AD__CLIENT_SECRET", client_secret);
            }
            if let Some(subscription_id) = &config_data.subscription_id {
                env::set_var("AZURE_AD__SUBSCRIPTION_ID", subscription_id);
            }
            if let Some(resource_group) = &config_data.resource_group {
                env::set_var("AZURE_AD__RESOURCE_GROUP", resource_group);
            }
            if let Some(namespace) = &config_data.namespace {
                env::set_var("AZURE_AD__NAMESPACE", namespace);
            }
            if let Some(connection_string) = &config_data.connection_string {
                env::set_var("SERVICEBUS__CONNECTION_STRING", connection_string);
            }
            if let Some(queue_name) = &config_data.queue_name {
                env::set_var("SERVICEBUS__QUEUE_NAME", queue_name);
            }
        }

        // Write environment variables to .env file
        if let Err(e) = self.write_env_file(&config_data) {
            log::error!("Failed to write .env file: {e}");
            return Err(e);
        }

        // Update config.toml with auth_method and other non-sensitive settings
        if let Err(e) = self.update_config_toml(&config_data) {
            log::error!("Failed to update config.toml: {e}");
            return Err(e);
        }

        log::info!("Configuration saved successfully.");
        Ok(Some(Msg::ShowSuccess(
            "Configuration saved successfully.".to_string(),
        )))
    }

    fn handle_config_cancel(&mut self) -> AppResult<Option<Msg>> {
        log::debug!("Config screen cancelled");

        if let Err(e) = self.unmount_config_screen() {
            self.error_reporter
                .report_mount_error("ConfigScreen", "unmount", e);
        }

        Ok(None)
    }

    fn handle_config_confirm_and_proceed(
        &mut self,
        config_data: ConfigUpdateData,
    ) -> AppResult<Option<Msg>> {
        log::info!(
            "Confirming and proceeding with configuration - auth method: {}",
            config_data.auth_method
        );

        log::debug!(
            "ConfigUpdateData: tenant_id={:?}, client_id={:?}, connection_string={:?}",
            config_data
                .tenant_id
                .as_ref()
                .map(|s| if s.is_empty() { "<empty>" } else { "<set>" }),
            config_data
                .client_id
                .as_ref()
                .map(|s| if s.is_empty() { "<empty>" } else { "<set>" }),
            config_data
                .connection_string
                .as_ref()
                .map(|s| if s.is_empty() { "<empty>" } else { "<set>" })
        );

        // Debug: Log actual values (first few chars only for security)
        if let Some(tenant_id) = &config_data.tenant_id {
            log::debug!(
                "Tenant ID value: {}...",
                &tenant_id.chars().take(8).collect::<String>()
            );
        }
        if let Some(client_id) = &config_data.client_id {
            log::debug!(
                "Client ID value: {}...",
                &client_id.chars().take(8).collect::<String>()
            );
        }

        // Save configuration to files and environment variables
        unsafe {
            if let Some(tenant_id) = &config_data.tenant_id {
                env::set_var("AZURE_AD__TENANT_ID", tenant_id);
            }
            if let Some(client_id) = &config_data.client_id {
                env::set_var("AZURE_AD__CLIENT_ID", client_id);
            }
            if let Some(client_secret) = &config_data.client_secret {
                env::set_var("AZURE_AD__CLIENT_SECRET", client_secret);
            }
            if let Some(subscription_id) = &config_data.subscription_id {
                env::set_var("AZURE_AD__SUBSCRIPTION_ID", subscription_id);
            }
            if let Some(resource_group) = &config_data.resource_group {
                env::set_var("AZURE_AD__RESOURCE_GROUP", resource_group);
            }
            if let Some(namespace) = &config_data.namespace {
                env::set_var("AZURE_AD__NAMESPACE", namespace);
            }
            if let Some(connection_string) = &config_data.connection_string {
                env::set_var("SERVICEBUS__CONNECTION_STRING", connection_string);
            }
            if let Some(queue_name) = &config_data.queue_name {
                env::set_var("SERVICEBUS__QUEUE_NAME", queue_name);
            }
        }

        // Write environment variables to .env file
        if let Err(e) = self.write_env_file(&config_data) {
            log::error!("Failed to write .env file: {e}");
            return Err(e);
        }

        // Update config.toml with auth_method and other non-sensitive settings
        if let Err(e) = self.update_config_toml(&config_data) {
            log::error!("Failed to update config.toml: {e}");
            return Err(e);
        }

        // Reload configuration to pick up the new environment variables
        if let Err(e) = crate::config::reload_config() {
            log::error!("Failed to reload configuration: {e}");
            return Err(crate::error::AppError::Config(format!(
                "Configuration reload failed: {e}"
            )));
        }

        // Recreate the auth service with the updated configuration
        if let Err(e) = self.create_auth_service() {
            log::error!("Failed to create auth service: {e}");
            return Err(e);
        }

        // Close the config screen
        if let Err(e) = self.unmount_config_screen() {
            self.error_reporter
                .report_mount_error("ConfigScreen", "unmount", e);
        }

        // Check auth method to determine next action
        if config_data.auth_method == "connection_string" {
            log::info!(
                "Configuration saved successfully. Creating Service Bus manager with connection string."
            );
            // For connection string auth, create Service Bus manager and proceed to namespace selection
            Ok(Some(Msg::AuthActivity(
                crate::components::common::AuthActivityMsg::CreateServiceBusManager,
            )))
        } else {
            log::info!("Configuration saved successfully. Proceeding to authentication.");
            // For other auth methods, start authentication process
            Ok(Some(Msg::AuthActivity(
                crate::components::common::AuthActivityMsg::Login,
            )))
        }
    }

    pub fn write_env_file(&self, config_data: &ConfigUpdateData) -> AppResult<()> {
        let env_path = ".env";
        let mut env_content = String::new();
        let mut existing_values = std::collections::HashMap::new();

        // Read existing .env file if it exists and collect current values
        if let Ok(existing_content) = fs::read_to_string(env_path) {
            for line in existing_content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    env_content.push_str(line);
                    env_content.push('\n');
                    continue;
                }

                // Extract existing values before skipping
                if let Some(eq_pos) = line.find('=') {
                    let key = &line[..eq_pos];
                    let value = &line[eq_pos + 1..];
                    if [
                        "AZURE_AD__TENANT_ID",
                        "AZURE_AD__CLIENT_ID",
                        "AZURE_AD__CLIENT_SECRET",
                        "AZURE_AD__SUBSCRIPTION_ID",
                        "AZURE_AD__RESOURCE_GROUP",
                        "AZURE_AD__NAMESPACE",
                        "SERVICEBUS__CONNECTION_STRING",
                        "SERVICEBUS__QUEUE_NAME",
                    ]
                    .contains(&key)
                    {
                        existing_values.insert(key.to_string(), value.to_string());
                        continue; // Skip this line, we'll add it back with new or existing value
                    }
                }

                env_content.push_str(line);
                env_content.push('\n');
            }
        }

        // Helper function to write value (new or preserve existing)
        let mut write_value = |key: &str, new_value: &Option<String>| {
            if let Some(value) = new_value {
                if !value.trim().is_empty() {
                    // Quote connection string to prevent formatting issues
                    if key == "SERVICEBUS__CONNECTION_STRING" {
                        env_content.push_str(&format!("{key}=\"{value}\"\n"));
                    } else {
                        env_content.push_str(&format!("{key}={value}\n"));
                    }
                    return;
                }
            }
            // If no new value provided, preserve existing if any
            if let Some(existing_value) = existing_values.get(key) {
                if !existing_value.trim().is_empty() {
                    // Quote connection string to prevent formatting issues
                    if key == "SERVICEBUS__CONNECTION_STRING" {
                        env_content.push_str(&format!("{key}=\"{existing_value}\"\n"));
                    } else {
                        env_content.push_str(&format!("{key}={existing_value}\n"));
                    }
                }
            }
        };

        // Write values (new or preserved)
        write_value("AZURE_AD__TENANT_ID", &config_data.tenant_id);
        write_value("AZURE_AD__CLIENT_ID", &config_data.client_id);
        write_value("AZURE_AD__CLIENT_SECRET", &config_data.client_secret);
        write_value("AZURE_AD__SUBSCRIPTION_ID", &config_data.subscription_id);
        write_value("AZURE_AD__RESOURCE_GROUP", &config_data.resource_group);
        write_value("AZURE_AD__NAMESPACE", &config_data.namespace);
        write_value(
            "SERVICEBUS__CONNECTION_STRING",
            &config_data.connection_string,
        );
        write_value("SERVICEBUS__QUEUE_NAME", &config_data.queue_name);

        // Write the updated content to .env file
        fs::write(env_path, env_content).map_err(|e| {
            crate::error::AppError::Config(format!("Failed to write .env file: {e}"))
        })?;

        log::info!("Environment variables saved to .env file");
        Ok(())
    }

    fn update_config_toml(&self, config_data: &ConfigUpdateData) -> AppResult<()> {
        let config_path = "config.toml";

        // Read existing config.toml if it exists
        let mut config_content = if let Ok(content) = fs::read_to_string(config_path) {
            content
        } else {
            // Create a basic config.toml structure if it doesn't exist
            String::from("[azure_ad]\n")
        };

        // Update the auth_method in the azure_ad section
        if config_content.contains("[azure_ad]") {
            // Find the azure_ad section and update auth_method
            let lines: Vec<&str> = config_content.lines().collect();
            let mut updated_lines = Vec::new();
            let mut in_azure_ad_section = false;
            let mut auth_method_updated = false;

            for line in lines {
                let trimmed = line.trim();

                if trimmed == "[azure_ad]" {
                    in_azure_ad_section = true;
                    updated_lines.push(line.to_string());
                } else if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    // Entering a new section
                    if in_azure_ad_section && !auth_method_updated {
                        updated_lines
                            .push(format!("auth_method = \"{}\"", config_data.auth_method));
                        auth_method_updated = true;
                    }
                    in_azure_ad_section = false;
                    updated_lines.push(line.to_string());
                } else if in_azure_ad_section && trimmed.starts_with("auth_method") {
                    // Replace existing auth_method line
                    updated_lines.push(format!("auth_method = \"{}\"", config_data.auth_method));
                    auth_method_updated = true;
                } else {
                    updated_lines.push(line.to_string());
                }
            }

            // If we didn't find auth_method in azure_ad section, add it
            if in_azure_ad_section && !auth_method_updated {
                updated_lines.push(format!("auth_method = \"{}\"", config_data.auth_method));
            }

            config_content = updated_lines.join("\n");
        } else {
            // Add azure_ad section with auth_method
            config_content.push_str(&format!(
                "\n[azure_ad]\nauth_method = \"{}\"\n",
                config_data.auth_method
            ));
        }

        // Write the updated config.toml
        fs::write(config_path, config_content).map_err(|e| {
            crate::error::AppError::Config(format!("Failed to write config.toml: {e}"))
        })?;

        log::info!("Configuration saved to config.toml");
        Ok(())
    }

    fn create_auth_service(&mut self) -> AppResult<()> {
        log::info!("Recreating auth service with updated configuration");

        let config = crate::config::get_config_or_panic();

        // Only create if we're using authentication (not connection_string mode)
        if config.azure_ad().auth_method != "connection_string" {
            let auth_service = std::sync::Arc::new(
                crate::services::AuthService::new(
                    config.azure_ad(),
                    self.state_manager.tx_to_main.clone(),
                    self.http_client.clone(),
                )
                .map_err(|e| crate::error::AppError::Component(e.to_string()))?,
            );

            // Set the global auth state for server components to use
            let auth_state = auth_service.auth_state_manager();
            server::auth::set_global_auth_state(auth_state.clone());

            // Start the token refresh service with failure callback
            let tx_clone = self.state_manager.tx_to_main.clone();
            tokio::spawn(async move {
                let failure_callback =
                    std::sync::Arc::new(move |error: server::auth::TokenRefreshError| {
                        log::error!("Token refresh failed: {error}");

                        // Send notification to UI
                        let _ = tx_clone.send(crate::components::common::Msg::AuthActivity(
                            crate::components::common::AuthActivityMsg::TokenRefreshFailed(
                                error.to_string(),
                            ),
                        ));
                    });

                auth_state
                    .start_refresh_service_with_callback(Some(failure_callback))
                    .await;
            });

            // Replace the auth service in the model with the new one
            self.auth_service = Some(auth_service);

            log::info!("Auth service recreated successfully");
        } else {
            // For connection_string mode, clear the auth service
            self.auth_service = None;
            log::info!("Auth service cleared for connection_string mode");
        }

        Ok(())
    }
}
