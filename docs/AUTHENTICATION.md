# Authentication Guide

Quetty supports multiple authentication methods for Azure Service Bus, allowing you to choose the best approach for your environment and security requirements.

## Authentication Methods Overview

| Method | Use Case | Security | Complexity |
|--------|----------|----------|------------|
| **Device Code** | Interactive CLI usage | High | Low |
| **Client Credentials** | Automated/service usage | High | Medium |
| **Connection String** | Quick setup, testing | Medium | Low |

## Device Code Authentication (Recommended)

Best for interactive use, provides the highest security with minimal setup complexity.

### Prerequisites
- Azure AD tenant access
- Azure AD app registration
- User account with Service Bus permissions

### Setup Steps

1. **Create Azure AD App Registration**:
   ```bash
   # Using Azure CLI
   az ad app create --display-name "Quetty CLI" \
     --public-client-redirect-uris "http://localhost"
   ```

2. **Configure API Permissions**:
   - Navigate to Azure Portal → Azure Active Directory → App registrations
   - Select your app → API permissions → Add a permission
   - Add `Azure Service Management` → Delegated permissions → `user_impersonation`
   - Add `Service Bus` → Delegated permissions → `user_impersonation`

3. **Configure Quetty**:
   ```toml
   [azure_ad]
   auth_method = "device_code"
   tenant_id = "your-tenant-id"
   client_id = "your-app-client-id"
   # Note: No client_secret needed for device code flow
   ```

4. **First Authentication**:
   - Run Quetty
   - Follow the on-screen instructions to authenticate
   - Visit the provided URL and enter the device code
   - Complete authentication in your browser

### Configuration Options
```toml
[azure_ad]
auth_method = "device_code"
tenant_id = "12345678-1234-1234-1234-123456789012"
client_id = "87654321-4321-4321-4321-210987654321"

# Optional: Override default authority (for sovereign clouds)
# authority_host = "https://login.microsoftonline.com"

# Optional: Override default scope
# scope = "https://servicebus.azure.net/.default"

# Optional: Azure resource discovery (auto-detected if not specified)
# subscription_id = "your-subscription-id"
# resource_group = "your-resource-group"
# namespace = "your-servicebus-namespace"
```

### How Device Code Flow Works

1. **Initiate Authentication**: Quetty requests a device code from Azure AD
2. **Display Instructions**: User sees verification URL and user code
3. **User Authentication**: User opens browser, enters code, signs in
4. **Token Retrieval**: Quetty polls Azure AD and receives access token
5. **Token Storage**: Token is cached for future use with automatic refresh

## Client Credentials Authentication

Best for automated scenarios, service accounts, and CI/CD pipelines.

### Prerequisites
- Azure AD app registration with client secret
- Service principal with Service Bus permissions

### Setup Steps

1. **Create App Registration with Secret**:
   ```bash
   # Create app registration
   az ad app create --display-name "Quetty Service"

   # Create service principal
   az ad sp create --id <app-id>

   # Create client secret
   az ad app credential reset --id <app-id> --display-name "QuettySecret"
   ```

2. **Assign Service Bus Permissions**:
   ```bash
   # Assign Azure Service Bus Data Owner role
   az role assignment create \
     --assignee <service-principal-id> \
     --role "Azure Service Bus Data Owner" \
     --scope "/subscriptions/<subscription-id>/resourceGroups/<rg-name>/providers/Microsoft.ServiceBus/namespaces/<namespace-name>"
   ```

3. **Configure Quetty**:
   ```toml
   [azure_ad]
   auth_method = "client_secret"
   tenant_id = "your-tenant-id"
   client_id = "your-app-client-id"
   client_secret = "your-client-secret"
   subscription_id = "your-subscription-id"
   resource_group = "your-resource-group"
   namespace = "your-servicebus-namespace"
   ```

### Environment Variables (Recommended for Secrets)
```bash
export AZURE_AD__TENANT_ID="your-tenant-id"
export AZURE_AD__CLIENT_ID="your-client-id"
export AZURE_AD__CLIENT_SECRET="your-client-secret"
export AZURE_AD__SUBSCRIPTION_ID="your-subscription-id"
export AZURE_AD__RESOURCE_GROUP="your-resource-group"
export AZURE_AD__NAMESPACE="your-namespace"
```

## Connection String Authentication

Simplest setup for testing and development environments.

### Prerequisites
- Azure Service Bus namespace
- Connection string with appropriate permissions

### Setup Steps

