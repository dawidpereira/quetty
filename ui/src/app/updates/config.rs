use crate::app::managers::state_manager::AppState;
use crate::app::model::Model;
use crate::components::common::{ComponentId, ConfigActivityMsg, ConfigUpdateData, Msg};
use crate::config::azure::{clear_master_password, set_master_password};
use crate::constants::env_vars::*;
use crate::error::AppResult;
use crate::utils::encryption::ConnectionStringEncryption;
use server::encryption::ClientSecretEncryption;
use std::env;
use std::fs;
use std::sync::Mutex;
use tuirealm::terminal::TerminalAdapter;

/// Thread-safe environment variable management
/// This provides a safe wrapper around env::set_var to prevent data races
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Safe wrapper for setting environment variables
/// This prevents data races by using a mutex lock and handles lock poisoning
fn safe_set_env_var(key: &str, value: &str) -> AppResult<()> {
    let _lock = ENV_LOCK.lock().map_err(|e| {
        crate::error::AppError::State(format!("Environment variable lock poisoned: {e}"))
    })?;
    unsafe {
        env::set_var(key, value);
    }
    Ok(())
}

/// Safe wrapper for removing environment variables
/// This prevents data races by using a mutex lock and handles lock poisoning
fn safe_remove_env_var(key: &str) -> AppResult<()> {
    let _lock = ENV_LOCK.lock().map_err(|e| {
        crate::error::AppError::State(format!("Environment variable lock poisoned: {e}"))
    })?;
    unsafe {
        env::remove_var(key);
    }
    Ok(())
}

// Constants for placeholder and error messages
const PLACEHOLDER_ENCRYPTED_CONNECTION_STRING: &str = "<<encrypted-connection-string-present>>";
const PLACEHOLDER_ENCRYPTED_CLIENT_SECRET: &str = "<<encrypted-client-secret-present>>";
const ERROR_MASTER_PASSWORD_REQUIRED: &str =
    "Master password required for connection string authentication";
const ERROR_MASTER_PASSWORD_REQUIRED_CLIENT_SECRET: &str =
    "Master password required for client secret authentication";
const DEFAULT_ENV_FILE_PATH: &str = "../.env";
const DEFAULT_CONFIG_FILE_PATH: &str = "../config.toml";

