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

### Option 1: Universal Installation Script (Recommended)

The fastest way to get Quetty up and running on any supported platform.

#### Unix/Linux/macOS One-Line Installation
```bash
curl -fsSL https://raw.githubusercontent.com/dawidpereira/quetty/main/install.sh | sh
```

#### Windows PowerShell One-Line Installation
```powershell
Invoke-RestMethod -Uri "https://raw.githubusercontent.com/dawidpereira/quetty/main/install.ps1" | Invoke-Expression
```

#### Advanced Installation Options

**Install specific version:**
```bash
# Unix/Linux/macOS
curl -fsSL https://raw.githubusercontent.com/dawidpereira/quetty/main/install.sh | sh -s -- --version v0.1.0-alpha.1

# Windows
Invoke-RestMethod -Uri "https://raw.githubusercontent.com/dawidpereira/quetty/main/install.ps1" -OutFile install.ps1
.\install.ps1 -Version "v0.1.0-alpha.1"
```

**Install to custom directory:**
```bash
# Unix/Linux/macOS
curl -fsSL https://raw.githubusercontent.com/dawidpereira/quetty/main/install.sh | sh -s -- --install-dir /opt/bin

# Windows
.\install.ps1 -InstallDir "C:\Tools\bin"
```

**System-wide installation:**
```bash
# Unix/Linux/macOS (requires sudo)
curl -fsSL https://raw.githubusercontent.com/dawidpereira/quetty/main/install.sh | sh -s -- --system

# Windows (requires admin PowerShell)
.\install.ps1 -System
```

**Install nightly build:**
```bash
# Unix/Linux/macOS
curl -fsSL https://raw.githubusercontent.com/dawidpereira/quetty/main/install.sh | sh -s -- --channel nightly

# Windows
.\install.ps1 -Channel "nightly"
```

**Preview installation (dry run):**
```bash
# Unix/Linux/macOS
curl -fsSL https://raw.githubusercontent.com/dawidpereira/quetty/main/install.sh | sh -s -- --dry-run

# Windows
.\install.ps1 -DryRun
```

#### Supported Platforms
- **Linux x64** - `quetty-linux-x64`
- **macOS Intel** - `quetty-macos-x64`
- **macOS Apple Silicon** - `quetty-macos-arm64`
- **Windows x64** - `quetty-windows-x64.exe`
- **Windows ARM64** - `quetty-windows-arm64.exe`

#### What the Script Does
1. **Auto-detects** your platform and architecture
2. **Downloads** the correct pre-built binary from GitHub releases
3. **Verifies** SHA256 checksum for security
4. **Installs** to `~/.local/bin` (user) or system directory
5. **Updates PATH** if needed
6. **Ready to use** - just run `quetty`

### Option 2: Manual Binary Download

Download pre-built binaries directly from [GitHub Releases](https://github.com/dawidpereira/quetty/releases):

1. **Download the correct binary** for your platform
2. **Extract the archive**:
   ```bash
   # Linux/macOS
   tar -xzf quetty-*-*.tar.gz
   chmod +x quetty-*

   # Windows
   # Extract the ZIP file and run the .exe
   ```
3. **Verify checksum** (recommended):
   ```bash
   # Download checksums.txt and verify
   sha256sum -c checksums.txt
   ```
4. **Move to PATH**:
   ```bash
   # Unix/Linux/macOS
   mv quetty-* ~/.local/bin/quetty

   # Windows - move to a directory in your PATH
   ```

### Option 3: Build from Source

For development or if you need the latest unreleased features:

> **📁 Directory Context**: All commands in this guide should be run from the project root directory (`quetty/`) unless otherwise specified.

1. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. **Clone the repository**:
   ```bash
   git clone https://github.com/dawidpereira/quetty.git
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
   ./target/release/quetty
   ```

5. **Add to PATH (Recommended)**:
   ```bash
   # Copy binary to local bin directory
   cp target/release/quetty ~/.local/bin/

   # Or add to current session PATH
   export PATH="$PWD/target/release:$PATH"

   # Make permanent (add to ~/.bashrc, ~/.zshrc, etc.)
   echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
   ```

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

# Or run the built binary directly
./target/release/quetty --version
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
