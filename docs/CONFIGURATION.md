# Configuration Reference

This document provides a complete reference for all Quetty configuration options, including explanations, examples, and best practices.

## Profile-Based Configuration System

Quetty uses a **profile-based configuration system** that allows you to manage multiple environments (development, staging, production) with separate, isolated settings and credentials.

### Configuration Resolution Priority

#### For Profile-Based Configuration (Recommended)
1. `--config <path>` (CLI argument - overrides profile system)
2. `~/.config/quetty/profiles/<profile-name>/config.toml` (profile-specific overrides)
3. `~/.config/quetty/profiles/<profile-name>/keys.toml` (profile-specific key bindings)
4. Embedded defaults (built into binary)

#### For Legacy Configuration (Compatibility)
5. `QUETTY_CONFIG_PATH` (environment variable)
6. `./config.toml` (current directory)
7. `../config.toml` (parent directory - for source builds)
8. `~/.config/quetty/config.toml` (Linux/macOS)
9. `%APPDATA%/quetty/config.toml` (Windows)

> 💻 **CLI Usage**: See [CLI_REFERENCE.md](CLI_REFERENCE.md) for complete command-line options.

### Profile Directory Structure

```
~/.config/quetty/                    # Main configuration directory
├── profiles/                        # Profile-specific configurations
│   ├── default/                     # Default profile
│   │   ├── .env                     # Environment variables and secrets
│   │   ├── config.toml              # Optional: profile-specific settings
│   │   └── keys.toml                # Optional: custom key bindings
│   ├── dev/                         # Development profile
│   │   ├── .env                     # Dev environment credentials
│   │   └── config.toml              # Dev-specific settings (optional)
│   ├── staging/                     # Staging profile
│   │   └── .env                     # Staging environment credentials
│   └── prod/                        # Production profile
│       ├── .env                     # Production credentials
│       └── config.toml              # Production-specific settings
└── quetty.log                       # Application logs (if enabled)
```

## Profile System Overview

> 🛠️ **Profile Setup**: See [INSTALLATION.md](INSTALLATION.md) for creating and setting up profiles.

### Profile Security Features

- **Credential Isolation**: Each profile has its own `.env` file with separate credentials
- **Configuration Isolation**: Optional profile-specific `config.toml` for different settings
- **Secure Storage**: `.env` files are created with restrictive permissions (600)
- **Path Security**: Profile names are validated to prevent directory traversal attacks

## Authentication Configuration

Authentication credentials are stored in profile-specific `.env` files for security.

### Azure AD Device Code Authentication
```bash
# In ~/.config/quetty/profiles/<profile>/.env
AZURE_AD__AUTH_METHOD=device_code
AZURE_AD__TENANT_ID=your-tenant-id
AZURE_AD__CLIENT_ID=your-client-id
```

### Azure AD Client Credentials
```bash
# In ~/.config/quetty/profiles/<profile>/.env
AZURE_AD__AUTH_METHOD=client_secret
AZURE_AD__TENANT_ID=your-tenant-id
AZURE_AD__CLIENT_ID=your-client-id
AZURE_AD__CLIENT_SECRET=your-client-secret
```

### Service Bus Connection String
```bash
# In ~/.config/quetty/profiles/<profile>/.env
SERVICEBUS__CONNECTION_STRING=Endpoint=sb://namespace.servicebus.windows.net/;SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=your-key
```

### Mixed Authentication by Environment
```bash
# Development profile - Connection string (fastest setup)
SERVICEBUS__CONNECTION_STRING=Endpoint=sb://dev-namespace.servicebus.windows.net/;...

# Staging profile - Device code (interactive testing)
AZURE_AD__AUTH_METHOD=device_code
AZURE_AD__TENANT_ID=staging-tenant-id
AZURE_AD__CLIENT_ID=staging-client-id

# Production profile - Client credentials (automated)
AZURE_AD__AUTH_METHOD=client_secret
AZURE_AD__TENANT_ID=prod-tenant-id
AZURE_AD__CLIENT_ID=prod-client-id
AZURE_AD__CLIENT_SECRET=prod-client-secret
```

## Configuration Format

Quetty uses TOML format for configuration. Here's the complete structure:

