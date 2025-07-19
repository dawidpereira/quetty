use crate::config::setup::initialize_config_dir;
use std::io::{self, Write};

/// Interactive setup wizard for first-time configuration
pub struct SetupWizard;

impl SetupWizard {
    /// Run the interactive setup wizard for specified profile
    pub fn run_for_profile(profile_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Validate profile name first for security
        if let Err(validation_error) = crate::config::validate_profile_name(profile_name) {
            return Err(format!("Invalid profile name: {validation_error}").into());
        }

        println!("🎯 Welcome to Quetty Setup Wizard!");
        if profile_name == "default" {
            println!("This will help you create your initial configuration.\n");
        } else {
            println!("Setting up profile: {profile_name}\n");
        }

        // Get safe profile directory path (validation already done above)
        let config_dir = initialize_config_dir()?;
        let profile_dir = config_dir.join("profiles").join(profile_name); // Safe after validation

        // Check if .env already exists in specified profile
        let env_path = profile_dir.join(".env");
        if env_path.exists() {
            print!(
                "Profile '{}' configuration already exists at: {}\nDo you want to update it? (y/N): ",
                profile_name,
                env_path.display()
            );
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if !input.trim().to_lowercase().starts_with('y') {
                println!("Setup cancelled. Existing profile '{profile_name}' preserved.");
                return Ok(());
            }
        }

        println!(
            "✓ Configuration directory created: {}",
            config_dir.display()
        );

        // Ask user about authentication method
        let auth_method = Self::prompt_auth_method()?;

        // If connection string auth, prompt for the connection string
        let connection_string = if auth_method == "connection_string" {
            Some(Self::prompt_connection_string()?)
        } else {
            None
        };

        // Create profile directory if it doesn't exist
        std::fs::create_dir_all(&profile_dir)?;

        // Write authentication method to .env file in specified profile
        let env_path = profile_dir.join(".env");
        let env_content = Self::generate_env_content(&auth_method, connection_string.as_deref())?;
        std::fs::write(&env_path, env_content)?;

        // Set secure permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&env_path, permissions)?;
        }

        println!("\n✅ Setup complete!");
        println!(
            "📁 Profile '{}' configuration saved to: {}",
            profile_name,
            env_path.display()
        );
        println!("🔧 Configuration uses embedded defaults with your custom authentication.");

        if auth_method == "client_secret" {
            println!("\n⚠️  Important: Add your Azure AD credentials to the .env file:");
            println!("   AZURE_AD__TENANT_ID=your-tenant-id");
            println!("   AZURE_AD__CLIENT_ID=your-client-id");
            println!("   AZURE_AD__CLIENT_SECRET=your-client-secret");
        } else if auth_method == "device_code" {
            println!(
                "\n📝 Note: Device code authentication will prompt you to sign in when you start the app."
            );
        } else if auth_method == "connection_string" {
            println!("\n✨ Connection string configured successfully!");
            println!("🔗 Your Service Bus connection is ready to use.");
            println!("💡 No additional authentication setup required.");
        }

        println!("\n🚀 Run 'quetty' to start the application!");

        // Invalidate profile cache since we created a new profile
        crate::config::invalidate_profile_cache();

