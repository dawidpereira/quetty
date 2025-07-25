# Troubleshooting Guide

This guide helps you diagnose and resolve common issues with Quetty. Issues are organized by category with step-by-step solutions.

> **📁 Directory Context**: Unless otherwise specified, run commands from the project root directory (`quetty/`).

## Quick Diagnostic Steps

Before diving into specific issues, try these general diagnostic steps:

1. **Check Version**: Ensure you're running the latest version
2. **Enable Debug Logging**: Add detailed logging for better diagnostics
3. **Test Configuration**: Verify your configuration is valid
4. **Check Network**: Ensure Azure Service Bus connectivity

### Enable Debug Logging

Add this to your `config.toml`:
```toml
[logging]
level = "debug"
file = "quetty-debug.log"
```

### Log File Locations

Quetty uses different log locations based on how it's built:

**Development builds** (debug):
- Default: `logs/quetty.log` (in project directory)
- Easy access: `tail -f logs/quetty.log`

**Production builds** (release):
- **macOS/Linux**: `~/.cache/quetty/logs/quetty.log`
- **Windows**: `%LOCALAPPDATA%/quetty/logs/quetty.log`

**Custom location**: Set `file = "path/to/your.log"` in config

Then check the log file for detailed error information.

## Authentication Issues

### Device Code Authentication

#### Issue: Device Code Expired
```
Error: The device code has expired. Please try again.
```

**Solutions:**
1. **Complete Flow Quickly**: Device codes typically expire in 15 minutes
2. **Restart Authentication**: Restart Quetty and complete the flow promptly
3. **Check Time Sync**: Ensure your system clock is accurate

#### Issue: User Canceled Authentication
```
Error: User canceled the authentication flow.
```

**Solutions:**
1. **Complete Flow**: Go through the entire browser authentication process
2. **Check URL**: Ensure you're visiting the correct verification URL
3. **Browser Issues**: Try a different browser or incognito mode

#### Issue: Invalid Tenant or Client ID
```
Error: Application with identifier 'client-id' was not found in the directory.
```

**Solutions:**
1. **Verify IDs**: Check tenant_id and client_id in configuration
2. **App Registration**: Ensure Azure AD app registration exists
3. **Permissions**: Verify app has required API permissions

### Client Credentials Authentication

#### Issue: Invalid Client Secret
```
Error: AADSTS7000215: Invalid client secret is provided.
```

**Solutions:**
1. **Check Secret**: Verify client_secret is correct and not expired
2. **Generate New Secret**: Create a new client secret in Azure AD
3. **Environment Variables**: Ensure environment variables are set correctly

#### Issue: Insufficient Permissions
```
Error: The user or administrator has not consented to use the application.
```

**Solutions:**
1. **Grant Permissions**: Assign required Service Bus permissions
2. **Admin Consent**: Have admin grant consent for the application
3. **Role Assignment**: Ensure service principal has proper roles

### Connection String Authentication

#### Issue: Invalid Connection String Format
```
Error: Invalid connection string format.
```

**Solutions:**
1. **Check Format**: Ensure connection string follows proper format:
   ```
   Endpoint=sb://namespace.servicebus.windows.net/;SharedAccessKeyName=...;SharedAccessKey=...
   ```
2. **Escape Characters**: Properly escape special characters
3. **Environment Variables**: Use environment variables for complex strings

#### Issue: Access Denied with Connection String
```
Error: UnauthorizedAccessException: Access to the path is denied.
```

**Solutions:**
1. **Policy Permissions**: Check shared access policy has required permissions (Send, Listen, Manage)
2. **Key Validity**: Ensure the shared access key is correct
3. **Namespace**: Verify the Service Bus namespace name is correct

## Network and Connectivity Issues

### General Network Issues

#### Issue: Timeout Errors
```
Error: The request timed out after 30 seconds.
```

**Solutions:**
1. **Check Connectivity**: Test internet connection to Azure
2. **Firewall Rules**: Ensure ports 443 (HTTPS) and 5671 (AMQP) are open
3. **Proxy Settings**: Configure proxy settings if behind corporate firewall
4. **Increase Timeouts**: Adjust timeout values in configuration

