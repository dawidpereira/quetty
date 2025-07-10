use crate::app::managers::state_manager::AppState;
use crate::app::model::Model;
use crate::components::common::{ComponentId, ConfigActivityMsg, ConfigUpdateData, Msg};
use crate::config::azure::{clear_master_password, set_master_password};
use crate::error::AppResult;
use crate::utils::encryption::ConnectionStringEncryption;
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
        }

        // Handle connection string encryption
        if let Some(master_password) = &config_data.master_password {
            // Set master password for runtime decryption
            set_master_password(master_password.clone());

            // Check if we need to encrypt a new connection string
            if let Some(connection_string) = &config_data.connection_string {
                if !connection_string.trim().is_empty()
                    && !connection_string.contains("<<encrypted-connection-string-present>>")
                {
                    // New connection string provided - encrypt it
                    log::info!("New connection string provided, encrypting with master password");
                    let encryption = ConnectionStringEncryption::new();
                    match encryption.encrypt_connection_string(connection_string, master_password) {
                        Ok(encrypted) => unsafe {
                            env::set_var("SERVICEBUS__ENCRYPTED_CONNECTION_STRING", &encrypted);
                            env::set_var("SERVICEBUS__ENCRYPTION_SALT", encryption.salt_base64());
                        },
                        Err(e) => {
                            log::error!("Failed to encrypt connection string: {e}");
                            return Err(crate::error::AppError::Config(format!(
                                "Connection string encryption failed: {e}"
                            )));
                        }
                    }
                } else {
                    log::info!(
                        "Using existing encrypted connection string with provided master password"
                    );
                }
            } else {
                log::info!("Master password set for existing encrypted connection string");
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
        log::debug!("Config/password popup cancelled");

        // Clear any pending config data
        self.state_manager.pending_config_data = None;

        // Check which component is mounted and unmount appropriately
        if self.app.mounted(&ComponentId::ConfigScreen) {
            if let Err(e) = self.unmount_config_screen() {
                self.error_reporter
                    .report_mount_error("ConfigScreen", "unmount", e);
            }
        }

        if self.app.mounted(&ComponentId::PasswordPopup) {
            if let Err(e) = self.unmount_password_popup() {
                self.error_reporter
                    .report_mount_error("PasswordPopup", "unmount", e);
            }

            // When password popup is cancelled, offer to open config screen instead
            return Ok(Some(Msg::ToggleConfigScreen));
        }

        Ok(None)
    }

    fn handle_config_confirm_and_proceed(
        &mut self,
        mut config_data: ConfigUpdateData,
    ) -> AppResult<Option<Msg>> {
        log::info!(
            "Confirming and proceeding with configuration - auth method: {}",
            config_data.auth_method
        );

        // If we have pending config data from the config screen, merge it with the password data
        if let Some(pending_data) = &self.state_manager.pending_config_data {
            log::info!("Merging pending config data from config screen with password popup data");

            // Preserve non-None values from pending config data
            if config_data.tenant_id.is_none() && pending_data.tenant_id.is_some() {
                config_data.tenant_id = pending_data.tenant_id.clone();
            }
            if config_data.client_id.is_none() && pending_data.client_id.is_some() {
                config_data.client_id = pending_data.client_id.clone();
            }
            if config_data.client_secret.is_none() && pending_data.client_secret.is_some() {
                config_data.client_secret = pending_data.client_secret.clone();
            }
            if config_data.subscription_id.is_none() && pending_data.subscription_id.is_some() {
                config_data.subscription_id = pending_data.subscription_id.clone();
            }
            if config_data.resource_group.is_none() && pending_data.resource_group.is_some() {
                config_data.resource_group = pending_data.resource_group.clone();
            }
            if config_data.namespace.is_none() && pending_data.namespace.is_some() {
                config_data.namespace = pending_data.namespace.clone();
            }
            if config_data.connection_string.is_none() && pending_data.connection_string.is_some() {
                config_data.connection_string = pending_data.connection_string.clone();
            }
            // Most importantly, preserve the queue name from the config screen
            if config_data.queue_name.is_none() && pending_data.queue_name.is_some() {
                config_data.queue_name = pending_data.queue_name.clone();
                log::info!(
                    "Preserved queue name from config screen: {:?}",
                    config_data.queue_name
                );
            }

            // Clear the pending config data after merging
            self.state_manager.pending_config_data = None;
        }

        self.log_config_data(&config_data);

        // Handle password validation and encryption
        if let Some(master_password) = &config_data.master_password {
            if let Some(msg) = self.handle_password_and_encryption(&config_data, master_password)? {
                return Ok(Some(msg));
            }
        } else if config_data.auth_method == "connection_string" {
            // Connection string auth requires master password
            let config = crate::config::get_config_or_panic();
            if config.servicebus().has_connection_string() {
                // We have encrypted connection string but no password provided
                // First update the auth method in config.toml, then show password popup
                log::info!(
                    "Connection string auth selected but no password provided - updating config and showing password popup"
                );

                // Store the config data from the config screen for later use
                self.state_manager.pending_config_data = Some(config_data.clone());

                // Update auth method in config.toml first
                if let Err(e) = self.update_config_toml(&config_data) {
                    log::error!("Failed to update config.toml: {e}");
                    return Err(e);
                }

                // Reload configuration to pick up the auth method change
                if let Err(e) = crate::config::reload_config() {
                    log::error!("Failed to reload configuration: {e}");
                    return Err(crate::error::AppError::Config(format!(
                        "Configuration reload failed: {e}"
                    )));
                }

                // Close config screen first
                if let Err(e) = self.unmount_config_screen() {
                    self.error_reporter
                        .report_mount_error("ConfigScreen", "unmount", e);
                }

                // Show password popup
                if let Err(e) = self.mount_password_popup(Some(
                    "Master password required for connection string authentication".to_string(),
                )) {
                    self.error_reporter
                        .report_mount_error("PasswordPopup", "mount", &e);
                    return Ok(Some(Msg::ToggleConfigScreen));
                }

                self.set_redraw(true);
                return Ok(None);
            } else {
                // No encrypted connection string exists - need to configure connection string
                // Keep the config screen open so user can enter connection string and password
                log::info!(
                    "Connection string auth selected but no encrypted connection string exists - user needs to configure"
                );
            }
        }

        // Persist configuration
        self.persist_configuration(&config_data)?;

        // Handle UI cleanup and determine next action
        self.cleanup_and_determine_next_action(&config_data)
    }

    fn log_config_data(&self, config_data: &ConfigUpdateData) {
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
    }

    fn handle_password_and_encryption(
        &mut self,
        config_data: &ConfigUpdateData,
        master_password: &str,
    ) -> AppResult<Option<Msg>> {
        if config_data.connection_string.is_none() {
            // Password popup mode - validate password
            self.validate_master_password(master_password)
        } else {
            // Config screen mode - handle encryption
            self.handle_connection_string_encryption(config_data, master_password)
        }
    }

    fn validate_master_password(&mut self, master_password: &str) -> AppResult<Option<Msg>> {
        log::info!("Password popup mode - validating master password");

        let config = crate::config::get_config_or_panic();
        set_master_password(master_password.to_string());

        match config.servicebus().connection_string() {
            Ok(Some(_)) => {
                log::info!("Password validation successful - connection string decrypted");

                // Check if we have pending config data with queue name that needs to be saved
                if let Some(pending_config) = &self.state_manager.pending_config_data {
                    if pending_config.queue_name.is_some() {
                        log::info!("Saving queue name from pending config data to .env file");

                        // Create a minimal config data with just the queue name for saving
                        let queue_config_data = crate::components::common::ConfigUpdateData {
                            auth_method: crate::utils::auth::AUTH_METHOD_CONNECTION_STRING
                                .to_string(),
                            tenant_id: None,
                            client_id: None,
                            client_secret: None,
                            subscription_id: None,
                            resource_group: None,
                            namespace: None,
                            connection_string: None,
                            master_password: None,
                            queue_name: pending_config.queue_name.clone(),
                        };

                        // Save queue name to environment and .env file
                        if let Some(queue_name) = &queue_config_data.queue_name {
                            unsafe {
                                std::env::set_var("SERVICEBUS__QUEUE_NAME", queue_name);
                            }
                            log::info!("Set queue name in environment: '{queue_name}'");
                        }

                        // Write to .env file
                        if let Err(e) = self.write_env_file(&queue_config_data) {
                            log::error!("Failed to write queue name to .env file: {e}");
                        } else {
                            log::info!("Queue name saved to .env file successfully");
                        }

                        // Clear pending config data since we've processed it
                        self.state_manager.pending_config_data = None;
                    }
                }

                // Reset authenticating flag since password is now valid
                self.state_manager.is_authenticating = false;
                Ok(None)
            }
            Ok(None) => {
                log::error!("Password validation failed - no connection string found");
                // Clear the password since there's no connection string to decrypt
                clear_master_password();

                self.state_manager.is_authenticating = true;

                if let Err(e) = self.unmount_password_popup() {
                    self.error_reporter
                        .report_mount_error("PasswordPopup", "unmount", e);
                }
                // Show config screen since there's no encrypted connection string configured
                Ok(Some(Msg::ToggleConfigScreen))
            }
            Err(e) => {
                log::error!("Password validation failed - decryption error: {e}");
                // Clear the incorrect password to prevent further issues
                clear_master_password();

                // Set authenticating flag to prevent namespace loading from starting
                self.state_manager.is_authenticating = true;
                if let Err(e) = self.mount_password_popup(Some(
                    "Invalid master password. Please try again or update configuration."
                        .to_string(),
                )) {
                    self.error_reporter
                        .report_mount_error("PasswordPopup", "mount", e);
                    // If we can't mount password popup, show config screen instead
                    return Ok(Some(Msg::ToggleConfigScreen));
                }

                self.set_redraw(true);

                // Return a message to stop the flow and prevent persist_configuration from running
                Ok(Some(Msg::ForceRedraw))
            }
        }
    }

    fn handle_connection_string_encryption(
        &mut self,
        config_data: &ConfigUpdateData,
        master_password: &str,
    ) -> AppResult<Option<Msg>> {
        set_master_password(master_password.to_string());

        if let Some(connection_string) = &config_data.connection_string {
            if !connection_string.trim().is_empty()
                && !connection_string.contains("<<encrypted-connection-string-present>>")
            {
                // New connection string - encrypt it
                log::info!("New connection string provided, encrypting with master password");
                let encryption = ConnectionStringEncryption::new();
                match encryption.encrypt_connection_string(connection_string, master_password) {
                    Ok(encrypted) => unsafe {
                        env::set_var("SERVICEBUS__ENCRYPTED_CONNECTION_STRING", &encrypted);
                        env::set_var("SERVICEBUS__ENCRYPTION_SALT", encryption.salt_base64());
                    },
                    Err(e) => {
                        log::error!("Failed to encrypt connection string: {e}");
                        return Err(crate::error::AppError::Config(format!(
                            "Connection string encryption failed: {e}"
                        )));
                    }
                }
            } else if connection_string.contains("<<encrypted-connection-string-present>>") {
                // Placeholder with password - verify password works with existing connection string
                log::info!("Placeholder connection string with password - verifying password");

                let config = crate::config::get_config_or_panic();
                match config.servicebus().connection_string() {
                    Ok(Some(_)) => {
                        // Password works with existing connection string - this is just password entry, not a change
                        log::info!("Password works with existing encrypted connection string");
                    }
                    Ok(None) => {
                        log::warn!("No encrypted connection string found despite placeholder");
                    }
                    Err(_) => {
                        // Password doesn't work - this is a password change
                        log::info!(
                            "Password doesn't work with existing connection string - clearing for new setup"
                        );
                        unsafe {
                            env::remove_var("SERVICEBUS__ENCRYPTED_CONNECTION_STRING");
                            env::remove_var("SERVICEBUS__ENCRYPTION_SALT");
                        }
                        return Err(crate::error::AppError::Config(
                            "Master password doesn't match existing encrypted connection string. Please enter your connection string again.".to_string()
                        ));
                    }
                }
            } else {
                log::info!(
                    "Using existing encrypted connection string with provided master password"
                );
            }
        } else {
            // Password change scenario
            let config = crate::config::get_config_or_panic();
            if config.servicebus().has_connection_string() {
                log::info!(
                    "Master password provided without connection string - assuming password change"
                );
                unsafe {
                    env::remove_var("SERVICEBUS__ENCRYPTED_CONNECTION_STRING");
                    env::remove_var("SERVICEBUS__ENCRYPTION_SALT");
                }
                return Err(crate::error::AppError::Config(
                    "Master password changed. Please enter your connection string again for security.".to_string()
                ));
            } else {
                log::info!("Master password set but no connection string available");
            }
        }

        Ok(None)
    }

    fn persist_configuration(&mut self, config_data: &ConfigUpdateData) -> AppResult<()> {
        // Set environment variables
        self.set_environment_variables(config_data);

        // Write to files
        if let Err(e) = self.write_env_file(config_data) {
            log::error!("Failed to write .env file: {e}");
            return Err(e);
        }

        if let Err(e) = self.update_config_toml(config_data) {
            log::error!("Failed to update config.toml: {e}");
            return Err(e);
        }

        // Reload configuration
        if let Err(e) = crate::config::reload_config() {
            log::error!("Failed to reload configuration: {e}");
            return Err(crate::error::AppError::Config(format!(
                "Configuration reload failed: {e}"
            )));
        }

        // Recreate auth service
        if let Err(e) = self.create_auth_service() {
            log::error!("Failed to create auth service: {e}");
            return Err(e);
        }

        Ok(())
    }

    fn set_environment_variables(&self, config_data: &ConfigUpdateData) {
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

            // Set queue name only for connection string auth
            if config_data.auth_method == crate::utils::auth::AUTH_METHOD_CONNECTION_STRING {
                if let Some(queue_name) = &config_data.queue_name {
                    if !queue_name.trim().is_empty() {
                        env::set_var("SERVICEBUS__QUEUE_NAME", queue_name);
                        log::info!("Updated queue name from config screen: '{queue_name}'");
                    }
                } else {
                    log::debug!("No queue name provided in config screen");
                }
            }
        }
    }

    fn cleanup_and_determine_next_action(
        &mut self,
        config_data: &ConfigUpdateData,
    ) -> AppResult<Option<Msg>> {
        self.state_manager.is_authenticating = false;

        // Clear any pending config data since we're proceeding with the final configuration
        self.state_manager.pending_config_data = None;

        // Cleanup UI components
        if self.app.mounted(&ComponentId::ConfigScreen) {
            if let Err(e) = self.unmount_config_screen() {
                self.error_reporter
                    .report_mount_error("ConfigScreen", "unmount", e);
            }
        }
        if self.app.mounted(&ComponentId::PasswordPopup) {
            if let Err(e) = self.app.umount(&ComponentId::PasswordPopup) {
                self.error_reporter.report_mount_error(
                    "PasswordPopup",
                    "unmount",
                    crate::error::AppError::Component(e.to_string()),
                );
            } else {
                // Update app state to prevent view errors
                if self.state_manager.app_state == AppState::PasswordPopup {
                    self.state_manager.app_state = AppState::Loading;
                }
            }
        }

        // Determine next action based on auth method
        if config_data.auth_method == crate::utils::auth::AUTH_METHOD_CONNECTION_STRING {
            log::info!(
                "Configuration saved successfully. Creating Service Bus manager with connection string."
            );
            Ok(Some(Msg::AuthActivity(
                crate::components::common::AuthActivityMsg::CreateServiceBusManager,
            )))
        } else {
            log::info!("Configuration saved successfully. Proceeding to authentication.");
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
                        "SERVICEBUS__ENCRYPTED_CONNECTION_STRING",
                        "SERVICEBUS__ENCRYPTION_SALT",
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
                    // Quote encrypted connection string to prevent formatting issues
                    if key == "SERVICEBUS__ENCRYPTED_CONNECTION_STRING" {
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
                    // Quote encrypted connection string to prevent formatting issues
                    if key == "SERVICEBUS__ENCRYPTED_CONNECTION_STRING" {
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

        // Write encrypted connection string if available
        let encrypted_connection_string =
            std::env::var("SERVICEBUS__ENCRYPTED_CONNECTION_STRING").ok();
        let encryption_salt = std::env::var("SERVICEBUS__ENCRYPTION_SALT").ok();

        write_value(
            "SERVICEBUS__ENCRYPTED_CONNECTION_STRING",
            &encrypted_connection_string,
        );
        write_value("SERVICEBUS__ENCRYPTION_SALT", &encryption_salt);

        // Write queue name only for connection string auth
        if config_data.auth_method == crate::utils::auth::AUTH_METHOD_CONNECTION_STRING {
            write_value("SERVICEBUS__QUEUE_NAME", &config_data.queue_name);
        } else {
            // Clear queue name for other auth methods by explicitly not adding it
            // This will remove the line from the .env file
            log::debug!("Clearing queue name for non-connection-string auth method");
        }

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