        Ok(())
    }

    /// Prompt user for authentication method
    fn prompt_auth_method() -> Result<String, Box<dyn std::error::Error>> {
        println!("Choose your authentication method:");
        println!("1. Device Code Flow (recommended for development)");
        println!("   - Interactive browser-based authentication");
        println!("   - No client secret required");
        println!("   - Great for personal use");
        println!();
        println!("2. Client Secret Flow (recommended for automation)");
        println!("   - Service principal authentication");
        println!("   - Requires client secret");
        println!("   - Best for CI/CD and automated scripts");
        println!();
        println!("3. Connection String (fastest setup)");
        println!("   - Direct Service Bus connection");
        println!("   - No Azure AD setup required");
        println!("   - Get from Azure Portal → Service Bus → Shared access policies");
        println!();

        loop {
            print!("Select option (1, 2, or 3): ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            match input.trim() {
                "1" => return Ok("device_code".to_string()),
                "2" => return Ok("client_secret".to_string()),
                "3" => return Ok("connection_string".to_string()),
                _ => println!("Please enter 1, 2, or 3."),
            }
        }
    }

    /// Prompt user for Service Bus connection string
    fn prompt_connection_string() -> Result<String, Box<dyn std::error::Error>> {
        println!("\n📋 Enter your Service Bus connection string:");
        println!("💡 You can find this in the Azure Portal:");
        println!("   1. Go to your Service Bus namespace");
        println!("   2. Select 'Shared access policies'");
        println!("   3. Click on a policy (e.g., 'RootManageSharedAccessKey')");
        println!("   4. Copy the 'Primary Connection String'");
        println!();

        loop {
            print!("Connection string: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let connection_string = input.trim().to_string();

            if connection_string.is_empty() {
                println!("Connection string cannot be empty. Please try again.");
                continue;
            }

            // Basic validation - connection string should contain required components
            if !connection_string.contains("Endpoint=sb://")
                || !connection_string.contains("SharedAccessKeyName=")
                || !connection_string.contains("SharedAccessKey=")
            {
                println!("⚠️  Invalid connection string format. It should contain:");
                println!("   - Endpoint=sb://...");
                println!("   - SharedAccessKeyName=...");
                println!("   - SharedAccessKey=...");
                println!("Please try again.");
                continue;
            }

            return Ok(connection_string);
        }
    }

    /// Generate .env content based on selected auth method
    fn generate_env_content(
        auth_method: &str,
        connection_string: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut env_content = String::new();
        env_content.push_str("# Environment variables for default profile\n");
        env_content.push_str("# SECRETS AND AUTHENTICATION ONLY\n");
        env_content.push_str(
            "# For other settings, create config.toml or keys.toml in this directory\n\n",
        );

        if auth_method == "connection_string" {
            // Connection string authentication
            env_content.push_str("# Connection string authentication\n");
            if let Some(conn_str) = connection_string {
                env_content.push_str(&format!("SERVICEBUS__CONNECTION_STRING={conn_str}\n\n"));
            } else {
                env_content.push_str("# SERVICEBUS__CONNECTION_STRING=your-connection-string\n\n");
            }
        } else {
            // Azure AD authentication methods
            env_content.push_str(&format!("# Authentication method: {auth_method}\n"));
            env_content.push_str(&format!("AZURE_AD__AUTH_METHOD={auth_method}\n\n"));

            if auth_method == "device_code" {
                env_content.push_str("# Device code flow - no additional credentials needed\n");
                env_content.push_str("# The app will prompt you to sign in interactively\n\n");
            } else if auth_method == "client_secret" {
                env_content.push_str("# Client secret flow - add your Azure AD credentials:\n");
                env_content.push_str("# AZURE_AD__TENANT_ID=your-tenant-id\n");
                env_content.push_str("# AZURE_AD__CLIENT_ID=your-client-id\n");
                env_content.push_str("# AZURE_AD__CLIENT_SECRET=your-client-secret\n\n");
            }

            // Add commented connection string option for Azure AD methods
            env_content.push_str("# Alternative: Service Bus connection string (if switching to connection string auth):\n");
            env_content.push_str("# SERVICEBUS__CONNECTION_STRING=your-connection-string\n");
        }

        env_content.push_str(
            "# SERVICEBUS__ENCRYPTED_CONNECTION_STRING=your-encrypted-connection-string\n",
        );
        env_content.push_str("# SERVICEBUS__ENCRYPTION_SALT=your-encryption-salt\n\n");

        env_content.push_str("# Optional: Azure resource information (if not auto-discovered)\n");
        env_content.push_str("# AZURE_AD__SUBSCRIPTION_ID=your-subscription-id\n");
        env_content.push_str("# AZURE_AD__RESOURCE_GROUP=your-resource-group\n");
        env_content.push_str("# AZURE_AD__NAMESPACE=your-servicebus-namespace\n");

        Ok(env_content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_env_content_device_code() {
        let env_content = SetupWizard::generate_env_content("device_code", None).unwrap();
        assert!(env_content.contains("AZURE_AD__AUTH_METHOD=device_code"));
        assert!(env_content.contains("Device code flow"));
    }

    #[test]
    fn test_generate_env_content_client_secret() {
        let env_content = SetupWizard::generate_env_content("client_secret", None).unwrap();
        assert!(env_content.contains("AZURE_AD__AUTH_METHOD=client_secret"));
        assert!(env_content.contains("AZURE_AD__CLIENT_SECRET"));
    }

    #[test]
    fn test_generate_env_content_connection_string() {
        let test_conn_str = "Endpoint=sb://test.servicebus.windows.net/;SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=testkey";
        let env_content =
            SetupWizard::generate_env_content("connection_string", Some(test_conn_str)).unwrap();
        assert!(env_content.contains("SERVICEBUS__CONNECTION_STRING="));
        assert!(env_content.contains(test_conn_str));
        assert!(env_content.contains("Connection string authentication"));
    }

    #[test]
    fn test_generate_env_content_connection_string_none() {
        let env_content = SetupWizard::generate_env_content("connection_string", None).unwrap();
        assert!(env_content.contains("# SERVICEBUS__CONNECTION_STRING=your-connection-string"));
        assert!(env_content.contains("Connection string authentication"));
    }
}