```toml
# Application Configuration
page_size = 100
peek_interval = 1
poll_timeout_ms = 10
tick_interval_millis = 250

# Dead Letter Queue Configuration
dlq_receive_timeout_secs = 10
dlq_send_timeout_secs = 10
dlq_max_attempts = 10
dlq_overall_timeout_cap_secs = 60
dlq_receive_timeout_cap_secs = 10
dlq_send_timeout_cap_secs = 15
dlq_retry_delay_ms = 500

# Input/UI Configuration
crossterm_input_listener_interval_ms = 20
crossterm_input_listener_retries = 5
ui_loading_frame_duration_ms = 100

# Bulk Operations Configuration
max_batch_size = 200
max_messages_to_process = 10000
operation_timeout_secs = 300

# Queue Statistics Configuration
queue_stats_display_enabled = true
queue_stats_cache_ttl_seconds = 60

# Theme Configuration
[theme]
theme_name = "nightfox"
flavor_name = "duskfox"

# Authentication Configuration
[auth]
method = "azure_ad"

# Service Bus Configuration
[servicebus]
connection_string = ""

# Azure AD Configuration
[azure_ad]
auth_method = "device_code"
tenant_id = ""
client_id = ""
client_secret = ""
subscription_id = ""
resource_group = ""
namespace = ""
authority_host = "https://login.microsoftonline.com"
scope = "https://servicebus.azure.net/.default"

# Key Bindings Configuration
[keys]
# Global keys
key_quit = "q"
key_help = "h"
key_theme = "t"

# Navigation keys
key_down = "j"
key_up = "k"
key_next_page = "n"
key_prev_page = "p"
key_alt_next_page = "]"
key_alt_prev_page = "["

# Message actions
key_send_to_dlq = "s"
key_resend_from_dlq = "s"
key_resend_and_delete_from_dlq = "S"
key_delete_message = "x"
key_alt_delete_message = "x"

# Message details actions
key_copy_message = "c"
key_yank_message = "y"
key_send_edited_message = "s"
key_replace_edited_message = "s"

# Bulk selection keys
key_toggle_selection = " "
key_select_all_page = "a"

# Queue/Namespace selection
key_queue_select = "o"
key_namespace_select = "o"

# Message composition keys
key_toggle_dlq = "d"
key_compose_multiple = "m"
key_compose_single = "n"

# Confirmation keys
key_confirm_yes = "y"
key_confirm_no = "n"

# Logging Configuration
[logging]
level = "info"
file = "quetty.log"
```

## Application Configuration

### Core Settings

#### `page_size`
- **Type**: Integer
- **Default**: `100`
- **Range**: `1-1000`
- **Description**: Number of messages displayed per page in the message list.
- **Impact**: Higher values load more messages at once but use more memory.

```toml
page_size = 50  # Show 50 messages per page
```

#### `peek_interval`
- **Type**: Integer (seconds)
- **Default**: `1`
- **Description**: Interval for peeking at new messages in queues.
- **Impact**: Lower values provide more real-time updates but increase Azure API calls.

#### `poll_timeout_ms`
- **Type**: Integer (milliseconds)
- **Default**: `10`
- **Description**: Timeout for individual polling operations.
- **Impact**: Affects responsiveness vs. resource usage.

#### `tick_interval_millis`
- **Type**: Integer (milliseconds)
- **Default**: `250`
- **Description**: UI refresh interval for animations and loading indicators.
- **Impact**: Lower values create smoother animations but use more CPU.

### Dead Letter Queue Configuration

#### `dlq_receive_timeout_secs`
- **Type**: Integer (seconds)
- **Default**: `10`
- **Range**: `1-60`
- **Description**: Maximum time to wait when receiving messages from DLQ.

#### `dlq_send_timeout_secs`
- **Type**: Integer (seconds)
- **Default**: `10`
- **Range**: `1-60`
- **Description**: Maximum time to wait when sending messages to main queue.

#### `dlq_max_attempts`
- **Type**: Integer
- **Default**: `10`
- **Range**: `1-100`
- **Description**: Maximum retry attempts for DLQ operations.

#### `dlq_overall_timeout_cap_secs`
- **Type**: Integer (seconds)
- **Default**: `60`
- **Description**: Total timeout for complete DLQ operation including all retries.

