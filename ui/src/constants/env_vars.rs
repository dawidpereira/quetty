/// Environment variable name constants
/// This module provides centralized constants for all environment variable names
/// used throughout the application to reduce string duplication.
//
// Azure AD environment variables
pub const AZURE_AD_TENANT_ID: &str = "AZURE_AD__TENANT_ID";
pub const AZURE_AD_CLIENT_ID: &str = "AZURE_AD__CLIENT_ID";
pub const AZURE_AD_CLIENT_SECRET: &str = "AZURE_AD__CLIENT_SECRET";
pub const AZURE_AD_ENCRYPTED_CLIENT_SECRET: &str = "AZURE_AD__ENCRYPTED_CLIENT_SECRET";
pub const AZURE_AD_CLIENT_SECRET_ENCRYPTION_SALT: &str = "AZURE_AD__CLIENT_SECRET_ENCRYPTION_SALT";
pub const AZURE_AD_SUBSCRIPTION_ID: &str = "AZURE_AD__SUBSCRIPTION_ID";
pub const AZURE_AD_RESOURCE_GROUP: &str = "AZURE_AD__RESOURCE_GROUP";
pub const AZURE_AD_NAMESPACE: &str = "AZURE_AD__NAMESPACE";

// Service Bus environment variables
pub const SERVICEBUS_ENCRYPTED_CONNECTION_STRING: &str = "SERVICEBUS__ENCRYPTED_CONNECTION_STRING";
pub const SERVICEBUS_ENCRYPTION_SALT: &str = "SERVICEBUS__ENCRYPTION_SALT";
pub const SERVICEBUS_QUEUE_NAME: &str = "SERVICEBUS__QUEUE_NAME";
