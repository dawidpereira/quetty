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

5. **Add to PATH (Recommended)**:
   ```bash
   # Copy binary to local bin directory
   cp ../target/release/quetty ~/.local/bin/

   # Or add to current session PATH
   export PATH="$PWD/../target/release:$PATH"

   # Make permanent (add to ~/.bashrc, ~/.zshrc, etc.)
   echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
   ```

### Option 2: Binary Releases (Coming Soon)

Pre-built binaries will be available for:
- Linux (x86_64, ARM64)
- macOS (Intel, Apple Silicon)
- Windows (x86_64)

> 📦 **Note**: Binary releases are planned for the next version. Currently, building from source is the only option.

## First-Time Setup

### 1. Configuration Initialization

Quetty uses a **profile-based configuration system** that allows you to manage multiple environments (development, staging, production) with separate, isolated settings.

#### Option A: Interactive Setup Wizard (Recommended)
```bash
# Setup default profile
quetty --setup

# Setup specific profiles for different environments
quetty --profile dev --setup       # Development environment
quetty --profile staging --setup   # Staging environment
quetty --profile prod --setup      # Production environment
```
This will guide you through creating your configuration with helpful prompts.

#### Option B: Quick Setup
```bash
quetty --config-dir  # Show where config will be stored
quetty              # Will auto-create default profile on first run
```

#### Option C: Manual Profile Setup
Create profiles manually for different environments:
```bash
# Create profile directories
mkdir -p ~/.config/quetty/profiles/dev
mkdir -p ~/.config/quetty/profiles/staging
mkdir -p ~/.config/quetty/profiles/prod

# Create environment-specific .env files with credentials
echo "# Development environment" > ~/.config/quetty/profiles/dev/.env
echo "# Staging environment" > ~/.config/quetty/profiles/staging/.env
echo "# Production environment" > ~/.config/quetty/profiles/prod/.env
```

> 📁 **Configuration Location**: Quetty stores profiles in `~/.config/quetty/profiles/`. See [CONFIGURATION.md](CONFIGURATION.md) for complete directory structure and configuration options.

### 2. Authentication Setup

The setup wizard will guide you through authentication configuration. You can choose from:
- **Connection String** (simplest - get from Azure Portal)
- **Azure AD Device Code** (interactive - good for development)
- **Client Credentials** (automated - good for production)

> 🔐 **Authentication Details**: See [CONFIGURATION.md](CONFIGURATION.md) for complete authentication configuration and examples.

### 3. Using Profiles

```bash
# Use default profile
quetty

# Use specific profiles
quetty --profile dev
quetty --profile staging
quetty --profile prod

# Create new profile
quetty --profile myproject --setup
```

> 📖 **Command Reference**: See [CLI_REFERENCE.md](CLI_REFERENCE.md) for all available commands and options.

## Verification

### 1. Test Installation
```bash
# If added to PATH
quetty --version

# From source directory
cd ui
cargo run --release -- --version
```

### 2. Test Profile System
```bash
# Check configuration directory
quetty --config-dir

# Create and test a profile
quetty --profile test --setup
quetty --profile test --version  # Should work if setup completed
```

### 3. Test Configuration
Launch Quetty with different profiles and verify:
- Authentication works correctly for each environment
- You can select your namespace
- You can view queues in your namespace
- Profile isolation works (different credentials per environment)

### 4. Test Basic Operations
- Navigate through queues
- View messages (if any exist)
- Open help with `h` key
- Switch between profiles during development


## Shell Integration

### Adding Quetty to PATH

#### Method 1: Copy to User Bin Directory
```bash
# Create local bin directory if it doesn't exist
mkdir -p ~/.local/bin

# Copy quetty binary
cp target/release/quetty ~/.local/bin/

# Add to PATH (if not already there)
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc  # For Bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc   # For Zsh
```

#### Method 2: Add Build Directory to PATH
```bash
# Add project's target directory to PATH
echo 'export PATH="/path/to/quetty/target/release:$PATH"' >> ~/.bashrc
```

#### Method 3: System-Wide Installation (Linux/macOS)
```bash
# Copy to system bin directory (requires sudo)
sudo cp target/release/quetty /usr/local/bin/
```

### Shell Aliases and Functions

Add useful aliases to your shell configuration:

```bash
# Basic aliases
alias q='quetty'
alias qdev='quetty --profile dev'
alias qstaging='quetty --profile staging'
alias qprod='quetty --profile prod'

# Setup aliases
alias qsetup='quetty --setup'
alias qsetup-dev='quetty --profile dev --setup'
alias qsetup-prod='quetty --profile prod --setup'

# Utility aliases
alias qconfig='quetty --config-dir'
alias qhelp='quetty --help'
```


## Performance Optimization

For best performance, always use release builds:
```bash
cargo build --release
```

> ⚡ **Performance Tuning**: See [CONFIGURATION.md](CONFIGURATION.md) for performance configuration options.

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

4. **Get Help**: See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for detailed solutions or create a GitHub issue

## Next Steps

1. **Learn the interface**: See [USER_GUIDE.md](USER_GUIDE.md) for usage instructions
2. **Configure settings**: See [CONFIGURATION.md](CONFIGURATION.md) for all options
3. **Get help**: See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for common issues

## Development Setup

For contributors, see [CONTRIBUTING.md](CONTRIBUTING.md) for development setup instructions.