#### `dlq_retry_delay_ms`
- **Type**: Integer (milliseconds)
- **Default**: `500`
- **Description**: Delay between DLQ operation retry attempts.

### Bulk Operations Configuration

#### `max_batch_size`
- **Type**: Integer
- **Default**: `200`
- **Range**: `1-1000`
- **Description**: Maximum number of messages processed in a single batch operation.
- **Impact**: Higher values are more efficient but may hit Azure Service Bus limits.

#### `max_messages_to_process`
- **Type**: Integer
- **Default**: `10000`
- **Description**: Maximum total messages allowed in any bulk operation.
- **Safety**: Prevents accidental processing of extremely large message sets.

#### `operation_timeout_secs`
- **Type**: Integer (seconds)
- **Default**: `300`
- **Description**: Global timeout for bulk operations.

### UI Configuration

#### `crossterm_input_listener_interval_ms`
- **Type**: Integer (milliseconds)
- **Default**: `20`
- **Description**: Polling interval for keyboard input detection.
- **Impact**: Lower values improve input responsiveness but use more CPU.

#### `ui_loading_frame_duration_ms`
- **Type**: Integer (milliseconds)
- **Default**: `100`
- **Description**: Duration between animation frames for loading indicators.

### Queue Statistics Configuration

#### `queue_stats_display_enabled`
- **Type**: Boolean
- **Default**: `true`
- **Description**: Enable/disable queue statistics display.
- **Impact**: Disabling saves API calls but removes queue metrics.

#### `queue_stats_cache_ttl_seconds`
- **Type**: Integer (seconds)
- **Default**: `60`
- **Description**: Cache duration for queue statistics. Set to `0` to disable caching.

## Theme Configuration

### `[theme]` Section

#### `theme_name`
- **Type**: String
- **Default**: `"nightfox"`
- **Options**: `"nightfox"`, `"catppuccin"`, `"quetty"`
- **Description**: Name of the theme family to use.

#### `flavor_name`
- **Type**: String
- **Default**: `"duskfox"`
- **Description**: Specific theme variant within the theme family.

**Available Themes and Flavors**:
```toml
# Nightfox themes
[theme]
theme_name = "nightfox"
flavor_name = "nightfox"     # Dark blue theme
# flavor_name = "duskfox"    # Darker variant
# flavor_name = "dawnfox"    # Light variant
# flavor_name = "nordfox"    # Nord-inspired
# flavor_name = "terafox"    # Green accent
# flavor_name = "carbonfox"  # Carbon-inspired

# Catppuccin themes
[theme]
theme_name = "catppuccin"
flavor_name = "mocha"        # Dark theme
# flavor_name = "macchiato"  # Medium dark
# flavor_name = "frappe"     # Medium light
# flavor_name = "latte"      # Light theme

# Quetty themes
[theme]
theme_name = "quetty"
flavor_name = "dark"         # Custom dark theme
# flavor_name = "light"      # Custom light theme
```

## Authentication Configuration

### Service Bus Connection String

#### `[servicebus]` Section
```toml
[servicebus]
connection_string = "Endpoint=sb://namespace.servicebus.windows.net/;SharedAccessKeyName=RootManageSharedAccessKey;SharedAccessKey=..."
```

### Authentication Configuration

#### `[auth]` Section

#### `method`
- **Type**: String
- **Options**: `"azure_ad"`, `"connection_string"`
- **Description**: Primary authentication method to use.

```toml
[auth]
method = "azure_ad"  # or "connection_string"
```

### Azure AD Configuration

#### `[azure_ad]` Section

#### `auth_method`
- **Type**: String
- **Options**: `"device_code"`, `"client_secret"`
- **Description**: Azure AD authentication flow to use.

#### Device Code Authentication
```toml
[azure_ad]
auth_method = "device_code"
tenant_id = "your-tenant-id"
client_id = "your-client-id"
```

#### Client Credentials Authentication
```toml
[azure_ad]
auth_method = "client_secret"
tenant_id = "your-tenant-id"
client_id = "your-client-id"
client_secret = "your-client-secret"
```


