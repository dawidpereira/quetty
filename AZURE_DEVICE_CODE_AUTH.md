# Azure Device Code Authentication Flow

## Overview

Device code authentication is a secure method for authenticating users in applications where direct user input is limited or not possible. This flow is particularly useful for CLI applications, headless services, or devices with limited input capabilities.

## How Device Code Authentication Works

### 1. Application Requests Device Code
The application initiates authentication by requesting a device code from Azure AD. This request includes:
- Client ID (Application ID)
- Tenant ID
- Scope (permissions requested)

### 2. Azure AD Returns Device Code Information
Azure AD responds with:
- **User Code**: A short code (e.g., "ABCD-1234") that the user will enter
- **Verification URI**: The URL where the user goes to authenticate (typically https://microsoft.com/devicelogin)
- **Device Code**: Used by the application to poll for authentication status
- **Interval**: How often to poll for completion
- **Expires In**: How long the codes are valid

### 3. User Authenticates
The user:
1. Opens a web browser and navigates to the verification URI
2. Enters the user code when prompted
3. Signs in with their Azure AD credentials
4. Reviews and approves the requested permissions

### 4. Application Polls for Completion
While the user is authenticating, the application polls Azure AD at the specified interval to check if authentication is complete.

### 5. Token Retrieval
Once the user completes authentication, the polling request returns:
- Access token
- Refresh token (if requested)
- ID token (if requested)
- Token expiration time

## Implementation in Quetty

### Configuration Requirements

The application needs the following configuration:
```toml
[auth.azure_ad]
flow = "device_code"
client_id = "your-application-id"
tenant_id = "your-tenant-id"
scope = "https://servicebus.azure.net/.default"
```

### Authentication States

1. **NotAuthenticated**: Initial state
2. **AwaitingDeviceCode**: After device code is requested, contains:
   - `user_code`: Code for user to enter
   - `verification_uri`: URL for authentication
   - `message`: Instructions for the user
3. **Authenticated**: Successfully authenticated with valid token
4. **Failed**: Authentication failed with error message

### User Experience

1. User initiates authentication
2. Application displays:
   - The verification URL (with option to copy or open in browser)
   - The user code (with option to copy)
   - Instructions on how to complete authentication
3. User completes authentication in their browser
4. Application automatically detects completion and proceeds

### Security Benefits

- **No Password Storage**: The application never handles user passwords
- **User Control**: Users authenticate directly with Azure AD
- **MFA Support**: Supports all Azure AD authentication methods including MFA
- **Consent Management**: Users can review and approve permissions
- **Token Refresh**: Access tokens can be refreshed without user interaction

### Token Management

- Access tokens are typically valid for 1 hour
- The application should refresh tokens before expiration
- Refresh tokens allow obtaining new access tokens without user interaction
- Tokens should be stored securely and never logged

### Best Practices

1. **Clear Instructions**: Provide clear, concise instructions to users
2. **Copy Functionality**: Allow users to easily copy the code and URL
3. **Timeout Handling**: Inform users if the authentication times out
4. **Error Messages**: Provide helpful error messages if authentication fails
5. **Secure Storage**: Never log or display tokens in plain text
6. **Token Refresh**: Implement automatic token refresh to avoid repeated authentication
