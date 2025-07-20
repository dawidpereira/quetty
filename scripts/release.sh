#!/bin/bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

VERSION=$1
if [ -z "$VERSION" ]; then
  echo -e "${RED}❌ Usage: ./scripts/release.sh <version>${NC}"
  echo -e "${BLUE}Examples:${NC}"
  echo "  ./scripts/release.sh 1.2.0"
  echo "  ./scripts/release.sh 1.2.0-beta.1"
  echo "  ./scripts/release.sh 1.2.0-rc.1"
  echo ""
  echo -e "${BLUE}💡 Tip:${NC} Install cargo-edit for better cross-platform support:"
  echo "  cargo install cargo-edit"
  exit 1
fi

# Validate version format (stricter semantic versioning)
if ! [[ $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?(\+[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$ ]]; then
  echo -e "${RED}❌ Invalid version format${NC}"
  echo -e "${BLUE}Use semantic versioning:${NC} 1.2.0 or 1.2.0-beta.1"
  exit 1
fi

echo -e "${BLUE}🚀 Starting release process for version ${YELLOW}$VERSION${NC}"

# Check if we're on main branch
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$CURRENT_BRANCH" != "main" ]; then
  echo -e "${RED}❌ Not on main branch (currently on: $CURRENT_BRANCH)${NC}"
  echo -e "${BLUE}Please checkout main branch first:${NC} git checkout main"
  exit 1
fi

# Check if working directory is clean
if ! git diff-index --quiet HEAD --; then
  echo -e "${RED}❌ Working directory is not clean${NC}"
  echo -e "${BLUE}Please commit or stash your changes first${NC}"
  exit 1
fi

# Pull latest changes
echo -e "${BLUE}📥 Pulling latest changes...${NC}"
git pull origin main

# Check if tag already exists
if git tag | grep -q "^v$VERSION$"; then
  echo -e "${RED}❌ Tag v$VERSION already exists${NC}"
  exit 1
fi

# Update Cargo.toml files
echo -e "${BLUE}📝 Updating Cargo.toml files...${NC}"

# Check if cargo-edit is available for portable version management
if command -v cargo-set-version >/dev/null 2>&1; then
  echo -e "${BLUE}Using cargo set-version for cross-platform compatibility${NC}"
  cargo set-version --workspace "$VERSION"
else
  echo -e "${YELLOW}cargo-edit not found, using sed (less portable)${NC}"
  # Fallback to awk for better cross-platform compatibility
  tmp=$(mktemp)
  awk -v version="$VERSION" 'NR==3{sub(/^version = ".*"/, "version = \"" version "\"")} 1' ui/Cargo.toml > "$tmp" && mv "$tmp" ui/Cargo.toml
  tmp=$(mktemp)
  awk -v version="$VERSION" 'NR==3{sub(/^version = ".*"/, "version = \"" version "\"")} 1' server/Cargo.toml > "$tmp" && mv "$tmp" server/Cargo.toml
fi

# Verify changes
echo -e "${BLUE}🔍 Verifying version updates...${NC}"
UI_VERSION=$(grep "^version" ui/Cargo.toml | cut -d'"' -f2)
SERVER_VERSION=$(grep "^version" server/Cargo.toml | cut -d'"' -f2)

if [ "$UI_VERSION" != "$VERSION" ] || [ "$SERVER_VERSION" != "$VERSION" ]; then
  echo -e "${RED}❌ Version update failed${NC}"
  echo "Expected: $VERSION"
  echo "UI: $UI_VERSION"
  echo "Server: $SERVER_VERSION"
  exit 1
fi

echo -e "${GREEN}✅ Updated versions:${NC}"
echo "  ui/Cargo.toml: $UI_VERSION"
echo "  server/Cargo.toml: $SERVER_VERSION"

# Test build to ensure version works
echo -e "${BLUE}🔨 Testing build...${NC}"
cargo check --workspace

# Check if there are changes to commit
if git diff-index --quiet HEAD --; then
  echo -e "${YELLOW}⚠️  No changes to commit (versions already match)${NC}"
else
  # Commit version bump (including Cargo.lock for reproducible builds)
  echo -e "${BLUE}📝 Committing version bump...${NC}"
  git add ui/Cargo.toml server/Cargo.toml Cargo.lock
  git commit -m "chore: bump version to $VERSION"
fi

# Create and push tag
echo -e "${BLUE}🏷️  Creating tag v$VERSION...${NC}"
git tag -a "v$VERSION" -m "Release version $VERSION"

# Push changes and tag
echo -e "${BLUE}🚀 Pushing to repository...${NC}"
git push origin main
git push origin "v$VERSION"

echo ""
echo -e "${GREEN}✅ Successfully released version $VERSION!${NC}"
echo ""
echo -e "${BLUE}Next steps:${NC}"
echo "  • GitHub Actions will build release artifacts"
echo "  • Check the release at: https://github.com/dawidpereira/quetty/releases"
echo "  • Monitor the build: https://github.com/dawidpereira/quetty/actions"
echo ""
echo -e "${BLUE}To create the next development version:${NC}"
echo "  git checkout main"
echo "  # Edit Cargo.toml files with next version (e.g., $(echo $VERSION | awk -F. '{print $1"."($2+1)".0-dev"}')"
echo "  git add ui/Cargo.toml server/Cargo.toml"
echo "  git commit -m \"chore: bump to next development version\""
echo "  git push origin main"
