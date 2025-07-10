# Installation Guide

This guide covers the complete installation and setup process for Quetty, from system requirements to first-time configuration.

## System Requirements

### Minimum Requirements
- **Operating System**: Linux, macOS, or Windows
- **Rust**: 1.70 or later (latest stable recommended)
- **Memory**: 100MB available RAM
- **Network**: Internet access for Azure Service Bus connectivity

### Recommended Requirements
- **Rust**: Latest stable version via [rustup](https://rustup.rs/)
- **Terminal**: Modern terminal with Unicode support (e.g., Terminal.app, iTerm2, Windows Terminal, Alacritty, Ghostty)
- **Azure**: Active Azure subscription with Service Bus namespace

## Installation

### Option 1: Build from Source (Current)

1. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. **Clone the repository**:
   ```bash
   git clone https://github.com/yourusername/quetty.git
   cd quetty
   ```

3. **Build the application**:
   ```bash
   # For development
   cargo build

   # For optimized release build (recommended)
   cargo build --release
   ```

4. **Run Quetty**:
   ```bash
   cd ui
   cargo run --release
   ```

### Option 2: Binary Releases (Coming Soon)

Pre-built binaries will be available for:
- Linux (x86_64, ARM64)
- macOS (Intel, Apple Silicon)
- Windows (x86_64)

> 📦 **Note**: Binary releases are planned for the next version. Currently, building from source is the only option.

## First-Time Setup

### 1. Configuration Initialization

On first launch, Quetty will create a configuration file at `ui/config.toml`. You can also copy the example configuration:

```bash
cd ui
cp config.example.toml config.toml
```

### 2. Authentication Setup

Choose one of the following authentication methods:

#### Option A: Azure AD Device Code (Recommended)
```toml
[azure_ad]
auth_method = "device_code"
tenant_id = "your-tenant-id"
client_id = "your-client-id"
```

#### Option B: Connection String
```toml
[servicebus]
connection_string = "Endpoint=sb://namespace.servicebus.windows.net/;SharedAccessKeyName=..."
```

#### Option C: Client Credentials (Service Principal)
```toml
[azure_ad]
auth_method = "client_credentials"
tenant_id = "your-tenant-id"
client_id = "your-client-id"
client_secret = "your-client-secret"
```

For detailed authentication setup, see [AUTHENTICATION.md](AUTHENTICATION.md).

### 3. Environment Variables (Optional)

You can set configuration via environment variables instead of the config file:

```bash
# Azure AD Configuration
export AZURE_AD__TENANT_ID="your-tenant-id"
export AZURE_AD__CLIENT_ID="your-client-id"
export AZURE_AD__CLIENT_SECRET="your-client-secret"  # For client credentials only

# Service Bus Configuration
export SERVICEBUS__CONNECTION_STRING="your-connection-string"

# Application Configuration
export QUETTY_PAGE_SIZE=100
export QUETTY_THEME_NAME="nightfox"
export QUETTY_THEME_FLAVOR="duskfox"
```

## Verification

### 1. Test Installation
```bash
cd ui
cargo run --release -- --version
```

### 2. Test Configuration
Launch Quetty and verify:
- Authentication works correctly
- You can select your namespace
- You can view queues in your namespace

### 3. Test Basic Operations
- Navigate through queues
- View messages (if any exist)
- Open help with `h` key

## Directory Structure

After installation, your Quetty directory will look like:

```
quetty/
├── ui/
│   ├── config.toml          # Main configuration file
│   ├── config.example.toml  # Configuration template
│   ├── quetty.log          # Application logs (if file logging enabled)
│   └── src/                # Source code
├── server/                 # Core library
├── themes/                 # Built-in themes
│   ├── catppuccin/
│   ├── nightfox/
│   └── quetty/
└── README.md
```

## Performance Optimization

### Build Optimizations
For best performance, always use release builds:
```bash
cargo build --release
```

### Configuration Tuning
Edit `config.toml` for optimal performance:
```toml
# Adjust page size based on your queue volume
page_size = 100

# Reduce polling frequency for large queues
poll_timeout_ms = 100

# Optimize for your network conditions
dlq_receive_timeout_secs = 30
```

## Troubleshooting Installation

### Common Issues

#### Rust Compilation Errors
```bash
# Update Rust to latest version
rustup update

# Clear cargo cache if needed
cargo clean
cargo build --release
```

#### Permission Issues
```bash
# Ensure Rust/Cargo are in PATH
echo $PATH | grep -q ".cargo/bin" || echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

#### Network/Firewall Issues
- Ensure ports 443 (HTTPS) and 5671/5672 (AMQP) are accessible
- Verify Azure Service Bus endpoint is reachable
- Check corporate firewall/proxy settings

#### Configuration Issues
- Verify Azure AD app registration permissions
- Check Service Bus connection string format
- Ensure tenant/client IDs are correct

### Getting Help

If you encounter issues:

1. **Check logs**: Enable debug logging in `config.toml`:
   ```toml
   [logging]
   level = "debug"
   file = "quetty.log"
   ```

2. **Verify connectivity**: Test your Azure Service Bus connection with Azure CLI:
   ```bash
   az servicebus queue list --resource-group YOUR-RG --namespace-name YOUR-NAMESPACE
   ```

3. **Check configuration**: Use the built-in configuration screen (`Ctrl+C` in the app)

4. **Consult documentation**: See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for detailed solutions

5. **Report issues**: Create an issue on GitHub with:
   - Operating system and version
   - Rust version (`rustc --version`)
   - Error messages and logs
   - Configuration (with sensitive data removed)

## Next Steps

Once installation is complete:

1. **Configure authentication**: Follow [AUTHENTICATION.md](AUTHENTICATION.md) for detailed auth setup
2. **Learn the interface**: Read [USER_GUIDE.md](USER_GUIDE.md) for complete usage instructions
3. **Customize appearance**: See [THEMING.md](THEMING.md) to customize themes and colors
4. **Optimize configuration**: Reference [CONFIGURATION.md](CONFIGURATION.md) for all available options

## Development Installation

For contributors and developers, additional setup steps are required:

1. **Install development tools**:
   ```bash
   # Install pre-commit hooks
   pip install pre-commit
   pre-commit install
   ```

2. **Run tests**:
   ```bash
   cargo test
   ```

3. **Check code quality**:
   ```bash
   cargo fmt
   cargo clippy
   ```

For complete development setup, see [CONTRIBUTING.md](CONTRIBUTING.md).
