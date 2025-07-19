# CLI Reference

This document provides a complete reference for all Quetty command-line interface options, including detailed explanations, examples, and common usage patterns.

> **📁 Directory Context**: Unless otherwise specified, run commands from the project root directory (`quetty/`).

## Overview

Quetty provides a rich command-line interface that supports profile-based configuration, flexible authentication methods, and various utility commands for managing your Azure Service Bus environments.

## Basic Syntax

```bash
quetty [OPTIONS]
```

## Command-Line Options

### Core Options

#### `--profile, -p <NAME>`
Use a specific profile for configuration and credentials.

```bash
# Use default profile (equivalent to no flag)
quetty

# Use specific profile
quetty --profile dev
quetty -p staging
quetty --profile production
```

**Profile Resolution:**
- Profiles are loaded from `~/.config/quetty/profiles/<NAME>/`
- Each profile has its own `.env` file for credentials
- Profile-specific `config.toml` and `keys.toml` are optional

**Examples:**
```bash
# Development environment
quetty --profile dev

# Production environment with explicit profile
quetty -p prod

# Testing environment
quetty --profile test
```

#### `--config, -c <FILE>`
Use a custom configuration file, bypassing the profile system.

```bash
# Use custom config file
quetty --config /path/to/custom-config.toml
quetty -c ./special-config.toml

# Use config with profile (rare - config takes precedence)
quetty --profile dev --config /path/to/override.toml
```

**When to use:**
- Testing configuration changes
- One-off configurations
- CI/CD environments with custom setups
- Legacy configuration files

### Setup and Configuration

#### `--setup`
Run the interactive setup wizard to configure authentication and create profiles.

```bash
# Setup default profile
quetty --setup

# Setup specific profile
quetty --profile dev --setup
quetty --profile staging --setup
quetty --profile prod --setup
```

> 🛠️ **Setup Details**: See [INSTALLATION.md](INSTALLATION.md) for complete setup instructions.

#### `--config-dir`
Show the configuration directory path and exit.

```bash
# Show configuration directory
quetty --config-dir
# Output: /home/user/.config/quetty
```

**Use cases:**
- Verify configuration location
- Debugging configuration issues
- Scripting and automation
- Manual configuration file management

### Information Options

#### `--help, -h`
Display help information and exit.

```bash
quetty --help
quetty -h
```

#### `--version, -V`
Show version information and exit.

```bash
quetty --version
quetty -V
# Output: quetty 0.1.0
```

## Usage Patterns

### Profile Usage Patterns

#### Daily Development
```bash
# Switch between environments
quetty --profile dev      # Development work
quetty --profile staging  # Testing
quetty --profile prod     # Production monitoring
```

#### Team Workflows
```bash
# Consistent naming across team
quetty --profile team-dev
quetty --profile team-staging
quetty --profile team-prod
```

### Configuration Management

#### Custom Configuration Testing
```bash
# Test configuration without affecting profiles
quetty --config test-config.toml

# Test profile with temporary config override
quetty --profile dev --config temporary-settings.toml
```

#### Configuration Verification
```bash
# Check configuration location
quetty --config-dir

# Verify profile exists and works
quetty --profile test --version

# Debug configuration loading
RUST_LOG=debug quetty --profile dev
```

## Environment Variables

All command-line options can be supplemented with environment variables:

```bash
# Default profile selection
export QUETTY_PROFILE=dev
quetty  # Will use dev profile

# Configuration file override
export QUETTY_CONFIG_PATH=/path/to/config.toml
quetty  # Will use specified config file
```

## Error Handling

### Profile Not Found
```bash
$ quetty --profile nonexistent
Error: Profile 'nonexistent' does not exist.

Available profiles: default, dev, staging, prod

To create a new profile, run: quetty -p nonexistent --setup
```

### Invalid Profile Name
```bash
$ quetty --profile ../etc/passwd
Error: Invalid profile name '../etc/passwd': Profile name cannot contain path separators or traversal sequences
```

### Configuration Issues
```bash
$ quetty --config nonexistent.toml
Error: Configuration file not found: nonexistent.toml
```

## Integration Examples

### Shell Aliases
```bash
# Add to ~/.bashrc or ~/.zshrc
alias q='quetty'
alias qdev='quetty --profile dev'
alias qstaging='quetty --profile staging'
alias qprod='quetty --profile prod'
alias qsetup='quetty --setup'
```

### CI/CD Integration
```bash
# In CI/CD pipeline
export AZURE_AD__TENANT_ID="${CI_TENANT_ID}"
export AZURE_AD__CLIENT_ID="${CI_CLIENT_ID}"
export AZURE_AD__CLIENT_SECRET="${CI_CLIENT_SECRET}"
export AZURE_AD__AUTH_METHOD="client_secret"

# Run with CI profile
quetty --profile ci
```

### Docker Integration
```dockerfile
# In Dockerfile
COPY target/release/quetty /usr/local/bin/
ENTRYPOINT ["quetty", "--profile", "docker"]
```

### Scripting
```bash
#!/bin/bash
# Check if profile exists before using
if quetty --profile "$ENVIRONMENT" --version >/dev/null 2>&1; then
    quetty --profile "$ENVIRONMENT"
else
    echo "Profile $ENVIRONMENT not found. Setting up..."
    quetty --profile "$ENVIRONMENT" --setup
fi
```

## Advanced Usage

### Profile Inheritance
While Quetty doesn't support profile inheritance directly, you can achieve similar results:

```bash
# Base configuration in shared location
quetty --config /shared/base-config.toml --profile dev

# Environment-specific overrides via .env files
# /home/user/.config/quetty/profiles/dev/.env contains environment-specific values
```

### Configuration Composition
```bash
# Use base config with profile-specific credentials
quetty --config /shared/prod-config.toml --profile prod-secrets
```

### Debugging
```bash
# Enable debug logging
RUST_LOG=debug quetty --profile dev

# Trace configuration loading
RUST_LOG=trace quetty --profile staging

# Debug specific modules
RUST_LOG=quetty::config=debug quetty --profile prod
```

## Best Practices

- Use descriptive profile names: `dev`, `staging`, `prod`
- Include team/project prefix for clarity: `myteam-dev`
- Avoid special characters in profile names

> 📚 **Complete Guidelines**: See [CONFIGURATION.md](CONFIGURATION.md) for security and organization best practices.

## Troubleshooting

### Quick Diagnostics
```bash
# Check configuration location
quetty --config-dir

# Verify profile works
quetty --profile dev --version

# Debug configuration loading
RUST_LOG=debug quetty --profile dev
```

> 🔧 **Detailed Help**: See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for complete troubleshooting guide.

## Related Documentation

- **[Installation Guide](INSTALLATION.md)** - Setting up Quetty
- **[Configuration Reference](CONFIGURATION.md)** - Complete configuration options
- **[User Guide](USER_GUIDE.md)** - Interface and features
