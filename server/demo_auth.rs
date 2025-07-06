// Demo script showing the simplified authentication system
// This is not part of the compiled application, just for demonstration

use server::auth::{AuthProvider, create_auth_provider};
use server::service_bus_manager::AzureAdConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file
    dotenv::dotenv().ok();

    // Example 1: Using connection string authentication
    println!("=== Connection String Authentication ===");
    let connection_string = std::env::var("SERVICEBUS__CONNECTION_STRING").ok();
    let azure_ad_config = AzureAdConfig::default();

    let auth_provider = create_auth_provider(
        "connection_string",
        connection_string.as_deref(),
        &azure_ad_config,
    )?;

    match auth_provider.authenticate().await {
        Ok(token) => {
            println!("Auth Type: {:?}", auth_provider.auth_type());
            println!(
                "Token (first 50 chars): {}...",
                &token.token[..50.min(token.token.len())]
            );
        }
        Err(e) => {
            println!("Authentication failed: {}", e);
        }
    }

    // Example 2: Using Azure AD device code flow
    println!("\n=== Azure AD Device Code Flow ===");
    let mut azure_ad_config = AzureAdConfig::default();
    azure_ad_config.auth_method = "device_code".to_string();

    let auth_provider =
        create_auth_provider("azure_ad", connection_string.as_deref(), &azure_ad_config)?;

    println!("Device code flow would show interactive prompt here");
    match auth_provider.authenticate().await {
        Ok(token) => {
            println!("Auth succeeded!");
            println!("Auth Type: {:?}", auth_provider.auth_type());
            println!(
                "Token (first 50 chars): {}...",
                &token.token[..50.min(token.token.len())]
            );
        }
        Err(e) => {
            println!("Authentication failed: {}", e);
        }
    }

    println!("\nNote: Fallback authentication is no longer supported. If auth fails, it fails.");

    Ok(())
}
