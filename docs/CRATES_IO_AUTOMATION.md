# Crates.io Publishing Automation

This document describes the automated crates.io publishing setup for Quetty.

## Overview

Quetty has automated publishing to crates.io that triggers on **stable releases only**. The automation publishes two crates:

1. **`quetty-server`** - Core Azure Service Bus client library
2. **`quetty`** - Main terminal application (installable binary)

## How It Works

### Automatic Publishing (GitHub Actions)

The automation runs when you create a GitHub release with a stable version tag (no `-alpha`, `-beta`, or `-rc` suffixes).

**Trigger:** Push tags matching `v*.*.*` (stable versions only)
**Workflow:** `.github/workflows/release.yml`

**Process:**
1. **Validate release** - Ensures tag format and stable version
2. **Build artifacts** - Creates binaries for all platforms
3. **Publish server crate** - Publishes `quetty-server` to crates.io
4. **Wait for availability** - Polls crates.io until server crate is available
5. **Update UI dependencies** - Replaces path dependency with published version
6. **Publish UI crate** - Publishes `quetty` (main installable package)
7. **Create GitHub release** - Uploads artifacts and generates changelog

### Manual Testing

Use the helper script to test publishing locally:

```bash
# Dry-run test (recommended)
./scripts/prepare_crates_release.sh 1.0.0 true

# Actual publishing (requires CARGO_REGISTRY_TOKEN)
export CARGO_REGISTRY_TOKEN=your_token_here
./scripts/prepare_crates_release.sh 1.0.0
```

## Release Process

### For Stable Releases

1. **Prepare release:**
   ```bash
   ./scripts/release.sh 1.0.0
   ```

2. **Push to trigger automation:**
   ```bash
   git push origin main v1.0.0
   ```

3. **Monitor automation:**
   - Check GitHub Actions: https://github.com/dawidpereira/quetty/actions
   - Verify crates.io: https://crates.io/crates/quetty

### For Pre-releases

Pre-releases (versions with `-alpha`, `-beta`, `-rc`) will:
- ✅ Build release artifacts
- ✅ Create GitHub release
- ❌ **NOT** publish to crates.io

## Installation After Publishing

Once published, users can install via:

```bash
cargo install quetty
```

## Required Secrets

The GitHub repository requires this secret for automation:

- **`CARGO_REGISTRY_TOKEN`** - Crates.io API token with publishing permissions

## Troubleshooting

### Common Issues

1. **Publishing fails for UI crate**
   - Usually means server crate isn't available yet
   - Automation waits up to 5 minutes for availability

2. **Version already exists**
   - Crates.io doesn't allow republishing same version
   - Use a new version number

3. **Metadata validation fails**
   - Check that all required fields are present in Cargo.toml
   - Validate with: `cargo publish --dry-run -p quetty-server`

### Manual Recovery

If automation fails, you can manually publish:

```bash
# Set token
export CARGO_REGISTRY_TOKEN=your_token

# Publish server crate first
cargo publish -p quetty-server

# Wait for availability, then update UI dependency and publish
./scripts/prepare_crates_release.sh 1.0.0
```

## Crate Information

### quetty-server
- **Purpose:** Core Azure Service Bus client library
- **Type:** Library crate
- **Dependencies:** Azure SDK, async runtime, crypto libraries

### quetty
- **Purpose:** Terminal application (main package)
- **Type:** Binary crate with library
- **Binary name:** `quetty`
- **Dependencies:** TUI libraries, quetty-server

Both crates share workspace metadata and are versioned together.