1. **Get Connection String from Azure Portal**:
   - Navigate to Service Bus namespace
   - Settings → Shared access policies
   - Select or create a policy with required permissions
   - Copy the connection string

2. **Configure Quetty**:
   ```toml
   [servicebus]
   connection_string = "Endpoint=sb://namespace.servicebus.windows.net/;SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=..."
   ```

3. **Using Environment Variables** (Recommended):
   ```bash
   export SERVICEBUS__CONNECTION_STRING="Endpoint=sb://..."
   ```

### Connection String Format
```
Endpoint=sb://<namespace>.servicebus.windows.net/;SharedAccessKeyName=<key-name>;SharedAccessKey=<key-value>
```


## Authentication Configuration Reference

### Complete Configuration Example
```toml
# Choose one authentication method

# Method 1: Device Code (Interactive)
[azure_ad]
auth_method = "device_code"
tenant_id = "12345678-1234-1234-1234-123456789012"
client_id = "87654321-4321-4321-4321-210987654321"

# Method 2: Client Credentials (Automated)
# [azure_ad]
# auth_method = "client_secret"
# tenant_id = "12345678-1234-1234-1234-123456789012"
# client_id = "87654321-4321-4321-4321-210987654321"
# client_secret = "your-client-secret"

# Method 3: Connection String (Simple)
# [servicebus]
# connection_string = "Endpoint=sb://..."


# Optional Azure resource information (auto-discovered if not specified)
subscription_id = "11111111-1111-1111-1111-111111111111"
resource_group = "my-resource-group"
namespace = "my-servicebus-namespace"

# Optional authority customization (for sovereign clouds)
authority_host = "https://login.microsoftonline.com"  # Default
scope = "https://servicebus.azure.net/.default"       # Default
```

### Environment Variable Reference
```bash
# Azure AD Configuration
AZURE_AD__TENANT_ID="..."
AZURE_AD__CLIENT_ID="..."
AZURE_AD__CLIENT_SECRET="..."    # Client credentials only
AZURE_AD__SUBSCRIPTION_ID="..."
AZURE_AD__RESOURCE_GROUP="..."
AZURE_AD__NAMESPACE="..."
AZURE_AD__AUTHORITY_HOST="..."
AZURE_AD__SCOPE="..."

# Service Bus Configuration
SERVICEBUS__CONNECTION_STRING="..."

# Authentication method selection
AZURE_AD__AUTH_METHOD="device_code"  # or "client_secret"
```

## Authentication Best Practices

### Security
- **Never commit secrets**: Use environment variables or Azure Key Vault
- **Principle of least privilege**: Grant only necessary Service Bus permissions
- **Rotate credentials**: Regularly rotate client secrets and connection strings

### Permissions Required

#### For Azure Navigation & Resource Discovery (Minimal Permissions)
When using Azure AD authentication with interactive Azure resource navigation (subscription → resource group → namespace → queue), you need these role assignments:

1. **Reader** role at **Subscription level** (Required for Azure navigation)
   - Scope: `/subscriptions/{subscription-id}`
   - Purpose: Enables discovery of resource groups, namespaces, and Azure resources
   - **Critical**: Without this, resource group discovery will return empty results even if other operations work

2. **Contributor** role at **Namespace level** (Required for Service Bus operations)
   - Scope: `/subscriptions/{subscription-id}/resourceGroups/{resource-group}/providers/Microsoft.ServiceBus/namespaces/{namespace}`
   - Purpose: Access to Service Bus queues and connection strings

3. **Azure Service Bus Data Receiver** role at **Namespace level** (Required for message operations)
   - Scope: `/subscriptions/{subscription-id}/resourceGroups/{resource-group}/providers/Microsoft.ServiceBus/namespaces/{namespace}`
   - Purpose: Read messages from queues

4. **Azure Service Bus Data Sender** role at **Namespace level** (Required for message operations)
   - Scope: `/subscriptions/{subscription-id}/resourceGroups/{resource-group}/providers/Microsoft.ServiceBus/namespaces/{namespace}`
   - Purpose: Send messages to queues

#### Alternative: Simplified Setup (No Azure Navigation)
If you don't need interactive Azure resource navigation and will provide all Azure resource details in configuration:

- **Contributor** role at **Namespace level**
- **Azure Service Bus Data Receiver** role at **Namespace level**
- **Azure Service Bus Data Sender** role at **Namespace level**
- Provide `subscription_id`, `resource_group`, and `namespace` in configuration

