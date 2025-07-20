#!/bin/bash

# Quetty Universal Installation Script
# Supports Linux x64, macOS Intel, and macOS Apple Silicon
# Usage: curl -fsSL https://raw.githubusercontent.com/dawidpereira/quetty/main/install.sh | sh

set -e

# Configuration
REPO_OWNER="dawidpereira"
REPO_NAME="quetty"
GITHUB_API_URL="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}"
INSTALL_DIR=""
VERSION=""
CHANNEL="stable"
DRY_RUN=false
FORCE=false
UNINSTALL=false

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print functions
print_info() {
  printf "${BLUE}INFO:${NC} %s\n" "$1"
}

print_success() {
  printf "${GREEN}SUCCESS:${NC} %s\n" "$1"
}

print_warning() {
  printf "${YELLOW}WARNING:${NC} %s\n" "$1"
}

print_error() {
  printf "${RED}ERROR:${NC} %s\n" "$1" >&2
}

# Help message
show_help() {
  cat <<EOF
Quetty Universal Installation Script

USAGE:
    curl -fsSL https://raw.githubusercontent.com/dawidpereira/quetty/main/install.sh | sh
    curl -fsSL https://raw.githubusercontent.com/dawidpereira/quetty/main/install.sh | sh -s -- [OPTIONS]

OPTIONS:
    --version VERSION       Install specific version (e.g., v0.1.0-alpha.1)
    --channel CHANNEL       Install from channel: stable (default), nightly
    --install-dir DIR       Custom installation directory (default: ~/.local/bin)
    --system               Install system-wide to /usr/local/bin (requires sudo)
    --dry-run              Show what would be installed without executing
    --force                Force reinstall even if already installed
    --uninstall            Remove installed Quetty binary
    --help                 Show this help message

EXAMPLES:
    # Install latest stable release
    curl -fsSL https://raw.githubusercontent.com/dawidpereira/quetty/main/install.sh | sh

    # Install specific version
    curl -fsSL https://raw.githubusercontent.com/dawidpereira/quetty/main/install.sh | sh -s -- --version v0.1.0-alpha.1

    # Install to custom directory
    curl -fsSL https://raw.githubusercontent.com/dawidpereira/quetty/main/install.sh | sh -s -- --install-dir /opt/bin

    # Install system-wide
    curl -fsSL https://raw.githubusercontent.com/dawidpereira/quetty/main/install.sh | sh -s -- --system

    # Install nightly build
    curl -fsSL https://raw.githubusercontent.com/dawidpereira/quetty/main/install.sh | sh -s -- --channel nightly

    # Dry run to see what would happen
    curl -fsSL https://raw.githubusercontent.com/dawidpereira/quetty/main/install.sh | sh -s -- --dry-run

EOF
}

# Parse command line arguments
parse_args() {
  while [[ $# -gt 0 ]]; do
    case $1 in
    --version)
      VERSION="$2"
      shift 2
      ;;
    --channel)
      CHANNEL="$2"
      shift 2
      ;;
    --install-dir)
      INSTALL_DIR="$2"
      shift 2
      ;;
    --system)
      INSTALL_DIR="/usr/local/bin"
      shift
      ;;
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    --force)
      FORCE=true
      shift
      ;;
    --uninstall)
      UNINSTALL=true
      shift
      ;;
    --help)
      show_help
      exit 0
      ;;
    *)
      print_error "Unknown option: $1"
      show_help
      exit 1
      ;;
    esac
  done
}

# Detect platform and architecture
detect_platform() {
  local os=""
  local arch=""

  # Detect OS
  case "$(uname -s)" in
  Linux*)
    os="linux"
    ;;
  Darwin*)
    os="macos"
    ;;
  *)
    print_error "Unsupported operating system: $(uname -s)"
    print_info "Supported platforms: Linux, macOS"
    exit 1
    ;;
  esac

  # Detect architecture
  case "$(uname -m)" in
  x86_64 | amd64)
    arch="x64"
    ;;
  aarch64 | arm64)
    if [[ "$os" == "macos" ]]; then
      arch="arm64"
    else
      print_error "ARM64 Linux is not currently supported"
      print_info "Supported architectures: x64 (Linux), x64/arm64 (macOS)"
      exit 1
    fi
    ;;
  *)
    print_error "Unsupported architecture: $(uname -m)"
    print_info "Supported architectures: x64 (Linux), x64/arm64 (macOS)"
    exit 1
    ;;
  esac

  # Set platform-specific values
  PLATFORM="${os}-${arch}"
  ARTIFACT_NAME="quetty-${PLATFORM}"
  ARCHIVE_EXT="tar.gz"

  print_info "Detected platform: ${PLATFORM}"
}

# Check if command exists
command_exists() {
  command -v "$1" >/dev/null 2>&1
}

