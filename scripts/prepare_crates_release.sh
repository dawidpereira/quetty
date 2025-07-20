#!/bin/bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

VERSION=$1
DRY_RUN=${2:-false}

if [ -z "$VERSION" ]; then
  echo -e "${RED}❌ Usage: ./scripts/prepare_crates_release.sh <version> [dry-run]${NC}"
  echo -e "${BLUE}Examples:${NC}"
  echo "  ./scripts/prepare_crates_release.sh 1.2.0"
  echo "  ./scripts/prepare_crates_release.sh 1.2.0 true  # dry-run mode"
  echo ""
  echo -e "${BLUE}💡 Note:${NC} This script prepares crates for publishing to crates.io"
  exit 1
fi

# Validate version format
if ! [[ $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?(\+[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$ ]]; then
  echo -e "${RED}❌ Invalid version format${NC}"
  echo -e "${BLUE}Use semantic versioning:${NC} 1.2.0 or 1.2.0-beta.1"
  exit 1
fi

echo -e "${BLUE}🚀 Preparing crates for release version ${YELLOW}$VERSION${NC}"
if [ "$DRY_RUN" = "true" ]; then
  echo -e "${YELLOW}🧪 Running in dry-run mode${NC}"
fi

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ] || [ ! -d "server" ] || [ ! -d "ui" ]; then
  echo -e "${RED}❌ Must be run from the project root directory${NC}"
  exit 1
fi

# Check for CARGO_REGISTRY_TOKEN if not in dry-run mode
if [ "$DRY_RUN" != "true" ]; then
  if [ -z "${CARGO_REGISTRY_TOKEN:-}" ]; then
    echo -e "${RED}❌ CARGO_REGISTRY_TOKEN environment variable is required for publishing${NC}"
    echo -e "${BLUE}Set it with:${NC} export CARGO_REGISTRY_TOKEN=your_token_here"
    exit 1
  fi
fi

# Update version in workspace Cargo.toml
echo -e "${BLUE}📝 Updating workspace version to $VERSION...${NC}"

# Check if cargo-edit is available for portable version management
if command -v cargo-set-version >/dev/null 2>&1; then
  echo -e "${BLUE}Using cargo set-version for cross-platform compatibility${NC}"
  cargo set-version --workspace "$VERSION"
else
  echo -e "${YELLOW}cargo-edit not found, using awk (less portable)${NC}"
  # Use awk to update the workspace version in root Cargo.toml
  tmp=$(mktemp)
  awk -v version="$VERSION" '/^\[workspace\.package\]/{flag=1; print; next} flag && /^version = /{sub(/version = ".*"/, "version = \"" version "\""); flag=0} 1' Cargo.toml > "$tmp" && mv "$tmp" Cargo.toml
fi

# Verify changes
echo -e "${BLUE}🔍 Verifying version updates...${NC}"
WORKSPACE_VERSION=$(grep -A 10 "^\[workspace\.package\]" Cargo.toml | grep "^version" | cut -d'"' -f2)

if [ "$WORKSPACE_VERSION" != "$VERSION" ]; then
  echo -e "${RED}❌ Version update failed${NC}"
  echo "Expected: $VERSION"
  echo "Workspace: $WORKSPACE_VERSION"
  exit 1
fi

echo -e "${GREEN}✅ Updated workspace version:${NC}"
echo "  Cargo.toml: $WORKSPACE_VERSION"

# Validate crates
echo -e "${BLUE}🔍 Validating crates...${NC}"
cargo check -p quetty-server
cargo check -p quetty

# Dry run publish for both crates
echo -e "${BLUE}🧪 Testing crate publishing (dry-run)...${NC}"

echo -e "${BLUE}Testing server crate...${NC}"
cargo publish --dry-run -p quetty-server

echo -e "${BLUE}Testing UI crate...${NC}"
cargo publish --dry-run -p quetty

if [ "$DRY_RUN" = "true" ]; then
  echo -e "${GREEN}✅ Dry-run completed successfully!${NC}"
  echo -e "${BLUE}All crates are ready for publishing to crates.io${NC}"
  exit 0
fi

# Actual publishing
echo -e "${BLUE}📦 Publishing to crates.io...${NC}"

# Publish server crate first
echo -e "${BLUE}Publishing server crate...${NC}"
cargo publish -p quetty-server
echo -e "${GREEN}✅ Server crate published successfully!${NC}"

# Wait for server crate to be available
echo -e "${BLUE}⏳ Waiting for server crate to become available on crates.io...${NC}"
sleep 30

# Poll crates.io API to ensure the crate is available
for i in {1..10}; do
  if curl -f "https://crates.io/api/v1/crates/quetty-server/$VERSION" > /dev/null 2>&1; then
    echo -e "${GREEN}✅ Server crate is now available on crates.io!${NC}"
    break
  fi
  echo -e "${YELLOW}Attempt $i: Server crate not yet available, waiting...${NC}"
  sleep 30
done

# Check if we exceeded the timeout
if ! curl -f "https://crates.io/api/v1/crates/quetty-server/$VERSION" > /dev/null 2>&1; then
  echo -e "${RED}❌ Server crate never became available - aborting.${NC}"
  exit 1
fi

# Update UI crate dependency to use published version
echo -e "${BLUE}📝 Updating UI crate to use published server crate...${NC}"
# Create backup
cp ui/Cargo.toml ui/Cargo.toml.bak

# Replace path dependency with published version (robust, no tmp artifacts)
sed -Ei 's#^quetty_server[[:space:]]*=.*#quetty_server = { package = "quetty-server", version = "'"$VERSION"'" }#' ui/Cargo.toml

# Clean up any possible tmp files from previous runs
rm -f ui/Cargo.toml.tmp

echo -e "${BLUE}Updated UI Cargo.toml dependency:${NC}"
grep "quetty_server" ui/Cargo.toml

# Validate UI crate with new dependency
echo -e "${BLUE}🔍 Validating UI crate with published dependency...${NC}"
cargo check -p quetty

# Dry run publish UI crate
echo -e "${BLUE}Testing UI crate publishing...${NC}"
cargo publish --dry-run -p quetty

# Publish UI crate
echo -e "${BLUE}Publishing UI crate...${NC}"
cargo publish -p quetty
echo -e "${GREEN}✅ UI crate published successfully!${NC}"

# Restore original Cargo.toml for development
echo -e "${BLUE}🔄 Restoring original UI Cargo.toml for development...${NC}"
mv ui/Cargo.toml.bak ui/Cargo.toml

echo ""
echo -e "${GREEN}🎉 Both crates published to crates.io successfully!${NC}"
echo ""
echo -e "${BLUE}📦 Installation:${NC}"
echo "  cargo install quetty"
echo ""
echo -e "${BLUE}🔗 Crates.io links:${NC}"
echo "  Server: https://crates.io/crates/quetty-server"
echo "  UI: https://crates.io/crates/quetty"
echo ""
echo -e "${BLUE}💡 Next steps:${NC}"
echo "  1. Test installation: cargo install quetty"
echo "  2. Create GitHub release with: git push origin v$VERSION"
echo "  3. Monitor release build at: https://github.com/dawidpereira/quetty/actions"