#### Issue: DNS Resolution Failures
```
Error: No such host is known (namespace.servicebus.windows.net)
```

**Solutions:**
1. **DNS Settings**: Check DNS configuration
2. **Network Issues**: Verify general internet connectivity
3. **VPN/Proxy**: Issues with corporate VPN or proxy
4. **Azure Status**: Check Azure Service Bus service status

### Azure Service Bus Specific

#### Issue: Service Bus Namespace Not Found
```
Error: The messaging entity 'sb://namespace.servicebus.windows.net/' could not be found.
```

**Solutions:**
1. **Namespace Name**: Verify Service Bus namespace name is correct
2. **Region**: Ensure namespace exists in the expected Azure region
3. **Subscription**: Check if namespace is in the correct subscription
4. **Permissions**: Verify access to the namespace

#### Issue: Queue Not Found
```
Error: The messaging entity 'queue-name' could not be found.
```

**Solutions:**
1. **Queue Name**: Verify queue name is spelled correctly
2. **Queue Exists**: Ensure queue exists in the namespace
3. **Permissions**: Check if you have access to the specific queue
4. **Case Sensitivity**: Queue names are case-sensitive

## Performance Issues

### Slow Loading

#### Issue: Message List Takes Long to Load
**Symptoms:** Long delays when viewing message lists

**Solutions:**
1. **Reduce Page Size**: Lower `page_size` in configuration
   ```toml
   page_size = 50  # Instead of 200
   ```
2. **Optimize Polling**: Increase `poll_timeout_ms`
   ```toml
   poll_timeout_ms = 100  # Instead of 10
   ```
3. **Network Optimization**: Check network speed to Azure
4. **Queue Size**: Very large queues may naturally be slower

#### Issue: High Memory Usage
**Symptoms:** Quetty uses excessive memory

**Solutions:**
1. **Smaller Pages**: Use smaller page sizes
2. **Restart Periodically**: Restart Quetty to clear cache
3. **Disable Statistics**: Turn off queue statistics if not needed
   ```toml
   queue_stats_display_enabled = false
   ```

### Bulk Operation Performance

#### Issue: Bulk Operations Are Slow
**Symptoms:** Bulk delete/DLQ operations take very long

**Solutions:**
1. **Reduce Batch Size**: Lower `max_batch_size`
   ```toml
   max_batch_size = 50  # Instead of 200
   ```
2. **Increase Timeouts**: Extend operation timeouts
   ```toml
   operation_timeout_secs = 600  # 10 minutes
   ```
3. **Network Issues**: Check Azure Service Bus connectivity
4. **Throttling**: Azure may be throttling your requests

## UI and Display Issues

### Terminal Compatibility

#### Issue: Garbled Text or Missing Characters
**Symptoms:** Text displays incorrectly, boxes instead of characters

**Solutions:**
1. **Unicode Support**: Ensure terminal supports Unicode/UTF-8
2. **Font Issues**: Use a font that supports all required characters
3. **Terminal Settings**: Check terminal character encoding settings
4. **Different Terminal**: Try a different terminal application

#### Issue: Colors Not Displaying
**Symptoms:** No colors or wrong colors in interface

**Solutions:**
1. **Color Support**: Verify terminal supports colors
   ```bash
   echo $TERM  # Should show color-capable terminal
   ```
2. **Terminal Settings**: Check color settings in terminal preferences
3. **Theme Issues**: Try a different theme
4. **Force Colors**: Some terminals need explicit color enabling

### Theme Issues

#### Issue: Theme Not Loading
**Symptoms:** Theme doesn't change or reverts to default

**Solutions:**
1. **Theme Files**: Verify theme files exist in correct location
2. **TOML Syntax**: Check theme file for syntax errors
3. **File Permissions**: Ensure theme files are readable
4. **Configuration**: Verify theme settings in config.toml

#### Issue: Poor Readability
**Symptoms:** Text is hard to read, poor contrast

**Solutions:**
1. **Different Theme**: Try a different built-in theme
2. **Terminal Background**: Adjust terminal background color
3. **Custom Theme**: Create or modify theme for better contrast
4. **Lighting**: Consider room lighting conditions