# Check prerequisites
check_prerequisites() {
  local missing_deps=()

  # Check for required commands
  if ! command_exists curl; then
    missing_deps+=(curl)
  fi

  if ! command_exists tar; then
    missing_deps+=(tar)
  fi

  # Check for checksum command
  if ! command_exists sha256sum && ! command_exists shasum; then
    missing_deps+=(sha256sum)
  fi

  if [[ ${#missing_deps[@]} -gt 0 ]]; then
    print_error "Missing required dependencies: ${missing_deps[*]}"
    print_info "Please install the missing dependencies and try again"
    exit 1
  fi
}

# Get latest release version
get_latest_version() {
  if [[ "$CHANNEL" == "nightly" ]]; then
    # For nightly, use the nightly-latest tag
    echo "nightly-latest"
  else
    # For stable, get latest non-prerelease
    local latest_url="${GITHUB_API_URL}/releases/latest"
    local latest_tag

    latest_tag=$(curl -fsSL "$latest_url" | grep '"tag_name":' | sed -E 's/.*"tag_name": "([^"]+)".*/\1/')

    if [[ -z "$latest_tag" ]]; then
      print_error "Could not determine latest version"
      exit 1
    fi

    echo "$latest_tag"
  fi
}

# Get download URL for version
get_download_url() {
  local version="$1"
  local filename="${ARTIFACT_NAME}-${version#v}.${ARCHIVE_EXT}"

  if [[ "$version" == "nightly-latest" ]]; then
    # For nightly builds, construct URL directly
    echo "https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${version}/${filename}"
  else
    # For releases, get URL from API
    local release_url="${GITHUB_API_URL}/releases/tags/${version}"
    local download_url

    download_url=$(curl -fsSL "$release_url" | grep "browser_download_url.*${filename}\"" | sed -E 's/.*"browser_download_url": "([^"]+)".*/\1/')

    if [[ -z "$download_url" ]]; then
      print_error "Could not find download URL for ${filename} in ${version}"
      print_info "Available assets:"
      curl -fsSL "$release_url" | grep "browser_download_url" | sed -E 's/.*"browser_download_url": "([^"]+)".*/\1/' | xargs -I {} basename {}
      exit 1
    fi

    echo "$download_url"
  fi
}

# Get checksum URL
get_checksum_url() {
  local version="$1"
  local checksum_filename="${ARTIFACT_NAME}-${version#v}.${ARCHIVE_EXT}.sha256"

  if [[ "$version" == "nightly-latest" ]]; then
    echo "https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${version}/${checksum_filename}"
  else
    local release_url="${GITHUB_API_URL}/releases/tags/${version}"
    local checksum_url

    checksum_url=$(curl -fsSL "$release_url" | grep "browser_download_url.*${checksum_filename}" | sed -E 's/.*"browser_download_url": "([^"]+)".*/\1/')

    echo "$checksum_url"
  fi
}

# Set installation directory
set_install_dir() {
  if [[ -z "$INSTALL_DIR" ]]; then
    INSTALL_DIR="$HOME/.local/bin"
  fi

  # Expand tilde
  INSTALL_DIR="${INSTALL_DIR/#\~/$HOME}"

  print_info "Installation directory: ${INSTALL_DIR}"
}

# Check if Quetty is already installed
check_existing_installation() {
  local quetty_path="${INSTALL_DIR}/quetty"

  if [[ -f "$quetty_path" ]] && [[ ! "$FORCE" == true ]]; then
    print_warning "Quetty is already installed at ${quetty_path}"

    if command_exists "$quetty_path"; then
      local current_version
      current_version=$("$quetty_path" --version 2>/dev/null | head -n1 || echo "unknown")
      print_info "Current version: ${current_version}"
    fi

    print_info "Use --force to reinstall or --uninstall to remove"
    exit 1
  fi
}

# Create installation directory
create_install_dir() {
  if [[ ! -d "$INSTALL_DIR" ]]; then
    print_info "Creating installation directory: ${INSTALL_DIR}"

    if [[ "$DRY_RUN" == false ]]; then
      mkdir -p "$INSTALL_DIR" || {
        print_error "Failed to create installation directory: ${INSTALL_DIR}"
        print_info "You may need to run with sudo or choose a different directory"
        exit 1
      }
    fi
  fi
}

# Download and verify file
download_and_verify() {
  local url="$1"
  local output_file="$2"
  local checksum_url="$3"

  print_info "Downloading: $(basename "$output_file")"

  if [[ "$DRY_RUN" == false ]]; then
    curl -fsSL --progress-bar "$url" -o "$output_file" || {
      print_error "Failed to download ${url}"
      exit 1
    }
  fi

  # Verify checksum if available
  if [[ -n "$checksum_url" ]]; then
    print_info "Verifying checksum..."

    if [[ "$DRY_RUN" == false ]]; then
      local expected_checksum
      expected_checksum=$(curl -fsSL "$checksum_url" 2>/dev/null | awk '{print $1}' || echo "")

      if [[ -n "$expected_checksum" ]]; then
        local actual_checksum

        if command_exists sha256sum; then
          actual_checksum=$(sha256sum "$output_file" | awk '{print $1}')
        else
          actual_checksum=$(shasum -a 256 "$output_file" | awk '{print $1}')
        fi

        if [[ "$expected_checksum" == "$actual_checksum" ]]; then
          print_success "Checksum verification passed"
        else
          print_error "Checksum verification failed"
          print_error "Expected: ${expected_checksum}"
          print_error "Actual:   ${actual_checksum}"
          exit 1
        fi
      else
        print_warning "Could not retrieve checksum for verification"
      fi
    fi
  else
    print_warning "No checksum available for verification"
  fi
}

# Extract and install
extract_and_install() {
  local archive_file="$1"
  local temp_dir="$2"

  print_info "Extracting archive..."

  if [[ "$DRY_RUN" == false ]]; then
    tar -xzf "$archive_file" -C "$temp_dir" || {
      print_error "Failed to extract ${archive_file}"
      exit 1
    }

    # Find the binary (should be named after artifact)
    local binary_path="${temp_dir}/${ARTIFACT_NAME}"

    if [[ ! -f "$binary_path" ]]; then
      print_error "Binary not found in archive: ${ARTIFACT_NAME}"
      print_info "Archive contents:"
      tar -tzf "$archive_file" | head -10
      exit 1
    fi

    # Install binary
    print_info "Installing to: ${INSTALL_DIR}/quetty"

    cp "$binary_path" "${INSTALL_DIR}/quetty" || {
      print_error "Failed to copy binary to ${INSTALL_DIR}"
      exit 1
    }

    chmod +x "${INSTALL_DIR}/quetty" || {
      print_error "Failed to make binary executable"
      exit 1
    }
  fi
}

# Check PATH
check_path() {
  if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    print_warning "Installation directory is not in PATH: ${INSTALL_DIR}"
    print_info "Add the following to your shell profile (.bashrc, .zshrc, etc.):"
    print_info "export PATH=\"${INSTALL_DIR}:\$PATH\""
    print_info ""
    print_info "Or run: echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.bashrc"
    print_info "Then restart your shell or run: source ~/.bashrc"
  fi
}

# Uninstall function
uninstall_quetty() {
  local quetty_path="${INSTALL_DIR}/quetty"

  if [[ -f "$quetty_path" ]]; then
    print_info "Removing Quetty from: ${quetty_path}"

    if [[ "$DRY_RUN" == false ]]; then
      rm "$quetty_path" || {
        print_error "Failed to remove ${quetty_path}"
        exit 1
      }
    fi

    print_success "Quetty has been uninstalled"
  else
    print_warning "Quetty not found at: ${quetty_path}"
  fi

  exit 0
}

# Main installation function
main() {
  print_info "Quetty Universal Installation Script"
  print_info "======================================"

  # Parse arguments
  parse_args "$@"

  # Set installation directory
  set_install_dir

  # Handle uninstall
  if [[ "$UNINSTALL" == true ]]; then
    uninstall_quetty
  fi

  # Detect platform
  detect_platform

  # Check prerequisites
  check_prerequisites

  # Check existing installation
  check_existing_installation

  # Determine version
  if [[ -z "$VERSION" ]]; then
    print_info "Determining latest ${CHANNEL} version..."
    VERSION=$(get_latest_version)
  fi

  print_info "Installing Quetty ${VERSION} for ${PLATFORM}"

  # Get download URLs
  DOWNLOAD_URL=$(get_download_url "$VERSION")
  CHECKSUM_URL=$(get_checksum_url "$VERSION")

  print_info "Download URL: ${DOWNLOAD_URL}"

  if [[ "$DRY_RUN" == true ]]; then
    print_info "DRY RUN - The following actions would be performed:"
    print_info "1. Create directory: ${INSTALL_DIR}"
    print_info "2. Download: ${DOWNLOAD_URL}"
    if [[ -n "$CHECKSUM_URL" ]]; then
      print_info "3. Verify checksum from: ${CHECKSUM_URL}"
    fi
    print_info "4. Extract and install binary to: ${INSTALL_DIR}/quetty"
    print_info "5. Set executable permissions"
    check_path
    exit 0
  fi

  # Create temporary directory
  TEMP_DIR=$(mktemp -d)
  trap "rm -rf '$TEMP_DIR'" EXIT

  # Create installation directory
  create_install_dir

  # Download and verify
  ARCHIVE_FILE="${TEMP_DIR}/$(basename "$DOWNLOAD_URL")"
  download_and_verify "$DOWNLOAD_URL" "$ARCHIVE_FILE" "$CHECKSUM_URL"

  # Extract and install
  extract_and_install "$ARCHIVE_FILE" "$TEMP_DIR"

  # Check installation
  if command -v "${INSTALL_DIR}/quetty" >/dev/null 2>&1; then
    local installed_version
    installed_version=$("${INSTALL_DIR}/quetty" --version 2>/dev/null | head -n1 || echo "unknown")
    print_success "Quetty installed successfully!"
    print_info "Version: ${installed_version}"
    print_info "Location: ${INSTALL_DIR}/quetty"
  else
    print_error "Installation verification failed"
    exit 1
  fi

  # Check PATH
  check_path

  print_info ""
  print_success "Installation complete! You can now run 'quetty' to get started."
  print_info "Run 'quetty --help' for usage information."
}

# Run main function with all arguments
main "$@"
