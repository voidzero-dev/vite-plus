#!/bin/bash
# Vite+ CLI Installer
# https://viteplus.dev/install.sh
#
# Usage:
#   curl -fsSL https://viteplus.dev/install.sh | bash
#
# Environment variables:
#   VITE_PLUS_VERSION - Version to install (default: latest)
#   VITE_PLUS_INSTALL_DIR - Installation directory (default: ~/.vite-plus)
#   NPM_CONFIG_REGISTRY - Custom npm registry URL (default: https://registry.npmjs.org)

set -e

VITE_PLUS_VERSION="${VITE_PLUS_VERSION:-latest}"
INSTALL_DIR="${VITE_PLUS_INSTALL_DIR:-$HOME/.vite-plus}"
# npm registry URL (strip trailing slash if present)
NPM_REGISTRY="${NPM_CONFIG_REGISTRY:-https://registry.npmjs.org}"
NPM_REGISTRY="${NPM_REGISTRY%/}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info() {
  echo -e "${BLUE}info${NC}: $1"
}

success() {
  echo -e "${GREEN}success${NC}: $1"
}

warn() {
  echo -e "${YELLOW}warn${NC}: $1"
}

error() {
  echo -e "${RED}error${NC}: $1"
  exit 1
}

# Detect platform
detect_platform() {
  local os arch

  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Darwin) os="darwin" ;;
    Linux) os="linux" ;;
    MINGW*|MSYS*|CYGWIN*) os="win32" ;;
    *) error "Unsupported operating system: $os" ;;
  esac

  case "$arch" in
    x86_64|amd64) arch="x64" ;;
    arm64|aarch64) arch="arm64" ;;
    *) error "Unsupported architecture: $arch" ;;
  esac

  echo "${os}-${arch}"
}