## Profile Management Issues

### Profile Not Found

#### Issue: Profile doesn't exist
```
Error: Profile 'myprofile' does not exist.
Available profiles: default, dev, staging
```

**Solutions:**
1. **Check Available Profiles**: List existing profiles
   ```bash
   ls ~/.config/quetty/profiles/
   ```
2. **Create Profile**: Use interactive setup to create the profile
   ```bash
   quetty --profile myprofile --setup
   ```
3. **Check Profile Name**: Verify the profile name spelling
4. **Verify Configuration Directory**:
   ```bash
   quetty --config-dir
   ```

### Invalid Profile Names

#### Issue: Security validation error
```
Error: Invalid profile name '../etc/passwd': Profile name cannot contain path separators or traversal sequences
```

**Solutions:**
1. **Use Valid Characters**: Only use letters, numbers, hyphens, and underscores
2. **No Path Separators**: Avoid `/`, `\`, `..`, `.`
3. **Examples of Valid Names**: `dev`, `staging`, `prod`, `test-env`, `my_profile`
4. **Examples of Invalid Names**: `../config`, `/absolute/path`, `test/../prod`

### Profile Permission Issues

#### Issue: Cannot read profile configuration
```
Error: Permission denied reading profile configuration
```

**Solutions:**
1. **Fix Directory Permissions**:
   ```bash
   chmod 700 ~/.config/quetty/profiles/*/
   ```
2. **Fix .env File Permissions**:
   ```bash
   chmod 600 ~/.config/quetty/profiles/*/.env
   ```
3. **Check Ownership**:
   ```bash
   ls -la ~/.config/quetty/profiles/
   ```
4. **Recreate Profile**: If permissions are severely broken, recreate the profile

### Profile Authentication Issues

#### Issue: Profile authentication fails
```
Error: Authentication failed for profile 'prod'
```

**Solutions:**
1. **Verify Credentials**: Check the `.env` file in the profile directory
   ```bash
   cat ~/.config/quetty/profiles/prod/.env
   ```
2. **Update Expired Credentials**: Client secrets and tokens can expire
3. **Check Authentication Method**: Verify the auth method is correctly set
4. **Test Interactively**: Try device code authentication to test connectivity
5. **Recreate Profile**: Use `--setup` to reconfigure authentication

### Profile Configuration Conflicts

#### Issue: Configuration not loading from profile
```
Profile settings not being applied
```

**Solutions:**
1. **Check Configuration Priority**: Profile configs override global configs
2. **Verify TOML Syntax**:
   ```bash
   # Test TOML syntax (if python3 available)
   python3 -c "import toml; print(toml.load('~/.config/quetty/profiles/dev/config.toml'))"
   ```
3. **Debug Configuration Loading**:
   ```bash
   RUST_LOG=debug quetty --profile dev
   ```
4. **Clear Configuration Cache**: Restart Quetty to reload configuration

### Profile Environment Variables

#### Issue: Environment variables not working in profiles
```
Environment variables in .env file not being loaded
```

**Solutions:**
1. **Check .env File Format**: Ensure proper format without spaces around `=`
   ```bash
   # Correct format
   AZURE_AD__TENANT_ID=your-tenant-id

   # Incorrect format
   AZURE_AD__TENANT_ID = your-tenant-id
   ```
2. **Verify File Permissions**: Ensure .env file is readable
3. **No Comments on Same Line**: Put comments on separate lines
4. **Check for Special Characters**: Quote values with special characters

### Profile Switching Issues

#### Issue: Cannot switch between profiles
```
Error switching to profile or authentication fails
```

**Solutions:**
1. **Clear Authentication Cache**: Authentication tokens may be cached
2. **Wait for Timeout**: Previous authentication may need to timeout
3. **Restart Application**: Fresh start can resolve token conflicts
4. **Check Profile Isolation**: Ensure each profile has separate credentials

## Configuration Issues

### Invalid Configuration

#### Issue: Configuration File Not Found
```
Warning: Configuration file not found, using defaults.
```

**Solutions:**
1. **Create Config**: Copy `config.default.toml` to `config.toml`
2. **Check Location**: Ensure config file is in correct directory (`config.toml`)
3. **Permissions**: Verify file is readable

#### Issue: Configuration Syntax Errors
```
Error: Failed to parse configuration: expected '='
```

**Solutions:**
1. **TOML Syntax**: Check TOML syntax is valid
2. **Quotes**: Ensure strings are properly quoted
3. **Validation Tool**: Use a TOML validator online
4. **Start Fresh**: Copy example configuration and modify gradually

### Environment Variables

#### Issue: Environment Variables Not Working
**Symptoms:** Configuration via environment variables is ignored

**Solutions:**
1. **Naming Convention**: Use correct format: `SECTION__KEY`
   ```bash
   export AZURE_AD__CLIENT_ID="your-client-id"
   ```
2. **Shell Environment**: Ensure variables are exported
3. **Case Sensitivity**: Use exact case as shown in documentation
4. **Restart Application**: Restart Quetty after setting variables

## Azure Resource Discovery Issues

### Discovery Failures

#### Issue: No Subscriptions Found
```
Error: No accessible subscriptions found.
```

**Solutions:**
1. **Authentication**: Ensure authentication is working correctly
2. **Permissions**: Check if account has access to subscriptions
3. **Azure CLI Test**: Test with `az account list`
4. **Multi-Tenant**: Check if you need to switch tenant

#### Issue: No Resource Groups Found
```
Error: No resource groups found in subscription.
```

**Solutions:**
1. **Subscription**: Verify correct subscription is selected
2. **Permissions**: Ensure read access to resource groups
3. **Region Filter**: Check if resource groups exist in expected regions
4. **Manual Configuration**: Specify resource group explicitly in config

#### Issue: No Service Bus Namespaces Found
```
Error: No Service Bus namespaces found.
```

**Solutions:**
1. **Resource Group**: Verify resource group contains Service Bus namespaces
2. **Permissions**: Check Service Bus permissions
3. **Namespace State**: Ensure namespaces are active (not deleted/disabled)
4. **Region**: Check if looking in correct Azure region

## Message Operation Issues

### Message Loading

#### Issue: No Messages Visible
**Symptoms:** Message list is empty when queue has messages

**Solutions:**
1. **Queue State**: Check if messages are in active state
2. **Peek Permissions**: Ensure you have peek/read permissions
3. **Message Lock**: Messages might be locked by other consumers
4. **Dead Letter Queue**: Check if messages are in DLQ

#### Issue: Message Content Not Displaying
**Symptoms:** Message list shows messages but content is empty

**Solutions:**
1. **Message Size**: Very large messages might not display properly
2. **Encoding Issues**: Binary content may not display correctly
3. **Permissions**: Check if you have message content read permissions
4. **Network Issues**: Content loading might be failing

### Message Operations

#### Issue: Delete Operations Fail
```
Error: Failed to delete message: MessageLockLostException
```

**Solutions:**
1. **Message Lock**: Message lock might have expired
2. **Retry Operation**: Try the delete operation again
3. **Queue Activity**: High queue activity can cause lock issues
4. **Timeout Settings**: Increase operation timeout

#### Issue: DLQ Operations Fail
```
Error: Failed to send message to DLQ
```

**Solutions:**
1. **DLQ Exists**: Ensure dead letter queue is enabled
2. **Permissions**: Check send permissions to DLQ
3. **Message State**: Message might already be processed
4. **Network Issues**: Check connectivity to Azure

## Common Error Messages

### Authentication Errors

| Error Message | Cause | Solution |
|---------------|--------|----------|
| `AADSTS50020: User account is disabled` | Account disabled | Contact administrator |
| `AADSTS65001: User not consented` | Missing consent | Grant application permissions |
| `AADSTS7000215: Invalid client secret` | Wrong/expired secret | Update client secret |

### Network Errors

| Error Message | Cause | Solution |
|---------------|--------|----------|
| `Connection timeout` | Network issues | Check internet connectivity |
| `SSL handshake failed` | Certificate issues | Check system certificates |
| `Host not found` | DNS issues | Check DNS settings |

### Service Bus Errors

| Error Message | Cause | Solution |
|---------------|--------|----------|
| `MessageLockLostException` | Message lock expired | Retry operation |
| `MessagingEntityNotFoundException` | Queue/namespace not found | Verify names |
| `UnauthorizedAccessException` | Insufficient permissions | Check permissions |

## Diagnostic Commands

### Azure CLI Testing
Test your Azure setup with these commands:

```bash
# Test authentication
az login