impl<T> Model<T>
where
    T: TerminalAdapter,
{
    /// Handle configuration-related messages from the UI
    ///
    /// Processes configuration save, confirm-and-proceed, and cancel operations.
    /// This includes validating configuration data, encrypting sensitive values,
    /// writing to environment variables and .env files, and determining next actions.
    ///
    /// # Arguments
    /// * `msg` - The configuration activity message to process
    ///
    /// # Returns
    /// * `Ok(Some(Msg))` - Next UI action to take
    /// * `Ok(None)` - No further action needed
    /// * `Err(AppError)` - Configuration processing failed
    pub fn update_config(&mut self, msg: ConfigActivityMsg) -> AppResult<Option<Msg>> {
        match msg {
            ConfigActivityMsg::Save(config_data) => self.handle_config_save(config_data),
            ConfigActivityMsg::ConfirmAndProceed(config_data) => {
                self.handle_config_confirm_and_proceed(config_data)
            }
            ConfigActivityMsg::Cancel => self.handle_config_cancel(),
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

        // Validate that client secret auth method requires master password for encryption
        if config_data.auth_method == "client_secret" {
            if let Some(client_secret) = &config_data.client_secret {
                if !client_secret.trim().is_empty()
                    && !client_secret.contains(PLACEHOLDER_ENCRYPTED_CLIENT_SECRET)
                    && config_data.master_password.is_none()
                {
                    log::error!(
                        "Client secret provided without master password - encryption required"
                    );
                    return Err(crate::error::AppError::Config(
                        "Master password is required when providing a client secret for encryption"
                            .to_string(),
                    ));
                }
            }
        }

        // Save sensitive data to environment variables (which will be written to .env)
        if let Some(tenant_id) = &config_data.tenant_id {
            safe_set_env_var(AZURE_AD_TENANT_ID, tenant_id)?;
        }
        if let Some(client_id) = &config_data.client_id {
            safe_set_env_var(AZURE_AD_CLIENT_ID, client_id)?;
        }
        // Note: client_secret is handled separately in encryption section below to ensure it's always encrypted
        if let Some(subscription_id) = &config_data.subscription_id {
            safe_set_env_var(AZURE_AD_SUBSCRIPTION_ID, subscription_id)?;
        }
        if let Some(resource_group) = &config_data.resource_group {
            safe_set_env_var(AZURE_AD_RESOURCE_GROUP, resource_group)?;
        }
        if let Some(namespace) = &config_data.namespace {
            safe_set_env_var(AZURE_AD_NAMESPACE, namespace)?;
        }

        // Handle connection string encryption
        if let Some(master_password) = &config_data.master_password {
            // Set master password for runtime decryption
            set_master_password(master_password.clone());

            // Check if we need to encrypt a new connection string
            if let Some(connection_string) = &config_data.connection_string {
                if !connection_string.trim().is_empty()
                    && !connection_string.contains(PLACEHOLDER_ENCRYPTED_CONNECTION_STRING)
                {
                    // New connection string provided - encrypt it
                    log::info!("New connection string provided, encrypting with master password");
                    let encryption = ConnectionStringEncryption::new();
                    match encryption.encrypt_connection_string(connection_string, master_password) {
                        Ok(encrypted) => {
                            safe_set_env_var(SERVICEBUS_ENCRYPTED_CONNECTION_STRING, &encrypted)?;
                            safe_set_env_var(
                                SERVICEBUS_ENCRYPTION_SALT,
                                &encryption.salt_base64(),
                            )?;
                        }
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
            }

            // Check if we need to encrypt a new client secret
            if let Some(client_secret) = &config_data.client_secret {
                if !client_secret.trim().is_empty()
                    && !client_secret.contains(PLACEHOLDER_ENCRYPTED_CLIENT_SECRET)
                {
                    // New client secret provided - encrypt it
                    log::info!("New client secret provided, encrypting with master password");
                    let encryption = server::encryption::ClientSecretEncryption::new();
                    match encryption.encrypt_client_secret(client_secret, master_password) {
                        Ok(encrypted) => {
                            safe_set_env_var(AZURE_AD_ENCRYPTED_CLIENT_SECRET, &encrypted)?;
                            safe_set_env_var(
                                AZURE_AD_CLIENT_SECRET_ENCRYPTION_SALT,
                                &encryption.salt_base64(),
                            )?;
                            // Remove any plain text client secret
                            safe_remove_env_var(AZURE_AD_CLIENT_SECRET)?;
                        }
                        Err(e) => {
                            log::error!("Failed to encrypt client secret: {e}");
                            return Err(crate::error::AppError::Config(format!(
                                "Client secret encryption failed: {e}"
                            )));
                        }
                    }
                } else {
                    log::info!(
                        "Using existing encrypted client secret with provided master password"
                    );
                }
            }

            // Handle decryption of encrypted client secret for runtime use
            if std::env::var(AZURE_AD_ENCRYPTED_CLIENT_SECRET).is_ok()
                && std::env::var(AZURE_AD_CLIENT_SECRET_ENCRYPTION_SALT).is_ok()
            {
                match self.decrypt_and_set_client_secret(master_password) {
                    Ok(_) => log::info!("Client secret decrypted and set for runtime use"),
                    Err(e) => {
                        log::error!("Failed to decrypt client secret: {e}");
                        return Err(e);
                    }
                }
            }

            if config_data.connection_string.is_none() {
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

        // Merge with pending config data if available
        self.merge_pending_config_data(&mut config_data);

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
                if let Err(e) =
                    self.mount_password_popup(Some(ERROR_MASTER_PASSWORD_REQUIRED.to_string()))
                {
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
        } else if config_data.auth_method == "client_secret" {
            // Client secret auth requires master password if encrypted client secret exists
            if std::env::var(AZURE_AD_ENCRYPTED_CLIENT_SECRET).is_ok() {
                // We have encrypted client secret but no password provided
                // First update the auth method in config.toml, then show password popup
                log::info!(
                    "Client secret auth selected but no password provided - updating config and showing password popup"
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

                // Show password popup for client secret decryption
                if let Err(e) = self.mount_password_popup(Some(
                    ERROR_MASTER_PASSWORD_REQUIRED_CLIENT_SECRET.to_string(),
                )) {
                    self.error_reporter
                        .report_mount_error("PasswordPopup", "mount", &e);
                    return Ok(Some(Msg::ToggleConfigScreen));
                }

                self.set_redraw(true);
                return Ok(None);
            } else {
                // No encrypted client secret exists - need to configure client secret
                // Keep the config screen open so user can enter client secret and password
                log::info!(
                    "Client secret auth selected but no encrypted client secret exists - user needs to configure"
                );
            }
        }

        // Persist configuration
        self.persist_configuration(&config_data)?;

        // Handle UI cleanup and determine next action
        self.cleanup_and_determine_next_action(&config_data)
    }

    /// Merge pending config data from config screen with current data
    fn merge_pending_config_data(&mut self, config_data: &mut ConfigUpdateData) {
        if let Some(pending_data) = &self.state_manager.pending_config_data {
            log::info!("Merging pending config data from config screen with password popup data");

            // Helper macro to merge optional fields
            macro_rules! merge_field {
                ($field:ident) => {
                    if config_data.$field.is_none() && pending_data.$field.is_some() {
                        config_data.$field = pending_data.$field.clone();
                    }
                };
            }

            // Merge all configuration fields
            merge_field!(tenant_id);
            merge_field!(client_id);
            merge_field!(client_secret);
            merge_field!(subscription_id);
            merge_field!(resource_group);
            merge_field!(namespace);
            merge_field!(connection_string);

            // Special handling for queue name with logging
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
        match config_data.auth_method.as_str() {
            "connection_string" => {
                if config_data.connection_string.is_none() {
                    // Password popup mode - validate connection string password
                    self.validate_master_password(master_password)
                } else {
                    // Config screen mode - handle connection string encryption
                    self.handle_connection_string_encryption(config_data, master_password)
                }
            }
            "client_secret" => {
                // Check if we're in password popup mode by looking at current app state
                if self.state_manager.app_state
                    == crate::app::managers::state_manager::AppState::PasswordPopup
                {
                    // Password popup mode - validate client secret password
                    self.validate_client_secret_password(master_password)
                } else if config_data.client_secret.is_some() {
                    // Config screen mode - handle client secret encryption
                    self.handle_client_secret_encryption(config_data, master_password)
                } else {
                    // No client secret provided and not in password popup mode
                    Ok(None)
                }
            }
            _ => {
                // Other auth methods don't need password/encryption handling
                Ok(None)
            }
        }
    }

    fn handle_client_secret_encryption(
        &mut self,
        config_data: &ConfigUpdateData,
        _master_password: &str,
    ) -> AppResult<Option<Msg>> {
        log::info!("Client secret auth - handling client secret encryption");

        // Directly handle the client secret configuration without recursion
        // This skips the password handling logic and goes straight to saving
        self.save_client_secret_config(config_data)
    }

    fn save_client_secret_config(
        &mut self,
        config_data: &ConfigUpdateData,
    ) -> AppResult<Option<Msg>> {
        log::info!("Saving client secret configuration directly");

        // First encrypt the client secret if provided
        if let Some(client_secret) = &config_data.client_secret {
            if let Some(master_password) = &config_data.master_password {
                if !client_secret.trim().is_empty()
                    && !client_secret.contains(PLACEHOLDER_ENCRYPTED_CLIENT_SECRET)
                    && !master_password.trim().is_empty()
                {
                    log::info!("Encrypting client secret with master password");
                    let encryption = server::encryption::ClientSecretEncryption::new();
                    match encryption.encrypt_client_secret(client_secret, master_password) {
                        Ok(encrypted) => {
                            safe_set_env_var(AZURE_AD_ENCRYPTED_CLIENT_SECRET, &encrypted)?;
                            safe_set_env_var(
                                AZURE_AD_CLIENT_SECRET_ENCRYPTION_SALT,
                                &encryption.salt_base64(),
                            )?;
                            safe_remove_env_var(AZURE_AD_CLIENT_SECRET)?;
                            log::info!("Client secret encrypted successfully");

                            // Immediately decrypt the client secret for runtime use
                            if let Some(master_password) = &config_data.master_password {
                                match self.decrypt_and_set_client_secret(master_password) {
                                    Ok(_) => log::info!(
                                        "Client secret decrypted and set for runtime use"
                                    ),
                                    Err(e) => {
                                        log::error!(
                                            "Failed to decrypt newly encrypted client secret: {e}"
                                        );
                                        return Err(e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to encrypt client secret: {e}");
                            return Err(crate::error::AppError::Config(format!(
                                "Client secret encryption failed: {e}"
                            )));
                        }
                    }
                }
            }
        }

        // Call the main config saving logic that handles file writing
        self.persist_configuration(config_data)?;

        // Handle UI cleanup and determine next action (authentication)
        self.cleanup_and_determine_next_action(config_data)
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
                            safe_set_env_var(SERVICEBUS_QUEUE_NAME, queue_name)?;
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

    fn validate_client_secret_password(&mut self, master_password: &str) -> AppResult<Option<Msg>> {
        log::info!("Password popup mode - validating client secret password");

        // Set master password for runtime decryption
        set_master_password(master_password.to_string());

        // Try to decrypt the client secret to validate the password
        match self.decrypt_and_set_client_secret(master_password) {
            Ok(_) => {
                log::info!("Password validation successful - client secret decrypted");

                // Check if we have pending config data with queue name that needs to be saved
                if let Some(pending_config) = &self.state_manager.pending_config_data {
                    if pending_config.queue_name.is_some() {
                        log::info!("Saving queue name from pending config data to .env file");

                        // Create a minimal config data with just the queue name for saving
                        let queue_config_data = crate::components::common::ConfigUpdateData {
                            auth_method: crate::utils::auth::AUTH_METHOD_CLIENT_SECRET.to_string(),
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
                            safe_set_env_var(SERVICEBUS_QUEUE_NAME, queue_name)?;
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

                // Close password popup and proceed with authentication
                log::info!(
                    "Closing password popup and proceeding with client secret authentication"
                );
                if let Err(e) = self.unmount_password_popup() {
                    self.error_reporter
                        .report_mount_error("PasswordPopup", "unmount", e);
                }

                // Recreate auth service to pick up the decrypted client secret
                log::info!("Recreating auth service with decrypted client secret");
                if let Err(e) = self.create_auth_service() {
                    log::error!(
                        "Failed to recreate auth service after client secret decryption: {e}"
                    );
                    return Err(e);
                }

                // Keep authenticating flag true since we're about to start authentication
                // It will be cleared when authentication succeeds or fails

                // Proceed with authentication
                Ok(Some(Msg::AuthActivity(
                    crate::components::common::AuthActivityMsg::Login,
                )))
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
                && !connection_string.contains(PLACEHOLDER_ENCRYPTED_CONNECTION_STRING)
            {
                // New connection string - encrypt it
                log::info!("New connection string provided, encrypting with master password");
                let encryption = ConnectionStringEncryption::new();
                match encryption.encrypt_connection_string(connection_string, master_password) {
                    Ok(encrypted) => {
                        safe_set_env_var(SERVICEBUS_ENCRYPTED_CONNECTION_STRING, &encrypted)?;
                        safe_set_env_var(SERVICEBUS_ENCRYPTION_SALT, &encryption.salt_base64())?;
                    }
                    Err(e) => {
                        log::error!("Failed to encrypt connection string: {e}");
                        return Err(crate::error::AppError::Config(format!(
                            "Connection string encryption failed: {e}"
                        )));
                    }
                }
            } else if connection_string.contains(PLACEHOLDER_ENCRYPTED_CONNECTION_STRING) {
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
                        safe_remove_env_var(SERVICEBUS_ENCRYPTED_CONNECTION_STRING)?;
                        safe_remove_env_var(SERVICEBUS_ENCRYPTION_SALT)?;
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
                safe_remove_env_var(SERVICEBUS_ENCRYPTED_CONNECTION_STRING)?;
                safe_remove_env_var(SERVICEBUS_ENCRYPTION_SALT)?;
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
        self.set_environment_variables(config_data)?;

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

    fn set_environment_variables(&self, config_data: &ConfigUpdateData) -> AppResult<()> {
        if let Some(tenant_id) = &config_data.tenant_id {
            safe_set_env_var(AZURE_AD_TENANT_ID, tenant_id)?;
        }
        if let Some(client_id) = &config_data.client_id {
            safe_set_env_var(AZURE_AD_CLIENT_ID, client_id)?;
        }
        // Note: client_secret is handled separately in encryption section to ensure it's always encrypted
        if let Some(subscription_id) = &config_data.subscription_id {
            safe_set_env_var(AZURE_AD_SUBSCRIPTION_ID, subscription_id)?;
        }
        if let Some(resource_group) = &config_data.resource_group {
            safe_set_env_var(AZURE_AD_RESOURCE_GROUP, resource_group)?;
        }
        if let Some(namespace) = &config_data.namespace {
            safe_set_env_var(AZURE_AD_NAMESPACE, namespace)?;
        }

        // Set queue name only for connection string auth
        if config_data.auth_method == crate::utils::auth::AUTH_METHOD_CONNECTION_STRING {
            if let Some(queue_name) = &config_data.queue_name {
                if !queue_name.trim().is_empty() {
                    safe_set_env_var(SERVICEBUS_QUEUE_NAME, queue_name)?;
                    log::info!("Updated queue name from config screen: '{queue_name}'");
                }
            } else {
                log::debug!("No queue name provided in config screen");
            }
        }
        Ok(())
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

    /// Write configuration data to the .env file for persistence
    ///
    /// Parses existing .env file, updates specified values, and writes back to disk.
    /// This preserves existing comments and environment variables not managed by the app.
    /// Only writes non-empty values from the configuration data.
    ///
    /// # Arguments
    /// * `config_data` - Configuration data to persist to .env file
    ///
    /// # Returns
    /// * `Ok(())` - .env file updated successfully
    /// * `Err(AppError)` - File I/O or parsing error occurred
    pub fn write_env_file(&self, config_data: &ConfigUpdateData) -> AppResult<()> {
        let env_path = DEFAULT_ENV_FILE_PATH;

        // Parse existing .env file
        let (mut env_content, existing_values) = self.parse_existing_env_file(env_path)?;

        // Update environment variables with new and existing values
        self.update_env_variables(&mut env_content, &existing_values, config_data);

        // Write the updated content to file
        self.write_env_content_to_file(env_path, &env_content)
    }

    /// Parse existing .env file and extract managed environment variables
    fn parse_existing_env_file(
        &self,
        env_path: &str,
    ) -> AppResult<(String, std::collections::HashMap<String, String>)> {
        let mut env_content = String::new();
        let mut existing_values = std::collections::HashMap::new();

        if let Ok(existing_content) = fs::read_to_string(env_path) {
            for line in existing_content.lines() {
                let line = line.trim();

                // Preserve comments and empty lines
                if line.is_empty() || line.starts_with('#') {
                    env_content.push_str(line);
                    env_content.push('\n');
                    continue;
                }

                // Extract managed environment variables
                if let Some(eq_pos) = line.find('=') {
                    let key = &line[..eq_pos];
                    let value = &line[eq_pos + 1..];

                    if Self::is_managed_env_key(key) {
                        existing_values.insert(key.to_string(), value.to_string());
                        continue; // Skip this line, we'll add it back with new or existing value
                    }
                }

                // Preserve non-managed environment variables
                env_content.push_str(line);
                env_content.push('\n');
            }
        }

        Ok((env_content, existing_values))
    }

    /// Check if an environment variable key is managed by this application
    fn is_managed_env_key(key: &str) -> bool {
        matches!(
            key,
            AZURE_AD_TENANT_ID
                | AZURE_AD_CLIENT_ID
                | AZURE_AD_CLIENT_SECRET
                | AZURE_AD_ENCRYPTED_CLIENT_SECRET
                | AZURE_AD_CLIENT_SECRET_ENCRYPTION_SALT
                | AZURE_AD_SUBSCRIPTION_ID
                | AZURE_AD_RESOURCE_GROUP
                | AZURE_AD_NAMESPACE
                | SERVICEBUS_ENCRYPTED_CONNECTION_STRING
                | SERVICEBUS_ENCRYPTION_SALT
                | SERVICEBUS_QUEUE_NAME
        )
    }

    /// Update environment variables in content with new values
    fn update_env_variables(
        &self,
        env_content: &mut String,
        existing_values: &std::collections::HashMap<String, String>,
        config_data: &ConfigUpdateData,
    ) {
        // Helper closure to write environment variable
        let mut write_env_var = |key: &str, new_value: &Option<String>| {
            self.write_env_variable(env_content, existing_values, key, new_value);
        };

        // Write configuration values
        write_env_var(AZURE_AD_TENANT_ID, &config_data.tenant_id);
        write_env_var(AZURE_AD_CLIENT_ID, &config_data.client_id);
        write_env_var(AZURE_AD_SUBSCRIPTION_ID, &config_data.subscription_id);
        write_env_var(AZURE_AD_RESOURCE_GROUP, &config_data.resource_group);
        write_env_var(AZURE_AD_NAMESPACE, &config_data.namespace);

        // Write encrypted credentials from environment
        let encrypted_connection_string =
            std::env::var(SERVICEBUS_ENCRYPTED_CONNECTION_STRING).ok();
        let encryption_salt = std::env::var(SERVICEBUS_ENCRYPTION_SALT).ok();
        let encrypted_client_secret = std::env::var(AZURE_AD_ENCRYPTED_CLIENT_SECRET).ok();
        let client_secret_encryption_salt =
            std::env::var(AZURE_AD_CLIENT_SECRET_ENCRYPTION_SALT).ok();

        write_env_var(
            SERVICEBUS_ENCRYPTED_CONNECTION_STRING,
            &encrypted_connection_string,
        );
        write_env_var(SERVICEBUS_ENCRYPTION_SALT, &encryption_salt);
        write_env_var(AZURE_AD_ENCRYPTED_CLIENT_SECRET, &encrypted_client_secret);
        write_env_var(
            AZURE_AD_CLIENT_SECRET_ENCRYPTION_SALT,
            &client_secret_encryption_salt,
        );

        // Handle queue name based on auth method
        if config_data.auth_method == crate::utils::auth::AUTH_METHOD_CONNECTION_STRING {
            write_env_var(SERVICEBUS_QUEUE_NAME, &config_data.queue_name);
        } else {
            log::debug!("Clearing queue name for non-connection-string auth method");
        }
    }

    /// Write a single environment variable to content
    fn write_env_variable(
        &self,
        env_content: &mut String,
        existing_values: &std::collections::HashMap<String, String>,
        key: &str,
        new_value: &Option<String>,
    ) {
        // Use new value if provided and not empty
        if let Some(value) = new_value {
            if !value.trim().is_empty() {
                Self::append_env_line(env_content, key, value);
                return;
            }
        }

        // Fall back to existing value if available
        if let Some(existing_value) = existing_values.get(key) {
            if !existing_value.trim().is_empty() {
                Self::append_env_line(env_content, key, existing_value);
            }
        }
    }

    /// Append an environment variable line to content with proper formatting
    fn append_env_line(env_content: &mut String, key: &str, value: &str) {
        // Quote connection strings to prevent formatting issues
        if key == SERVICEBUS_ENCRYPTED_CONNECTION_STRING {
            env_content.push_str(&format!("{key}=\"{value}\"\n"));
        } else {
            env_content.push_str(&format!("{key}={value}\n"));
        }
    }

    /// Write environment content to file
    fn write_env_content_to_file(&self, env_path: &str, env_content: &str) -> AppResult<()> {
        fs::write(env_path, env_content).map_err(|e| {
            crate::error::AppError::Config(format!("Failed to write .env file: {e}"))
        })?;

        log::info!("Environment variables saved to .env file");
        Ok(())
    }

    fn update_config_toml(&self, config_data: &ConfigUpdateData) -> AppResult<()> {
        let config_path = DEFAULT_CONFIG_FILE_PATH;

        log::info!(
            "Updating config.toml with auth_method: {}",
            config_data.auth_method
        );

        // Read existing config.toml if it exists
        let mut config_content = if let Ok(content) = fs::read_to_string(config_path) {
            log::debug!("Read existing config.toml file ({} chars)", content.len());
            content
        } else {
            // Create a basic config.toml structure if it doesn't exist
            log::debug!("Creating new config.toml file");
            String::from("[azure_ad]\n")
        };

        // Update the auth_method in the azure_ad section
        if config_content.contains("[azure_ad]") {
            log::debug!("Found [azure_ad] section in config.toml");
            // Find the azure_ad section and update auth_method
            let lines: Vec<&str> = config_content.lines().collect();
            let mut updated_lines = Vec::new();
            let mut in_azure_ad_section = false;
            let mut auth_method_updated = false;

            for (line_num, line) in lines.iter().enumerate() {
                let trimmed = line.trim();

                if trimmed == "[azure_ad]" {
                    log::debug!("Found [azure_ad] section at line {}", line_num + 1);
                    in_azure_ad_section = true;
                    updated_lines.push(line.to_string());
                } else if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    // Entering a new section
                    if in_azure_ad_section && !auth_method_updated {
                        log::debug!(
                            "Exiting [azure_ad] section at line {}, adding auth_method",
                            line_num + 1
                        );
                        updated_lines
                            .push(format!("auth_method = \"{}\"", config_data.auth_method));
                        auth_method_updated = true;
                    }
                    in_azure_ad_section = false;
                    updated_lines.push(line.to_string());
                } else if in_azure_ad_section && trimmed.starts_with("auth_method") {
                    // Replace existing auth_method line
                    log::debug!(
                        "Found auth_method line at line {}: '{}', replacing with '{}'",
                        line_num + 1,
                        trimmed,
                        config_data.auth_method
                    );
                    updated_lines.push(format!("auth_method = \"{}\"", config_data.auth_method));
                    auth_method_updated = true;
                } else {
                    updated_lines.push(line.to_string());
                }
            }

            // If we didn't find auth_method in azure_ad section, add it
            if in_azure_ad_section && !auth_method_updated {
                log::debug!("No auth_method found in [azure_ad] section, adding it");
                updated_lines.push(format!("auth_method = \"{}\"", config_data.auth_method));
            }

            config_content = updated_lines.join("\n");
            log::debug!("Updated config content ({} chars)", config_content.len());
        } else {
            // Add azure_ad section with auth_method
            log::debug!("No [azure_ad] section found, adding new section");
            config_content.push_str(&format!(
                "\n[azure_ad]\nauth_method = \"{}\"\n",
                config_data.auth_method
            ));
        }

        log::debug!("Writing updated config.toml to disk");
        // Write the updated config.toml
        fs::write(config_path, config_content).map_err(|e| {
            crate::error::AppError::Config(format!("Failed to write config.toml: {e}"))
        })?;

        log::info!(
            "Configuration saved to config.toml with auth_method: {}",
            config_data.auth_method
        );
        Ok(())
    }

    fn create_auth_service(&mut self) -> AppResult<()> {
        log::info!("Recreating auth service with updated configuration");

        let config = crate::config::get_config_or_panic();
        log::info!(
            "Current auth method from config: '{}'",
            config.azure_ad().auth_method
        );

        // Clear client secret environment variables if not using client_secret auth
        if config.azure_ad().auth_method != "client_secret" {
            log::info!(
                "Clearing client secret environment variables for non-client-secret auth method: {}",
                config.azure_ad().auth_method
            );
            safe_remove_env_var(AZURE_AD_CLIENT_SECRET)?;
        } else {
            log::info!("Keeping client secret environment variables for client_secret auth method");
        }

        // Only create if we're using authentication (not connection_string mode)
        if config.azure_ad().auth_method != "connection_string" {
            log::info!(
                "Creating auth service for method: {}",
                config.azure_ad().auth_method
            );
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

    fn decrypt_and_set_client_secret(&self, master_password: &str) -> AppResult<()> {
        // Get encrypted client secret and salt from environment
        let encrypted_client_secret =
            std::env::var(AZURE_AD_ENCRYPTED_CLIENT_SECRET).map_err(|_| {
                crate::error::AppError::Config(
                    format!("{AZURE_AD_ENCRYPTED_CLIENT_SECRET} environment variable not found")
                        .to_string(),
                )
            })?;

        let encryption_salt =
            std::env::var(AZURE_AD_CLIENT_SECRET_ENCRYPTION_SALT).map_err(|_| {
                crate::error::AppError::Config(
                    format!(
                        "{AZURE_AD_CLIENT_SECRET_ENCRYPTION_SALT} environment variable not found"
                    )
                    .to_string(),
                )
            })?;

        // Create encryption instance with the salt
        let encryption =
            ClientSecretEncryption::from_salt_base64(&encryption_salt).map_err(|e| {
                crate::error::AppError::Config(format!(
                    "Failed to initialize client secret encryption: {e}"
                ))
            })?;

        // Decrypt the client secret
        let decrypted_client_secret = encryption
            .decrypt_client_secret(&encrypted_client_secret, master_password)
            .map_err(|e| {
                crate::error::AppError::Config(format!("Failed to decrypt client secret: {e}"))
            })?;

        // Set the decrypted client secret as environment variable for runtime use
        safe_set_env_var(AZURE_AD_CLIENT_SECRET, &decrypted_client_secret)?;

        // Log first few characters for verification (security safe)
        let preview = if decrypted_client_secret.len() > 6 {
            format!("{}***", &decrypted_client_secret[..6])
        } else {
            "***".to_string()
        };
        log::info!(
            "Client secret successfully decrypted and set for runtime use (preview: {preview})"
        );
        Ok(())
    }
}