#### Optional Azure Resource Configuration
```toml
[azure_ad]
# ... auth config ...
subscription_id = "your-subscription-id"
resource_group = "your-resource-group"
namespace = "your-servicebus-namespace"
authority_host = "https://login.microsoftonline.com"
scope = "https://servicebus.azure.net/.default"
```

## Encryption Configuration

For enhanced security, Quetty supports encryption of sensitive authentication data including connection strings and client secrets. When encrypted data is detected on startup, the application will prompt for a master password.

### Encrypted Connection Strings

Instead of storing connection strings in plain text, you can use encrypted storage:

```bash
# Set encrypted connection string and its salt
export SERVICEBUS__ENCRYPTED_CONNECTION_STRING="<encrypted-connection-string>"
export SERVICEBUS__ENCRYPTION_SALT="<salt-for-connection-string-encryption>"
```

### Encrypted Client Secrets

For Azure AD client credentials flow, client secrets can be encrypted:

```bash
# Set encrypted client secret and its salt
export AZURE_AD__ENCRYPTED_CLIENT_SECRET="<encrypted-client-secret>"
export AZURE_AD__ENCRYPTION_SALT="<salt-for-client-secret-encryption>"
```

### Setting Up Encryption

1. **Through the UI**: Use the configuration screen to enter your credentials - they will be automatically encrypted when you provide a master password.

2. **Manual Setup**: Use the encryption utilities provided with Quetty to encrypt your credentials before setting the environment variables.

### Password Prompt Behavior

- **Startup Detection**: If encrypted data is detected, Quetty will show a password prompt on startup
- **Unified Password**: The same master password is used for all encrypted data (connection strings and client secrets)
- **Session Caching**: The password is cached for the duration of the application session
- **Error Handling**: Invalid passwords will show an error and allow retry

### Security Benefits

- **At-rest Encryption**: Credentials are encrypted using AES-256-GCM encryption
- **Key Derivation**: Uses PBKDF2 with 100,000 iterations and SHA-256
- **Unique Salts**: Each encrypted value uses a unique salt for additional security
- **Memory Safety**: Decrypted credentials are automatically zeroed from memory when no longer needed

## Key Bindings Configuration

### `[keys]` Section

All key bindings are customizable. Use single characters for most keys:

```toml
[keys]
# Global navigation
key_quit = "q"              # Quit application
key_help = "h"              # Show help
key_theme = "t"             # Theme picker

# List navigation
key_down = "j"              # Move down (vim-style)
key_up = "k"                # Move up (vim-style)
key_next_page = "n"         # Next page
key_prev_page = "p"         # Previous page
key_alt_next_page = "]"     # Alternative next page
key_alt_prev_page = "["     # Alternative previous page

# Message operations
key_delete_message = "x"         # Delete message
key_send_to_dlq = "s"           # Send to DLQ
key_resend_from_dlq = "s"       # Resend from DLQ
key_toggle_dlq = "d"            # Toggle DLQ view

# Bulk operations
key_toggle_selection = " "       # Toggle message selection (space)
key_select_all_page = "a"       # Select all on page

# Message composition
key_compose_single = "n"        # Used with Ctrl for single message
key_compose_multiple = "m"      # Compose multiple messages

# Confirmation dialogs
key_confirm_yes = "y"           # Confirm yes
key_confirm_no = "n"            # Confirm no
```

### Special Key Notations

- **Single characters**: `"q"`, `"h"`, `"j"`, `"k"`
- **Space**: `" "` (space character in quotes)
- **Special keys**: Arrow keys, Enter, Esc are handled automatically and cannot be rebound

## Logging Configuration

### `[logging]` Section

#### `level`
- **Type**: String
- **Options**: `"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"`
- **Default**: `"info"`
- **Description**: Minimum log level to output.

#### `file`
- **Type**: String (optional)
- **Description**: Path to log file. If not specified, logs go to default file (`quetty.log`).

#### `max_file_size_mb`
- **Type**: Integer (optional)
- **Default**: `10`
- **Description**: Maximum log file size in megabytes before rotation occurs.

#### `max_backup_files`
- **Type**: Integer (optional)
- **Default**: `5`
- **Description**: Maximum number of backup log files to keep when rotating.

