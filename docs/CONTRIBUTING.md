# Contributing to Quetty

Thank you for your interest in contributing to Quetty! This guide will help you get started with development, understand our processes, and make meaningful contributions.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Development Workflow](#development-workflow)
- [Code Standards](#code-standards)
- [Testing](#testing)
- [Documentation](#documentation)
- [Submitting Changes](#submitting-changes)
- [Code Review Process](#code-review-process)
- [Release Process](#release-process)

## Getting Started

### Prerequisites

- **Rust**: Latest stable version (install via [rustup](https://rustup.rs/))
- **Git**: For version control
- **Azure Account**: For testing Service Bus integration
- **Code Editor**: VS Code with rust-analyzer recommended

### First-Time Setup

1. **Fork the repository** on GitHub

2. **Clone your fork**:
   ```bash
   git clone https://github.com/YOUR-USERNAME/quetty.git
   cd quetty
   ```

3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/ORIGINAL-OWNER/quetty.git
   ```

4. **Install development tools**:
   ```bash
   # Install pre-commit hooks
   pip install pre-commit
   pre-commit install

   # Install cargo tools
   cargo install cargo-watch
   cargo install cargo-audit
   ```

## Development Setup

### Environment Configuration

1. **Create development config**:
   ```bash
   cp config.default.toml config.dev.toml
   # Edit config.dev.toml with your Azure credentials
   ```

2. **Set environment variables**:
   ```bash
   export RUST_LOG=debug
   export QUETTY_CONFIG=config.dev.toml
   ```

### Building and Running

> **📁 Directory Context**: Unless otherwise specified, run commands from the project root directory (`quetty/`).

```bash
# Build all components
cargo build

# Run the application
./target/release/quetty

# Run with file watching (auto-reload on changes) from ui directory
cd ui && cargo watch -x run

# Run tests
cargo test

# Run with specific config
./target/release/quetty --config config.dev.toml

# Run traffic simulator for testing
make test-server QUEUE=test-queue-name
```

For traffic simulation testing, see [TRAFFIC_SIMULATOR.md](TRAFFIC_SIMULATOR.md) for detailed usage instructions.


## Project Structure

```
quetty/
├── ui/                          # Main application (TUI)
│   ├── src/
│   │   ├── app/                 # Application state and lifecycle
│   │   ├── components/          # UI components
│   │   ├── config/             # Configuration handling
│   │   ├── services/           # Business logic services
│   │   ├── theme/              # Theme system
│   │   └── main.rs            # Entry point
│   └── Cargo.toml
├── config.default.toml     # Example configuration
├── server/                     # Core Service Bus library
│   ├── src/
│   │   ├── auth/               # Authentication providers
│   │   ├── service_bus_manager/ # Service Bus operations
│   │   ├── bulk_operations/    # Bulk operation handlers
│   │   └── lib.rs             # Library entry point
│   └── Cargo.toml
├── traffic-simulator/          # Standalone traffic simulation tool
│   ├── src/
│   │   ├── main.rs            # Traffic simulator application
│   │   ├── service_bus.rs     # Azure Service Bus wrapper
│   │   ├── producer.rs        # Message sending
│   │   ├── consumer.rs        # Message receiving
│   │   └── config.rs          # Configuration management
│   ├── config.toml            # Traffic-specific settings
│   └── Cargo.toml             # Independent project
├── themes/                     # Built-in themes
│   ├── nightfox/
│   ├── catppuccin/
│   └── quetty/
├── scripts/                    # Build and utility scripts
└── docs/                      # Additional documentation
    ├── TRAFFIC_SIMULATOR.md   # Traffic simulator guide
    └── ...                    # Other documentation
```

### Module Organization

#### UI Module (`ui/src/`)
- **`app/`**: Application state, lifecycle, and event handling
- **`components/`**: Reusable UI components and their logic
- **`config/`**: Configuration parsing and validation
- **`services/`**: Business logic and external service integration
- **`theme/`**: Theme loading and management
- **`utils/`**: Utility functions and helpers

#### Server Module (`server/src/`)
- **`auth/`**: Authentication providers and token management
- **`service_bus_manager/`**: Azure Service Bus operations
- **`bulk_operations/`**: Efficient bulk operation implementations
- **`common/`**: Shared types and utilities

#### Traffic Simulator (`traffic-simulator/src/`)
- **`main.rs`**: Standalone traffic simulation application
- **`service_bus.rs`**: Azure Service Bus client wrapper
- **`producer.rs`**: Message sending functionality
- **`consumer.rs`**: Message receiving functionality
- **`config.rs`**: Configuration loading and validation

See [TRAFFIC_SIMULATOR.md](TRAFFIC_SIMULATOR.md) for detailed development and usage instructions.

## Development Workflow

### Branch Strategy

1. **Main Branch**: `main` - stable, production-ready code
2. **Feature Branches**: `feature/description` - new features
3. **Bug Fix Branches**: `fix/description` - bug fixes
4. **Release Branches**: `release/v1.0.0` - release preparation

### Working on Features

1. **Create feature branch**:
   ```bash
   git checkout main
   git pull upstream main
   git checkout -b feature/your-feature-name
   ```

2. **Make changes**: Implement your feature with tests

3. **Commit frequently**: Use descriptive commit messages

4. **Keep branch updated**:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

5. **Test thoroughly**: Ensure all tests pass

6. **Submit pull request**: Create PR when ready

### Commit Message Format

Use conventional commits for consistency:

```
type(scope): description

body (optional)

footer (optional)
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code formatting changes
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

**Examples:**
```
feat(auth): add client credentials authentication support

fix(ui): resolve message list pagination issue

docs: update installation guide for Windows
```

## Code Standards

### Rust Code Style

We follow standard Rust conventions with some project-specific guidelines:

#### Formatting
- Use `cargo fmt` for consistent formatting
- Line length: 100 characters max
- Use 4 spaces for indentation (no tabs)

#### Naming Conventions
- `snake_case` for functions, variables, modules
- `PascalCase` for types, structs, enums
- `SCREAMING_SNAKE_CASE` for constants
- Clear, descriptive names over short names

#### Code Organization
```rust
// Standard library imports
use std::collections::HashMap;

// External crate imports
use tokio::time::{sleep, Duration};
use serde::{Deserialize, Serialize};

// Internal imports
use crate::config::Config;
use crate::service_bus::Manager;

// Module-level documentation
//! This module handles authentication...

/// Function documentation
///
/// # Arguments
/// * `config` - Configuration object
///
/// # Returns
/// Result with authentication token
pub async fn authenticate(config: &Config) -> Result<Token> {
    // Implementation
}
```

### Error Handling

Use `Result<T, E>` for error handling:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Network error: {0}")]
    Network(String),
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
}

pub type AuthResult<T> = Result<T, AuthError>;
```

### Async/Await Guidelines

- Use `async/await` for I/O operations
- Prefer `tokio` runtime for async execution
- Use `Arc` and `Mutex` carefully for shared state
- Consider using channels for communication between tasks

### Configuration Handling

Follow the established pattern for configuration:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct FeatureConfig {
    pub enabled: bool,
    pub timeout_secs: u64,
    #[serde(default = "default_retries")]
    pub max_retries: u32,
}

fn default_retries() -> u32 {
    3
}
```

## Testing

### Test Categories

1. **Unit Tests**: Test individual functions and modules
2. **Integration Tests**: Test component interactions
3. **End-to-End Tests**: Test complete user workflows
4. **Performance Tests**: Test performance characteristics

### Writing Tests

#### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing() {
        let config = Config::from_str(r#"
            page_size = 100
            [theme]
            theme_name = "nightfox"
        "#).unwrap();

        assert_eq!(config.page_size, 100);
        assert_eq!(config.theme.theme_name, "nightfox");
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = async_function().await;
        assert!(result.is_ok());
    }
}
```

#### Integration Tests
```rust
// tests/integration_test.rs
use quetty_server::ServiceBusManager;

#[tokio::test]
async fn test_message_operations() {
    let manager = ServiceBusManager::new(test_config()).await.unwrap();

    // Test sending message
    let message_id = manager.send_message("test content").await.unwrap();

    // Test receiving message
    let messages = manager.receive_messages(1).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, message_id);
}
```

### Test Configuration

Create test configurations that don't interfere with development:

```toml
# config.test.toml
page_size = 10
[azure_ad]
auth_method = "device_code"
tenant_id = "test-tenant"
client_id = "test-client"
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test integration_test

# Run with environment variable
RUST_LOG=debug cargo test
```

### Mock Testing

For Azure Service Bus operations, use mocking:

```rust
#[cfg(test)]
use mockall::predicate::*;

#[cfg_attr(test, mockall::automock)]
trait ServiceBusOperations {
    async fn send_message(&self, content: &str) -> Result<String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_with_mock() {
        let mut mock = MockServiceBusOperations::new();
        mock.expect_send_message()
            .with(eq("test"))
            .times(1)
            .returning(|_| Ok("msg-123".to_string()));

        let result = mock.send_message("test").await;
        assert_eq!(result.unwrap(), "msg-123");
    }
}
```

## Documentation

### Code Documentation

#### Module Documentation
```rust
//! # Authentication Module
//!
//! This module provides authentication capabilities for Azure Service Bus,
//! supporting multiple authentication flows including device code
//! and client credentials.
//!
//! ## Examples
//!
//! ```rust
//! use quetty_server::auth::Authenticator;
//!
//! let auth = Authenticator::new(config).await?;
//! let token = auth.get_token().await?;
//! ```
```

#### Function Documentation
```rust
/// Authenticates with Azure AD using device code flow.
///
/// This function initiates the device code authentication flow, which
/// requires user interaction through a web browser.
///
/// # Arguments
///
/// * `config` - Azure AD configuration containing tenant and client IDs
/// * `timeout` - Maximum time to wait for user authentication
///
/// # Returns
///
/// Returns `Ok(Token)` on successful authentication, or an error if:
/// - Device code expires before user completes authentication
/// - Network connectivity issues occur
/// - Invalid configuration is provided
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
///
/// let token = authenticate_device_code(&config, Duration::from_secs(300)).await?;
/// println!("Authentication successful: {}", token.access_token);
/// ```
///
/// # Errors
///
/// This function may return the following errors:
/// - `AuthError::DeviceCodeExpired` - User didn't complete flow in time
/// - `AuthError::InvalidConfig` - Missing or invalid configuration
/// - `AuthError::Network` - Network connectivity issues
pub async fn authenticate_device_code(
    config: &AzureAdConfig,
    timeout: Duration,
) -> AuthResult<Token> {
    // Implementation
}
```

### External Documentation

When adding new features, update relevant documentation files:

- **README.md**: Overview and quick start
- **USER_GUIDE.md**: User-facing feature documentation
- **CONFIGURATION.md**: New configuration options
- **API documentation**: For public APIs

### Documentation Testing

Ensure documentation examples compile and work:

```bash
# Test documentation examples
cargo test --doc

# Generate documentation
cargo doc --open
```

## Submitting Changes

### Pull Request Process

1. **Prepare your branch**:
   ```bash
   git checkout feature/your-feature
   git rebase upstream/main
   git push origin feature/your-feature
   ```

2. **Create pull request** with:
   - Clear title and description
   - Reference to any related issues
   - Screenshots for UI changes
   - Testing instructions

3. **Pull request template**:
   ```markdown
   ## Description
   Brief description of changes

   ## Type of Change
   - [ ] Bug fix
   - [ ] New feature
   - [ ] Breaking change
   - [ ] Documentation update

   ## Testing
   - [ ] Unit tests pass
   - [ ] Integration tests pass
   - [ ] Manual testing completed

   ## Checklist
   - [ ] Code follows style guidelines
   - [ ] Self-review completed
   - [ ] Documentation updated
   - [ ] Tests added/updated
   ```

### Pre-submission Checklist

- [ ] **Code compiles** without warnings
- [ ] **All tests pass** (`cargo test`)
- [ ] **Code is formatted** (`cargo fmt`)
- [ ] **Lints pass** (`cargo clippy`)
- [ ] **Documentation updated** for new features
- [ ] **Commit messages** follow convention
- [ ] **No sensitive data** in commits

## Code Review Process

### Review Criteria

Reviewers will check:

1. **Functionality**: Does the code work as intended?
2. **Code Quality**: Is the code well-structured and readable?
3. **Tests**: Are there adequate tests for the changes?
4. **Documentation**: Is documentation updated appropriately?
5. **Performance**: Are there any performance implications?
6. **Security**: Are there any security concerns?

### Addressing Review Comments

1. **Read carefully**: Understand the feedback
2. **Ask questions**: If unclear, ask for clarification
3. **Make changes**: Address all valid feedback
4. **Respond**: Mark conversations as resolved when addressed
5. **Update PR**: Push new commits or force-push after rebase

### Review Timeline

- **Initial response**: Within 2-3 business days
- **Follow-up reviews**: Within 1-2 business days
- **Merge timeline**: Varies based on complexity

## Release Process

### Version Numbering

We use [Semantic Versioning](https://semver.org/):
- **MAJOR.MINOR.PATCH** (e.g., 1.2.3)
- **Major**: Breaking changes
- **Minor**: New features (backward compatible)
- **Patch**: Bug fixes (backward compatible)

### Release Workflow

Quetty uses an automated release process with version management scripts:

#### For Maintainers: Creating a Release

1. **Ensure main branch is stable**
   ```bash
   git checkout main
   git pull origin main
   # Verify all tests pass
   cargo test
   ```

2. **Use the release script**
   ```bash
   # For stable release
   ./scripts/release.sh 1.2.0

   # For pre-release
   ./scripts/release.sh 1.2.0-beta.1
   ./scripts/release.sh 1.2.0-rc.1
   ```

   The script will:
   - Validate version format
   - Update `ui/Cargo.toml` and `server/Cargo.toml`
   - Test the build
   - Commit version changes
   - Create and push git tag
   - Trigger automated release workflow

3. **Monitor the release**
   - GitHub Actions will build cross-platform binaries
   - Artifacts will be uploaded to GitHub Releases
   - Release notes will be auto-generated

4. **Post-release: Bump to development version**
   ```bash
   # Update to next development version
   # Edit ui/Cargo.toml and server/Cargo.toml
   # Example: 1.2.0 → 1.3.0-dev
   git add ui/Cargo.toml server/Cargo.toml
   git commit -m "chore: bump to 1.3.0-dev"
   git push origin main
   ```

#### Release Channels

- **Stable releases**: `v1.2.0` - Production ready
- **Pre-releases**: `v1.2.0-beta.1`, `v1.2.0-rc.1` - Testing versions
- **Nightly builds**: Automated daily builds from main branch

#### Supported Platforms

Releases are automatically built for:
- Linux x64
- Windows x64 & ARM64
- macOS x64 & ARM64 (Intel and Apple Silicon)

Each release includes:
- Cross-platform binaries
- SHA256 checksums for verification
- Installation instructions
- Auto-generated changelog

6. **Merge back**: Merge release branch to main

### Changelog Format

```markdown
# Changelog

## [1.2.0] - 2024-01-15

### Added
- New theme system with Catppuccin themes
- Bulk message operations support
- Azure Client Credentials authentication

### Changed
- Improved message list performance
- Updated configuration format

### Fixed
- Fixed memory leak in message caching
- Resolved authentication token refresh issue

### Deprecated
- Old configuration format (will be removed in v2.0.0)
```

## Development Best Practices

### Performance Considerations

1. **Async Operations**: Use async for I/O-bound operations
2. **Memory Management**: Avoid unnecessary allocations
3. **Caching**: Cache expensive operations appropriately
4. **Lazy Loading**: Load data only when needed

### Security Guidelines

1. **Secrets**: Never commit secrets or credentials
2. **Input Validation**: Validate all user inputs
3. **Error Messages**: Don't leak sensitive information in errors
4. **Dependencies**: Regularly audit dependencies for vulnerabilities

### Debugging

#### Logging
```rust
use tracing::{debug, info, warn, error};

#[tracing::instrument]
pub async fn process_message(message: &Message) -> Result<()> {
    debug!("Processing message: {}", message.id);

    match process_step_1(message).await {
        Ok(_) => info!("Step 1 completed successfully"),
        Err(e) => {
            error!("Step 1 failed: {}", e);
            return Err(e);
        }
    }

    Ok(())
}
```

#### Debugging Tools
```bash
# Run with debugging (from ui directory for development)
cd ui && RUST_LOG=debug cargo run

# Profile performance (from ui directory for development)
cd ui && cargo run --release --features profiling

# Memory debugging
valgrind target/release/quetty
```

### Common Patterns

#### Error Propagation
```rust
use anyhow::{Context, Result};

pub fn complex_operation() -> Result<String> {
    let data = read_file("config.toml")
        .context("Failed to read configuration file")?;

    let parsed = parse_config(&data)
        .context("Failed to parse configuration")?;

    Ok(format!("Loaded config: {}", parsed.name))
}
```

#### Configuration Pattern
```rust
#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub feature: FeatureConfig,
}

#[derive(Debug, Deserialize)]
pub struct FeatureConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_enabled() -> bool { true }
fn default_timeout() -> u64 { 30 }
```

## Getting Help

### Communication Channels

1. **GitHub Issues**: Bug reports and feature requests
2. **GitHub Discussions**: General questions and ideas
3. **Code Reviews**: Direct feedback on pull requests

### Mentoring

New contributors can:
1. **Look for "good first issue" labels** on GitHub
2. **Ask questions** in issues or discussions
3. **Start small** with documentation or test improvements
4. **Pair with maintainers** for complex features

### Resources

- [Rust Book](https://doc.rust-lang.org/book/) - Learn Rust fundamentals
- [Async Programming](https://rust-lang.github.io/async-book/) - Async/await patterns
- [Azure Service Bus Docs](https://docs.microsoft.com/azure/service-bus/) - Service Bus concepts
- [TUI Development](https://github.com/fdehau/tui-rs) - Terminal UI library

Thank you for contributing to Quetty! Your efforts help make queue management better for everyone.
