use serde::Deserialize;

/// Authentication configuration for the Quetty UI application.
///
/// Defines the authentication behavior and preferences for the terminal user interface,
/// including primary authentication method selection and fallback configuration.
/// This configuration integrates with the broader authentication system to provide
/// a seamless user experience across different authentication scenarios.
///
/// # Configuration Options
///
/// - **Primary Method** - The preferred authentication method to use initially
/// - **Fallback Enabled** - Whether to allow fallback to alternative authentication methods
///
/// # Supported Authentication Methods
///
/// - `"connection_string"` - Azure Service Bus connection string with SAS authentication
/// - `"azure_ad"` - Azure Active Directory authentication (Device Code Flow)
/// - `"client_credentials"` - Azure AD Client Credentials Flow for automated scenarios
///
/// # Examples
///
/// ## Default Configuration
/// ```no_run
/// use ui::config::auth::AuthConfig;
///
/// // Use default settings (connection string with fallback enabled)
/// let config = AuthConfig::default();
/// assert_eq!(config.primary_method(), "connection_string");
/// assert_eq!(config.fallback_enabled(), true);
/// ```
///
/// ## Custom Configuration via TOML
/// ```toml
/// # In your configuration file
/// [auth]
/// primary_method = "azure_ad"
/// fallback_enabled = false
/// ```
///
/// ## Programmatic Configuration
/// ```no_run
/// use ui::config::auth::AuthConfig;
///
/// let config = AuthConfig {
///     primary_method: "azure_ad".to_string(),
///     fallback_enabled: true,
/// };
///
/// // Check configuration
/// if config.primary_method() == "azure_ad" {
///     println!("Using Azure AD authentication");
/// }
///
/// if config.fallback_enabled() {
///     println!("Fallback authentication is enabled");
/// }
/// ```
///
/// # Integration with Authentication System
///
/// This configuration works seamlessly with the authentication providers:
///
/// ```no_run
/// use ui::config::auth::AuthConfig;
///
/// let auth_config = AuthConfig::default();
///
/// // Configuration influences authentication flow
/// match auth_config.primary_method() {
///     "azure_ad" => {
///         // Initialize Azure AD authentication
///         println!("Setting up Azure AD authentication");
///     }
///     "connection_string" => {
///         // Initialize connection string authentication
///         println!("Setting up connection string authentication");
///     }
///     method => {
///         if auth_config.fallback_enabled() {
///             println!("Unknown method {}, falling back to default", method);
///         } else {
///             eprintln!("Unknown authentication method: {}", method);
///         }
///     }
/// }
/// ```
///
/// # Security Considerations
///
/// - Connection string authentication requires secure storage of connection strings
/// - Azure AD authentication provides more granular access control
/// - Client credentials should only be used in automated, secure environments
/// - Fallback mechanisms should be carefully configured based on security requirements
///
/// # Thread Safety
///
/// This configuration struct is safe to share across threads and can be cloned
/// efficiently for use in different parts of the application.
#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct AuthConfig {
    /// Primary authentication method to attempt first
    ///
    /// Supports: "connection_string", "azure_ad", "client_credentials"
    /// Default: "connection_string"
    #[serde(default = "default_primary_method")]
    pub primary_method: String,

    /// Whether to enable fallback to alternative authentication methods
    ///
    /// When enabled, allows graceful degradation to other authentication
    /// methods if the primary method fails or is unavailable.
    /// Default: true
    #[serde(default = "default_fallback_enabled")]
    pub fallback_enabled: bool,
}

fn default_primary_method() -> String {
    "connection_string".to_string()
}

fn default_fallback_enabled() -> bool {
    true
}

impl AuthConfig {
    /// Returns the configured primary authentication method.
    ///
    /// # Returns
    ///
    /// A string slice containing the primary authentication method identifier
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ui::config::auth::AuthConfig;
    ///
    /// let config = AuthConfig::default();
    /// match config.primary_method() {
    ///     "azure_ad" => println!("Using Azure AD authentication"),
    ///     "connection_string" => println!("Using connection string authentication"),
    ///     method => println!("Using authentication method: {}", method),
    /// }
    /// ```
    #[allow(dead_code)]
    pub fn primary_method(&self) -> &str {
        &self.primary_method
    }

    /// Returns whether fallback authentication is enabled.
    ///
    /// # Returns
    ///
    /// `true` if fallback authentication is enabled, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ui::config::auth::AuthConfig;
    ///
    /// let config = AuthConfig::default();
    /// if config.fallback_enabled() {
    ///     println!("Fallback authentication is available");
    /// } else {
    ///     println!("Only primary authentication method will be used");
    /// }
    /// ```
    #[allow(dead_code)]
    pub fn fallback_enabled(&self) -> bool {
        self.fallback_enabled
    }
}