# List subscriptions
az account list

# Test Service Bus access
az servicebus namespace list --resource-group YOUR-RG

# List queues
az servicebus queue list --resource-group YOUR-RG --namespace-name YOUR-NS
```

### Network Testing
```bash
# Test Azure connectivity
ping login.microsoftonline.com

# Test Service Bus endpoint
nslookup your-namespace.servicebus.windows.net

# Test HTTPS connectivity
curl -I https://your-namespace.servicebus.windows.net
```

### Configuration Testing
```bash
# Validate TOML syntax
python -c "import toml; toml.load('config.toml')"

# Check environment variables
env | grep -E "(AZURE_|SERVICEBUS_|THEME_)"
```

## Getting Additional Help

### Log Analysis

1. **Enable Debug Logging**:
   ```toml
   [logging]
   level = "debug"
   file = "detailed.log"
   ```

2. **Look for Patterns**: Search for ERROR, WARN, or specific error messages

3. **Timing Issues**: Look for timeout-related messages

### Community Support

1. **GitHub Issues**: Search existing issues and create new ones
2. **Documentation**: Review all documentation files
3. **Azure Support**: For Azure-specific issues, consult Azure documentation

### Information to Include in Bug Reports

When reporting issues, include:

1. **Operating System**: Windows, macOS, Linux (version)
2. **Rust Version**: Output of `rustc --version`
3. **Quetty Version**: Git commit hash or version
4. **Configuration**: Sanitized config.toml (remove secrets)
5. **Error Messages**: Complete error messages and stack traces
6. **Steps to Reproduce**: Detailed steps to reproduce the issue
7. **Logs**: Relevant portions of debug logs
8. **Environment**: Any special network/proxy/VPN setup

### Emergency Workarounds

If Quetty is completely broken:

1. **Use Azure Portal**: Access Service Bus through Azure Portal
2. **Azure CLI**: Use `az servicebus` commands for basic operations
3. **Service Bus Explorer**: Use standalone Service Bus Explorer tool
4. **PowerShell**: Use Azure PowerShell modules

### Recovery Procedures

#### Reset Configuration
```bash
# For legacy configuration files (deprecated - use profiles instead)
cp config.toml config.toml.backup
cp config.default.toml config.toml

