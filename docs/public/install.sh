#!/bin/bash
# Vite+ CLI Installer
# https://viteplus.dev/install.sh
#
# Usage:
#   curl -fsSL https://viteplus.dev/install.sh | bash
#
# Environment variables:
#   VITE_VERSION - Version to install (default: latest)
#   VITE_INSTALL_DIR - Installation directory (default: ~/.vite)
#   npm_config_registry - Custom npm registry URL (default: https://registry.npmjs.org)

set -e

VITE_VERSION="${VITE_VERSION:-latest}"
INSTALL_DIR="${VITE_INSTALL_DIR:-$HOME/.vite}"
BIN_DIR="$INSTALL_DIR/bin"
DIST_DIR="$INSTALL_DIR/dist"
# npm registry URL (strip trailing slash if present)
NPM_REGISTRY="${npm_config_registry:-https://registry.npmjs.org}"
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
  local path_line="export PATH=\"$BIN_DIR:\$PATH\""

  if [ -f "$shell_config" ]; then
    if ! grep -q "$BIN_DIR" "$shell_config" 2>/dev/null; then
      echo "" >> "$shell_config"
      echo "# Added by vite-plus installer" >> "$shell_config"
      echo "$path_line" >> "$shell_config"
      return 0
    fi
  fi
  return 1
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
  if [ "$VITE_VERSION" = "latest" ]; then
    info "Fetching latest version..."
    VITE_VERSION=$(get_latest_version)
  fi
  info "Installing vite-plus-cli v${VITE_VERSION}"

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

  # Download and extract native binary from platform package
  local binary_url="${NPM_REGISTRY}/${package_name}/-/vite-plus-cli-${package_suffix}-${VITE_VERSION}.tgz"
  info "Downloading native binary..."
  download_and_extract "$binary_url" "$BIN_DIR" 1 "package/${binary_name}"

  # Make binary executable
  chmod +x "$BIN_DIR/$binary_name"

  # Create a wrapper script named 'vite' that calls the binary with proper env
  cat > "$BIN_DIR/vite" << EOF
#!/bin/bash
# Vite+ CLI wrapper
export VITE_GLOBAL_CLI_JS_SCRIPTS_DIR="$DIST_DIR"
exec "$BIN_DIR/$binary_name" "\$@"
EOF
  chmod +x "$BIN_DIR/vite"

  # Download and extract JS bundle from main package
  local main_url="${NPM_REGISTRY}/vite-plus-cli/-/vite-plus-cli-${VITE_VERSION}.tgz"
  info "Downloading JS scripts..."

  # Create temp directory for extraction
  local temp_dir
  temp_dir=$(mktemp -d)
  download_and_extract "$main_url" "$temp_dir" 1 "package/dist"

  # Copy dist contents to DIST_DIR
  if [ -d "$temp_dir/dist" ]; then
    cp -r "$temp_dir/dist/"* "$DIST_DIR/"
  fi
  rm -rf "$temp_dir"

  success "Vite+ CLI installed to $INSTALL_DIR"

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
      if [ -f "$fish_config" ] && ! grep -q "$BIN_DIR" "$fish_config" 2>/dev/null; then
        echo "" >> "$fish_config"
        echo "# Added by vite-plus installer" >> "$fish_config"
        echo "set -gx PATH $BIN_DIR \$PATH" >> "$fish_config"
        path_added=true
        shell_config="config.fish"
      fi
      ;;
  esac

  if [ "$path_added" = true ]; then
    success "PATH updated in ~/$shell_config"
    echo ""
    echo "  To start using vite, run:"
    echo ""
    echo "    source ~/$shell_config"
    echo ""
    echo "  Or restart your terminal."
  else
    warn "Could not automatically update PATH"
    echo ""
    echo "  Please add the following to your shell profile:"
    echo ""
    echo "    export PATH=\"$BIN_DIR:\$PATH\""
  fi

  echo ""
  echo "  Then run:"
  echo ""
  echo "    vite --version"
  echo ""
}

main "$@"