#### `cleanup_on_startup`
- **Type**: Boolean (optional)
- **Default**: `true`
- **Description**: Whether to clean up old backup log files on application startup.

```toml
[logging]
level = "debug"              # Show debug information
file = "quetty.log"         # Log to file
max_file_size_mb = 10       # Rotate after 10MB
max_backup_files = 5        # Keep 5 backup files
cleanup_on_startup = true   # Clean old files on startup
# file = "/tmp/quetty.log"  # Absolute path
```

## Environment Variables

All configuration options can be set via environment variables using the format `SECTION__KEY`:

```bash
# Application settings
export QUETTY_PAGE_SIZE=50
export QUETTY_POLL_TIMEOUT_MS=20

# Theme settings
export THEME__THEME_NAME="catppuccin"
export THEME__FLAVOR_NAME="mocha"

# Authentication settings
export AUTH__METHOD="azure_ad"

# Azure AD settings
export AZURE_AD__AUTH_METHOD="device_code"
export AZURE_AD__TENANT_ID="your-tenant-id"
export AZURE_AD__CLIENT_ID="your-client-id"
export AZURE_AD__CLIENT_SECRET="your-client-secret"

# Azure AD encrypted settings (alternative to plain text)
export AZURE_AD__ENCRYPTED_CLIENT_SECRET="<encrypted-client-secret>"
export AZURE_AD__ENCRYPTION_SALT="<salt-for-client-secret-encryption>"

# Service Bus settings
export SERVICEBUS__CONNECTION_STRING="Endpoint=sb://..."

# Service Bus encrypted settings (alternative to plain text)
export SERVICEBUS__ENCRYPTED_CONNECTION_STRING="<encrypted-connection-string>"
export SERVICEBUS__ENCRYPTION_SALT="<salt-for-connection-string-encryption>"

# Key bindings
export KEYS__KEY_QUIT="q"
export KEYS__KEY_HELP="?"

# Logging
export LOGGING__LEVEL="debug"
export LOGGING__FILE="debug.log"
export LOGGING__MAX_FILE_SIZE_MB=50
export LOGGING__MAX_BACKUP_FILES=10
export LOGGING__CLEANUP_ON_STARTUP=false
```

## Configuration Validation

Quetty validates configuration on startup and will show helpful error messages for:

- Invalid authentication configuration
- Out-of-range numeric values
- Invalid theme names
- Conflicting key bindings
- Invalid log levels

## Configuration Examples

### Minimal Configuration
```toml
# Just authentication - everything else uses defaults
[auth]
method = "azure_ad"

[azure_ad]
auth_method = "device_code"
tenant_id = "your-tenant-id"
client_id = "your-client-id"
```

### Performance-Optimized Configuration
```toml
# Optimized for high-throughput scenarios
page_size = 500
poll_timeout_ms = 5
max_batch_size = 500
dlq_receive_timeout_secs = 30
queue_stats_cache_ttl_seconds = 300

[logging]
level = "warn"  # Reduce log verbosity
```

### Development Configuration
```toml
# Optimized for development/debugging
page_size = 10
tick_interval_millis = 100
dlq_retry_delay_ms = 1000

[theme]
theme_name = "quetty"
flavor_name = "dark"

[logging]
level = "debug"
file = "debug.log"
```

### Production Configuration
```toml
# Production-ready configuration
page_size = 100
poll_timeout_ms = 10
max_batch_size = 200
operation_timeout_secs = 600
queue_stats_cache_ttl_seconds = 120

[auth]
method = "azure_ad"

[azure_ad]
auth_method = "client_secret"

[logging]
level = "info"
file = "/var/log/quetty.log"
```

## Configuration Best Practices

### Security
- **Never commit secrets**: Use environment variables for sensitive data
- **Use encryption**: Enable encryption for connection strings and client secrets to protect credentials at rest
- **Strong master passwords**: Choose a strong master password for encryption - it protects all your encrypted credentials
- **Rotate credentials**: Regularly update client secrets and connection strings
- **Environment isolation**: Use different credentials for development, staging, and production environments

### Performance
- **Tune page size**: Balance between memory usage and loading time
- **Adjust timeouts**: Based on your network conditions and queue sizes
- **Cache statistics**: Use appropriate TTL for your monitoring needs