# For profile-based configuration
# Backup entire profile
cp -r ~/.config/quetty/profiles/myprofile ~/.config/quetty/profiles/myprofile.backup

# Reset specific profile
rm -rf ~/.config/quetty/profiles/myprofile
quetty --profile myprofile --setup

# Reset all profiles (nuclear option)
mv ~/.config/quetty/profiles ~/.config/quetty/profiles.backup
mkdir ~/.config/quetty/profiles
quetty --setup  # Recreate default profile
```

#### Clear Cache
```bash
# Remove token cache (varies by OS)
# Windows: %APPDATA%\quetty\
# macOS: ~/Library/Application Support/quetty/
# Linux: ~/.config/quetty/
```

#### Rebuild Application
```bash
# Clean rebuild
cargo clean
cargo build --release
```

## Monitoring and Prevention

### Health Checks

Regularly verify:
- Azure Service Bus connectivity
- Authentication token validity
- Configuration file integrity
- Theme file availability

### Best Practices

1. **Regular Updates**: Keep Quetty updated to latest version
2. **Configuration Backup**: Keep backups of working configurations
3. **Credential Rotation**: Regularly rotate Azure credentials
4. **Network Monitoring**: Monitor Azure Service Bus connectivity
5. **Log Rotation**: Prevent log files from growing too large

For additional help, see other documentation files or create an issue on GitHub with detailed information about your problem.