# Check for required commands
check_requirements() {
  local missing=()

  if ! command -v curl &> /dev/null; then
    missing+=("curl")
  fi

  if ! command -v tar &> /dev/null; then
    missing+=("tar")
  fi

  if [ ${#missing[@]} -ne 0 ]; then
    error "Missing required commands: ${missing[*]}"
  fi
}

# Get the latest version from npm registry
get_latest_version() {
  local version
  version=$(curl -s "${NPM_REGISTRY}/vite-plus-cli/latest" | grep -o '"version":"[^"]*"' | cut -d'"' -f4)
  if [ -z "$version" ]; then
    error "Failed to fetch latest version from npm registry"
  fi
  echo "$version"
}

# Download and extract file
download_and_extract() {
  local url="$1"
  local dest_dir="$2"
  local strip_components="$3"
  local filter="$4"

  info "Downloading from $url"

  if [ -n "$filter" ]; then
    curl -sL "$url" | tar xz -C "$dest_dir" --strip-components="$strip_components" "$filter" 2>/dev/null || \
    curl -sL "$url" | tar xz -C "$dest_dir" --strip-components="$strip_components"
  else
    curl -sL "$url" | tar xz -C "$dest_dir" --strip-components="$strip_components"
  fi
}

# Add to shell profile
add_to_path() {
  local shell_config="$1"
  local path_to_add="$INSTALL_DIR/current/bin"
  local path_line="export PATH=\"$path_to_add:\$PATH\""

  if [ -f "$shell_config" ]; then
    # Check if already has the current/bin path
    if grep -q "$path_to_add" "$shell_config" 2>/dev/null; then
      return 1
    fi
    echo "" >> "$shell_config"
    echo "# Added by vite-plus installer" >> "$shell_config"
    echo "$path_line" >> "$shell_config"
    return 0
  fi
  return 1
}

# Cleanup old versions, keeping only the most recent ones
cleanup_old_versions() {
  local max_versions=5
  local versions=()

  # List version directories (exclude 'current' symlink)
  for dir in "$INSTALL_DIR"/*/; do
    local name
    name=$(basename "$dir")
    if [ "$name" != "current" ] && [ -d "$dir" ]; then
      versions+=("$dir")
    fi
  done

  local count=${#versions[@]}
  if [ "$count" -le "$max_versions" ]; then
    return 0
  fi

  # Sort by creation time (oldest first) and delete excess
  local sorted_versions
  if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS: use stat -f %B for birth time
    sorted_versions=$(for v in "${versions[@]}"; do
      echo "$(stat -f %B "$v") $v"
    done | sort -n | head -n $((count - max_versions)) | cut -d' ' -f2-)
  else
    # Linux: use stat -c %W for birth time, fallback to %Y (mtime)
    sorted_versions=$(for v in "${versions[@]}"; do
      local btime
      btime=$(stat -c %W "$v" 2>/dev/null)
      if [ "$btime" = "0" ] || [ -z "$btime" ]; then
        btime=$(stat -c %Y "$v")
      fi
      echo "$btime $v"
    done | sort -n | head -n $((count - max_versions)) | cut -d' ' -f2-)
  fi

  # Delete oldest versions
  for old_version in $sorted_versions; do
    info "Removing old version: $(basename "$old_version")"
    rm -rf "$old_version"
  done
}

main() {
  echo ""
  echo "  Vite+ CLI Installer"
  echo ""

  check_requirements

  local platform
  platform=$(detect_platform)
  info "Detected platform: $platform"

  # Get version
  if [ "$VITE_PLUS_VERSION" = "latest" ]; then
    info "Fetching latest version..."
    VITE_PLUS_VERSION=$(get_latest_version)
  fi
  info "Installing vite-plus-cli v${VITE_PLUS_VERSION}"

  # Set up version-specific directories
  VERSION_DIR="$INSTALL_DIR/$VITE_PLUS_VERSION"
  BIN_DIR="$VERSION_DIR/bin"
  DIST_DIR="$VERSION_DIR/dist"
  CURRENT_LINK="$INSTALL_DIR/current"

  # Platform package name mapping (follows napi-rs convention)
  local package_suffix
  case "$platform" in
    darwin-arm64) package_suffix="darwin-arm64" ;;
    darwin-x64)
      warn "darwin-x64 is not currently supported. Only Apple Silicon (darwin-arm64) is supported."
      error "Unsupported platform: $platform"
      ;;
    linux-arm64) package_suffix="linux-arm64-gnu" ;;
    linux-x64) package_suffix="linux-x64-gnu" ;;
    win32-arm64)
      warn "win32-arm64 is not currently supported. Only win32-x64 is supported."
      error "Unsupported platform: $platform"
      ;;
    win32-x64) package_suffix="win32-x64-msvc" ;;
    *) error "Unsupported platform: $platform" ;;
  esac

  local package_name="@voidzero-dev/vite-plus-cli-${package_suffix}"
  local binary_name="vp"
  if [[ "$platform" == win32* ]]; then
    binary_name="vp.exe"
  fi

  # Create directories
  info "Creating directories..."
  mkdir -p "$BIN_DIR" "$DIST_DIR"

  # Download and extract native binary and .node files from platform package
  local platform_url="${NPM_REGISTRY}/${package_name}/-/vite-plus-cli-${package_suffix}-${VITE_PLUS_VERSION}.tgz"
  info "Downloading platform package..."

  # Create temp directory for extraction
  local platform_temp_dir
  platform_temp_dir=$(mktemp -d)
  download_and_extract "$platform_url" "$platform_temp_dir" 1

  # Copy binary to BIN_DIR
  cp "$platform_temp_dir/$binary_name" "$BIN_DIR/"
  chmod +x "$BIN_DIR/$binary_name"

  # Copy .node files to DIST_DIR (delete existing first to avoid system cache issues)
  for node_file in "$platform_temp_dir"/*.node; do
    rm -f "$DIST_DIR/$(basename "$node_file")"
    cp "$node_file" "$DIST_DIR/"
  done
  rm -rf "$platform_temp_dir"

  # Download and extract JS bundle from main package
  local main_url="${NPM_REGISTRY}/vite-plus-cli/-/vite-plus-cli-${VITE_PLUS_VERSION}.tgz"
  info "Downloading JS scripts..."

  # Create temp directory for extraction
  local temp_dir
  temp_dir=$(mktemp -d)
  download_and_extract "$main_url" "$temp_dir" 1

  # Copy dist contents to DIST_DIR
  if [ -d "$temp_dir/dist" ]; then
    cp -r "$temp_dir/dist/"* "$DIST_DIR/"
  fi

  # Copy package.json to VERSION_DIR for devEngines.runtime configuration
  if [ -f "$temp_dir/package.json" ]; then
    cp "$temp_dir/package.json" "$VERSION_DIR/"
  fi
  rm -rf "$temp_dir"

  # Create/update current symlink (use relative path for portability)
  ln -sfn "$VITE_PLUS_VERSION" "$CURRENT_LINK"

  # Cleanup old versions
  cleanup_old_versions

  success "Vite+ CLI installed to $VERSION_DIR"

  # Update PATH
  echo ""
  local path_added=false
  local shell_config=""

  case "$SHELL" in
    */zsh)
      if add_to_path "$HOME/.zshrc"; then
        path_added=true
        shell_config=".zshrc"
      fi
      ;;
    */bash)
      if add_to_path "$HOME/.bashrc"; then
        path_added=true
        shell_config=".bashrc"
      elif add_to_path "$HOME/.bash_profile"; then
        path_added=true
        shell_config=".bash_profile"
      fi
      ;;
    */fish)
      local fish_config="$HOME/.config/fish/config.fish"
      local path_to_add="$INSTALL_DIR/current/bin"
      if [ -f "$fish_config" ]; then
        if grep -q "$path_to_add" "$fish_config" 2>/dev/null; then
          : # Already has current/bin path
        else
          echo "" >> "$fish_config"
          echo "# Added by vite-plus installer" >> "$fish_config"
          echo "set -gx PATH $path_to_add \$PATH" >> "$fish_config"
          path_added=true
          shell_config="config.fish"
        fi
      fi
      ;;
  esac

  if [ "$path_added" = true ]; then
    success "PATH updated in ~/$shell_config"
    echo ""
    echo "  To start using vp, run:"
    echo ""
    echo "    source ~/$shell_config"
    echo ""
    echo "  Or restart your terminal."
  elif [ -n "$shell_config" ]; then
    info "PATH already configured in ~/$shell_config"
  else
    warn "Could not automatically update PATH"
    echo ""
    echo "  Please add the following to your shell profile:"
    echo ""
    echo "    export PATH=\"$INSTALL_DIR/current/bin:\$PATH\""
  fi

  echo ""
  echo "  Then run:"
  echo ""
  echo "    vp --version"
  echo ""
}

main "$@"