### Usability
- **Customize key bindings**: Match your workflow preferences
- **Choose appropriate themes**: Based on your terminal and preferences
- **Set appropriate log levels**: Debug for development, info/warn for production

### Environment-Specific Configurations

#### Development
```toml
page_size = 10              # Small pages for testing
poll_timeout_ms = 100       # More responsive
dlq_retry_delay_ms = 1000   # Longer delays for debugging

[logging]
level = "debug"
file = "dev.log"
```

#### Testing
```toml
page_size = 50
operation_timeout_secs = 60  # Shorter timeouts for faster feedback
max_batch_size = 50          # Smaller batches for testing

[logging]
level = "info"
```

#### Production
```toml
page_size = 200             # Larger pages for efficiency
operation_timeout_secs = 900 # Longer timeouts for reliability
queue_stats_cache_ttl_seconds = 300 # Cache stats longer

[logging]
level = "warn"              # Only warnings and errors
file = "/var/log/quetty.log"
```

## Profile-Specific Configuration Examples

### Development Profile (Fast Iteration)
```toml
# ~/.config/quetty/profiles/dev/config.toml
page_size = 25              # Smaller pages for quick browsing
poll_timeout_ms = 5         # Fast polling for responsive UI
peek_interval = 0.5         # Quick message updates

[theme]
theme_name = "quetty"       # Built-in theme for development
flavor_name = "dark"

[logging]
level = "debug"             # Detailed logging for development
file = "dev-quetty.log"
```

### Staging Profile (Testing Environment)
```toml
# ~/.config/quetty/profiles/staging/config.toml
page_size = 50              # Medium page size for testing
operation_timeout_secs = 120 # Moderate timeouts for testing

[theme]
theme_name = "catppuccin"   # Different theme to distinguish environment
flavor_name = "mocha"

[logging]
level = "info"              # Standard logging for staging
```

### Production Profile (Performance Optimized)
```toml
# ~/.config/quetty/profiles/prod/config.toml
page_size = 200             # Large pages for efficiency
operation_timeout_secs = 600 # Long timeouts for reliability
max_batch_size = 500        # Large batches for bulk operations
queue_stats_cache_ttl_seconds = 300 # Cache for performance

[theme]
theme_name = "nightfox"     # Production theme for distinction
flavor_name = "carbonfox"

[logging]
level = "warn"              # Minimal logging for production
file = "/var/log/quetty-prod.log"
max_file_size_mb = 50       # Larger log files for production
max_backup_files = 10       # More backup files for production
```

### Profile-Specific Key Bindings
```toml
# ~/.config/quetty/profiles/dev/keys.toml
[keys]
# Development-specific shortcuts
key_refresh = "F5"          # F5 for refresh (familiar to developers)
key_help = "F1"             # F1 for help (standard)

# ~/.config/quetty/profiles/prod/keys.toml
[keys]
# Production safety - longer key sequences
key_delete = "ctrl+shift+d" # Require modifier keys for destructive operations
key_dlq = "ctrl+shift+q"    # Require confirmation for DLQ operations
```

## Environment Variable Precedence

Environment variables in profile `.env` files take precedence over TOML configuration:

```bash
# In ~/.config/quetty/profiles/dev/.env
THEME__THEME_NAME=catppuccin     # Overrides theme.theme_name in config.toml
QUETTY_PAGE_SIZE=100             # Overrides page_size in config.toml
LOGGING__LEVEL=debug             # Overrides logging.level in config.toml
```

## Configuration Validation

Quetty validates profile names to ensure security:
- ✅ Valid: `dev`, `staging`, `prod`, `test-env`, `my_profile`
- ❌ Invalid: `../etc/passwd`, `test/../prod`, `/absolute/path`

## Troubleshooting Profiles

```bash
# Check current configuration directory
quetty --config-dir

# Verify profile exists
ls ~/.config/quetty/profiles/

# Test profile configuration
quetty --profile dev --version

# Debug configuration loading
RUST_LOG=debug quetty --profile dev
```

For troubleshooting configuration issues, see [TROUBLESHOOTING.md](TROUBLESHOOTING.md).