#### Role Assignment Commands
```bash
# Add Reader role at subscription level (for Azure navigation)
az role assignment create \
  --assignee <service-principal-id> \
  --role "Reader" \
  --scope "/subscriptions/{subscription-id}"

# Add Contributor role at namespace level (for Service Bus operations)
az role assignment create \
  --assignee <service-principal-id> \
  --role "Contributor" \
  --scope "/subscriptions/{subscription-id}/resourceGroups/{resource-group}/providers/Microsoft.ServiceBus/namespaces/{namespace}"

# Add Service Bus Data Receiver role at namespace level (for message operations)
az role assignment create \
  --assignee <service-principal-id> \
  --role "Azure Service Bus Data Receiver" \
  --scope "/subscriptions/{subscription-id}/resourceGroups/{resource-group}/providers/Microsoft.ServiceBus/namespaces/{namespace}"

# Add Service Bus Data Sender role at namespace level (for message operations)
az role assignment create \
  --assignee <service-principal-id> \
  --role "Azure Service Bus Data Sender" \
  --scope "/subscriptions/{subscription-id}/resourceGroups/{resource-group}/providers/Microsoft.ServiceBus/namespaces/{namespace}"
```

### Token Management
- **Automatic refresh**: Quetty automatically refreshes tokens before expiration
- **Token caching**: Tokens are cached to avoid repeated authentication
- **Cache location**: Tokens stored in OS-specific secure storage

## Troubleshooting Authentication

### Common Issues

#### Device Code Flow Issues
```
Error: Device code expired
Solution: Complete authentication within the time limit (usually 15 minutes)
```

```
Error: User canceled authentication
Solution: Restart Quetty and complete the authentication flow
```

#### Client Credentials Issues
```
Error: Invalid client secret
Solution: Verify client secret is correct and not expired
```

```
Error: Insufficient permissions
Solution: Ensure service principal has proper Service Bus role assignments
```

#### Connection String Issues
```
Error: Invalid connection string format
Solution: Verify connection string format and escape special characters
```

```
Error: Access denied
Solution: Check shared access policy permissions (Send, Listen, Manage)
```

#### Azure Navigation Issues
```
Error: Resource group discovery returns 0 results (empty)
Symptoms: Azure authentication succeeds, queue operations work, but no resource groups found
Root Cause: Missing Reader role at subscription level
Solution: Add Reader role assignment at subscription scope
```

```bash
# Fix: Add subscription-level Reader role
az role assignment create \
  --assignee <service-principal-id> \
  --role "Reader" \
  --scope "/subscriptions/{subscription-id}"
```

```
Error: "No resource groups found - this may be due to limited permissions"
Symptoms: Interactive navigation shows empty resource group picker
Root Cause: Service principal has namespace-level permissions but lacks subscription-level read access
Solution: The service principal needs both namespace-level Service Bus permissions AND subscription-level Reader permissions
```


### Debugging Authentication

1. **Enable debug logging**:
   ```toml
   [logging]
   level = "debug"
   file = "quetty.log"
   ```

2. **Check token cache**:
   - **Windows**: `%APPDATA%\quetty\tokens`
   - **macOS**: `~/Library/Application Support/quetty/tokens`
   - **Linux**: `~/.config/quetty/tokens`

3. **Test with Azure CLI**:
   ```bash
   # Test Azure AD authentication
   az login
   az servicebus queue list --resource-group <rg> --namespace-name <ns>

   # Test connection string
   az servicebus queue list --connection-string "<connection-string>"
   ```

4. **Validate app registration**:
   ```bash
   # Check app registration
   az ad app show --id <client-id>

   # Check service principal
   az ad sp show --id <client-id>

   # Check role assignments
   az role assignment list --assignee <principal-id>
   ```

## Multiple Authentication Methods

Quetty supports fallback authentication for resilience:

```toml
[auth]
primary_method = "azure_ad"
fallback_enabled = true

[auth.azure_ad]
flow = "device_code"
tenant_id = "..."
client_id = "..."

[auth.connection_string]
value = "Endpoint=sb://..."
```

This configuration will try Azure AD first, then fall back to connection string if Azure AD fails.

## Sovereign Cloud Support

For Azure Government, Azure China, or other sovereign clouds:

```toml
[azure_ad]
auth_method = "device_code"
authority_host = "https://login.microsoftonline.us"  # Azure Government
# authority_host = "https://login.chinacloudapi.cn"   # Azure China
scope = "https://servicebus.azure.us/.default"       # Azure Government
# scope = "https://servicebus.azure.cn/.default"      # Azure China
```

For more help with authentication issues, see [TROUBLESHOOTING.md](TROUBLESHOOTING.md).
