#!/bin/bash
# Vite+ CLI Installer
# https://viteplus.dev/install.sh
#
# Usage:
#   curl -fsSL https://viteplus.dev/install.sh | bash
#
# Environment variables:
#   VITE_PLUS_VERSION - Version to install (default: latest)
#   VITE_PLUS_HOME - Installation directory (default: ~/.vite-plus)
#   NPM_CONFIG_REGISTRY - Custom npm registry URL (default: https://registry.npmjs.org)
#   VITE_PLUS_LOCAL_BINARY - Path to locally built binary (for development/testing)
#   VITE_PLUS_LOCAL_PACKAGE - Path to local vite-plus-cli package dir (for development/testing)

set -e

VITE_PLUS_VERSION="${VITE_PLUS_VERSION:-latest}"
INSTALL_DIR="${VITE_PLUS_HOME:-$HOME/.vite-plus}"
# npm registry URL (strip trailing slash if present)
NPM_REGISTRY="${NPM_CONFIG_REGISTRY:-https://registry.npmjs.org}"
NPM_REGISTRY="${NPM_REGISTRY%/}"
# Local paths for development/testing
LOCAL_BINARY="${VITE_PLUS_LOCAL_BINARY:-}"
LOCAL_PACKAGE="${VITE_PLUS_LOCAL_PACKAGE:-}"

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

# Print user-friendly error message for curl failures
# Arguments: exit_code url
print_curl_error() {
  local exit_code="$1"
  local url="$2"

  # Map curl exit codes to user-friendly messages
  local error_desc
  case $exit_code in
    6)
      error_desc="DNS resolution failed - could not resolve hostname"
      ;;
    7)
      error_desc="Connection refused - the server may be down or unreachable"
      ;;
    28)
      error_desc="Connection timed out"
      ;;
    35)
      error_desc="SSL/TLS connection error"
      ;;
    60)
      error_desc="SSL certificate verification failed"
      ;;
    *)
      error_desc="Network error"
      ;;
  esac

  echo ""
  echo -e "${RED}error${NC}: ${error_desc} (curl exit code ${exit_code})"
  echo ""
  echo "  This may be caused by:"
  echo "    - Network connectivity issues"
  echo "    - Firewall or proxy blocking the connection"
  echo "    - DNS configuration problems"
  if [ $exit_code -eq 35 ] || [ $exit_code -eq 60 ]; then
    echo "    - Outdated SSL/TLS libraries"
  fi
  echo ""
  if [ -n "$url" ]; then
    echo "  Failed URL: $url"
    echo ""
    echo "  To debug, run:"
    echo "    curl -v \"$url\""
    echo ""
  fi
  exit 1
}

# Wrapper for curl with user-friendly error messages
# Arguments: same as curl
# Returns: exits with error message on failure, otherwise returns curl output
curl_with_error_handling() {
  local url=""
  local args=()

  # Parse arguments to find the URL (for error messages)
  for arg in "$@"; do
    case "$arg" in
      http://*|https://*)
        url="$arg"
        ;;
    esac
    args+=("$arg")
  done

  # Run curl and capture exit code
  set +e
  local output exit_code
  output=$(curl "${args[@]}" 2>&1)
  exit_code=$?
  set -e

  if [ $exit_code -eq 0 ]; then
    echo "$output"
    return 0
  fi

  print_curl_error "$exit_code" "$url"
}

# Detect libc type on Linux (gnu or musl)
detect_libc() {
  # Check for musl dynamic linker (most reliable method)
  if [ -e /lib/ld-musl-x86_64.so.1 ] || [ -e /lib/ld-musl-aarch64.so.1 ]; then
    echo "musl"
    return
  fi

  # Check if ldd exists and is musl-based
  if command -v ldd &> /dev/null; then
    if ldd --version 2>&1 | grep -qi musl; then
      echo "musl"
      return
    fi
  fi

  # Default to gnu (glibc)
  echo "gnu"
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

  # For Linux, append libc type to distinguish gnu vs musl
  if [ "$os" = "linux" ]; then
    local libc
    libc=$(detect_libc)
    echo "${os}-${arch}-${libc}"
  else
    echo "${os}-${arch}"
  fi
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

# Fetch package metadata from npm registry (cached for reuse)
# Uses VITE_PLUS_VERSION to fetch the correct version's metadata
PACKAGE_METADATA=""
fetch_package_metadata() {
  if [ -z "$PACKAGE_METADATA" ]; then
    local version_path metadata_url
    if [ "$VITE_PLUS_VERSION" = "latest" ]; then
      version_path="latest"
    else
      version_path="$VITE_PLUS_VERSION"
    fi
    metadata_url="${NPM_REGISTRY}/vite-plus-cli/${version_path}"
    PACKAGE_METADATA=$(curl_with_error_handling -s "$metadata_url")
    if [ -z "$PACKAGE_METADATA" ]; then
      error "Failed to fetch package metadata from: $metadata_url"
    fi
    # Check for npm registry error response
    # npm can return either {"error":"..."} or a plain JSON string like "version not found: test"
    if echo "$PACKAGE_METADATA" | grep -q '"error"'; then
      local error_msg
      error_msg=$(echo "$PACKAGE_METADATA" | grep -o '"error":"[^"]*"' | cut -d'"' -f4)
      error "Failed to fetch version '${version_path}': ${error_msg:-unknown error}"
    fi
    # Check if response is a plain error string (not a valid package object)
    # Use '"version":' to match JSON property, not just the word "version"
    if ! echo "$PACKAGE_METADATA" | grep -q '"version":'; then
      # Remove surrounding quotes from the error message if present
      local error_msg
      error_msg=$(echo "$PACKAGE_METADATA" | sed 's/^"//;s/"$//')
      error "Failed to fetch version '${version_path}': ${error_msg:-unknown error}"
    fi
  fi
  # PACKAGE_METADATA is set as a global variable, no need to echo
}

# Get the version from package metadata
# Sets RESOLVED_VERSION global variable
get_version_from_metadata() {
  # Call fetch_package_metadata to populate PACKAGE_METADATA global
  # Don't use command substitution as it would swallow the exit from error()
  fetch_package_metadata
  RESOLVED_VERSION=$(echo "$PACKAGE_METADATA" | grep -o '"version":"[^"]*"' | head -1 | cut -d'"' -f4)
  if [ -z "$RESOLVED_VERSION" ]; then
    error "Failed to extract version from package metadata"
  fi
}

# Get package suffix for platform from optionalDependencies
# Sets PACKAGE_SUFFIX global variable
# Platform format: darwin-arm64, darwin-x64, linux-x64, linux-arm64, win32-x64, etc.
# Package format: @voidzero-dev/vite-plus-cli-darwin-arm64, @voidzero-dev/vite-plus-cli-linux-x64-gnu, etc.
get_package_suffix() {
  local platform="$1"
  local matching_package

  # Call fetch_package_metadata to populate PACKAGE_METADATA global
  # Don't use command substitution as it would swallow the exit from error()
  fetch_package_metadata

  # Extract optionalDependencies keys that match the platform
  # Look for packages like @voidzero-dev/vite-plus-cli-{platform}[-suffix]
  matching_package=$(echo "$PACKAGE_METADATA" | grep -o "\"@voidzero-dev/vite-plus-cli-${platform}[^\"]*\"" | head -1 | tr -d '"')

  if [ -z "$matching_package" ]; then
    # List available platforms for helpful error message
    local available_platforms
    available_platforms=$(echo "$PACKAGE_METADATA" | grep -o '"@voidzero-dev/vite-plus-cli-[^"]*"' | sed 's/"@voidzero-dev\/vite-plus-cli-//g' | tr -d '"' | tr '\n' ', ' | sed 's/,$//')
    error "Unsupported platform: $platform. Available platforms: $available_platforms"
  fi

  # Extract suffix by removing the package prefix
  PACKAGE_SUFFIX="${matching_package#@voidzero-dev/vite-plus-cli-}"
}

# Download and extract file (silent mode - no progress bar)
download_and_extract() {
  local url="$1"
  local dest_dir="$2"
  local strip_components="$3"
  local filter="$4"

  # Download to temp file (silent mode)
  local temp_file
  temp_file=$(mktemp)

  # Run curl and capture exit code for error handling
  set +e
  curl -sL "$url" -o "$temp_file"
  local exit_code=$?
  set -e

  if [ $exit_code -ne 0 ]; then
    rm -f "$temp_file"
    print_curl_error "$exit_code" "$url"
  fi

  if [ -n "$filter" ]; then
    tar xzf "$temp_file" -C "$dest_dir" --strip-components="$strip_components" "$filter" 2>/dev/null || \
    tar xzf "$temp_file" -C "$dest_dir" --strip-components="$strip_components"
  else
    tar xzf "$temp_file" -C "$dest_dir" --strip-components="$strip_components"
  fi
  rm -f "$temp_file"
}

# Add to shell profile
# Returns: 0 = path added, 1 = file not found, 2 = path already exists
add_to_path() {
  local shell_config="$1"
  local path_to_add="$INSTALL_DIR/current/bin"
  local path_line="export PATH=\"$path_to_add:\$PATH\""

  if [ -f "$shell_config" ]; then
    # Check if already has the current/bin path
    if grep -q "$path_to_add" "$shell_config" 2>/dev/null; then
      return 2
    fi
    echo "" >> "$shell_config"
    echo "# Added by vite-plus installer" >> "$shell_config"
    echo "$path_line" >> "$shell_config"
    return 0
  fi
  return 1
}

# Add shims to shell profile
# Returns: 0 = path added, 1 = file not found, 2 = path already exists
add_shims_to_path() {
  local shell_config="$1"
  local shims_path="$INSTALL_DIR/shims"
  local path_line="export PATH=\"$shims_path:\$PATH\""

  if [ -f "$shell_config" ]; then
    # Check if already has the shims path
    if grep -q "$shims_path" "$shell_config" 2>/dev/null; then
      return 2
    fi
    echo "" >> "$shell_config"
    echo "# Vite-plus Node.js shims" >> "$shell_config"
    echo "$path_line" >> "$shell_config"
    return 0
  fi
  return 1
}

# Configure shims path for the current shell
# Returns: 0 = path added, 1 = file not found, 2 = path already exists
configure_shell_shims_path() {
  local shims_path="$INSTALL_DIR/shims"
  local result=1

  case "$SHELL" in
    */zsh)
      add_shims_to_path "$HOME/.zshrc" || result=$?
      ;;
    */bash)
      add_shims_to_path "$HOME/.bashrc" || result=$?
      if [ $result -eq 1 ]; then
        result=0
        add_shims_to_path "$HOME/.bash_profile" || result=$?
      fi
      ;;
    */fish)
      local fish_config="$HOME/.config/fish/config.fish"
      if [ -f "$fish_config" ]; then
        if grep -q "$shims_path" "$fish_config" 2>/dev/null; then
          result=2
        else
          echo "" >> "$fish_config"
          echo "# Vite-plus Node.js shims" >> "$fish_config"
          echo "set -gx PATH $shims_path \$PATH" >> "$fish_config"
          result=0
        fi
      fi
      ;;
  esac

  return $result
}

# Setup shims PATH - auto-enables if no node detected, otherwise prompts user
# Sets SHIMS_PATH_ADDED global variable
# Arguments: bin_dir - path to the bin directory containing vp
setup_shims_path() {
  local bin_dir="$1"
  local shims_path="$INSTALL_DIR/shims"
  SHIMS_PATH_ADDED="false"

  # Check if already in PATH
  if echo "$PATH" | tr ':' '\n' | grep -qx "$shims_path"; then
    # Refresh shims if already configured
    "$bin_dir/vp" env setup --refresh > /dev/null
    SHIMS_PATH_ADDED="already"
    return 0
  fi

  # Check if node is available on the system
  local node_available="false"
  if command -v node &> /dev/null; then
    node_available="true"
  fi

  # Auto-enable shims if node is not available (no prompt needed)
  if [ "$node_available" = "false" ]; then
    "$bin_dir/vp" env setup --refresh > /dev/null

    local path_result=0
    configure_shell_shims_path || path_result=$?

    if [ $path_result -eq 0 ]; then
      SHIMS_PATH_ADDED="true"
    elif [ $path_result -eq 2 ]; then
      SHIMS_PATH_ADDED="already"
    fi
    return 0
  fi

  # Prompt user (only in interactive mode, not CI)
  if [ -t 0 ] && [ -z "$CI" ]; then
    echo ""
    echo "Would you want Vite+ to manage Node.js versions?"
    # echo "This adds 'node', 'npm', and 'npx' shims to your PATH."
    echo -n "Press Enter to accept (Y/n): "
    read -r add_shims < /dev/tty

    if [ -z "$add_shims" ] || [ "$add_shims" = "y" ] || [ "$add_shims" = "Y" ]; then
      "$bin_dir/vp" env setup --refresh > /dev/null

      local path_result=0
      configure_shell_shims_path || path_result=$?

      if [ $path_result -eq 0 ]; then
        SHIMS_PATH_ADDED="true"
      elif [ $path_result -eq 2 ]; then
        SHIMS_PATH_ADDED="already"
      fi
    fi
  fi
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

  # Delete oldest versions (silently)
  for old_version in $sorted_versions; do
    rm -rf "$old_version"
  done
}

# Setup PATH - try ~/.local/bin symlink first, fallback to shell profile
# Returns via global variables:
#   SYMLINK_CREATED - "true" if symlink was created, "false" otherwise
#   SHELL_CONFIG_UPDATED - shell config file name if updated, empty otherwise
#   PATH_ALREADY_CONFIGURED - "true" if PATH was already set up
setup_path() {
  local local_bin="$HOME/.local/bin"
  local path_to_add="$INSTALL_DIR/current/bin"

  SYMLINK_CREATED="false"
  SHELL_CONFIG_UPDATED=""
  PATH_ALREADY_CONFIGURED="false"

  # Check if ~/.local/bin is in PATH
  if echo "$PATH" | tr ':' '\n' | grep -qx "$local_bin"; then
    # Create ~/.local/bin if it doesn't exist
    mkdir -p "$local_bin"
    # Create symlink (force overwrite if exists)
    ln -sf "$INSTALL_DIR/current/bin/vp" "$local_bin/vp"
    SYMLINK_CREATED="true"
    return 0
  fi

  # Fall back to adding to shell profile
  local path_result=0  # 0=added, 1=failed, 2=already exists

  case "$SHELL" in
    */zsh)
      add_to_path "$HOME/.zshrc" || path_result=$?
      [ $path_result -ne 1 ] && SHELL_CONFIG_UPDATED=".zshrc"
      ;;
    */bash)
      add_to_path "$HOME/.bashrc" || path_result=$?
      if [ $path_result -ne 1 ]; then
        SHELL_CONFIG_UPDATED=".bashrc"
      else
        path_result=0
        add_to_path "$HOME/.bash_profile" || path_result=$?
        [ $path_result -ne 1 ] && SHELL_CONFIG_UPDATED=".bash_profile"
      fi
      ;;
    */fish)
      local fish_config="$HOME/.config/fish/config.fish"
      if [ -f "$fish_config" ]; then
        if grep -q "$path_to_add" "$fish_config" 2>/dev/null; then
          path_result=2
          SHELL_CONFIG_UPDATED="config.fish"
        else
          echo "" >> "$fish_config"
          echo "# Added by vite-plus installer" >> "$fish_config"
          echo "set -gx PATH $path_to_add \$PATH" >> "$fish_config"
          path_result=0
          SHELL_CONFIG_UPDATED="config.fish"
        fi
      fi
      ;;
  esac

  if [ $path_result -eq 2 ]; then
    PATH_ALREADY_CONFIGURED="true"
  fi
}

main() {
  echo ""
  echo "Setting up VITE+(⚡︎)..."
  echo ""

  check_requirements

  local platform
  platform=$(detect_platform)

  # Local development mode: skip npm entirely
  if [ -n "$LOCAL_BINARY" ] && [ -n "$LOCAL_PACKAGE" ]; then
    # Validate local paths
    if [ ! -f "$LOCAL_BINARY" ]; then
      error "Local binary not found: $LOCAL_BINARY"
    fi
    if [ ! -d "$LOCAL_PACKAGE" ]; then
      error "Local package directory not found: $LOCAL_PACKAGE"
    fi
    # Use version as-is (default to "local-dev")
    if [ "$VITE_PLUS_VERSION" = "latest" ]; then
      VITE_PLUS_VERSION="local-dev"
    fi
  else
    # Fetch package metadata and resolve version from npm
    get_version_from_metadata
    VITE_PLUS_VERSION="$RESOLVED_VERSION"
  fi

  # Set up version-specific directories
  VERSION_DIR="$INSTALL_DIR/$VITE_PLUS_VERSION"
  BIN_DIR="$VERSION_DIR/bin"
  DIST_DIR="$VERSION_DIR/dist"
  CURRENT_LINK="$INSTALL_DIR/current"

  local binary_name="vp"
  if [[ "$platform" == win32* ]]; then
    binary_name="vp.exe"
  fi

  # Create directories
  mkdir -p "$BIN_DIR" "$DIST_DIR"

  # Download and extract native binary and .node files from platform package
  if [ -n "$LOCAL_BINARY" ]; then
    # Use local binary for development/testing
    info "Using local binary: $LOCAL_BINARY"
    cp "$LOCAL_BINARY" "$BIN_DIR/$binary_name"
    chmod +x "$BIN_DIR/$binary_name"
    # Note: .node files won't be available when using local binary
  else
    # Get package suffix from optionalDependencies (dynamic lookup)
    get_package_suffix "$platform"
    local package_name="@voidzero-dev/vite-plus-cli-${PACKAGE_SUFFIX}"
    local platform_url="${NPM_REGISTRY}/${package_name}/-/vite-plus-cli-${PACKAGE_SUFFIX}-${VITE_PLUS_VERSION}.tgz"

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
  fi

  # Copy JS bundle and assets from local package or download from npm
  local items_to_copy=("dist" "templates" "rules" "AGENTS.md" "package.json")
  if [ -n "$LOCAL_PACKAGE" ]; then
    # Use local package for development/testing
    info "Using local package: $LOCAL_PACKAGE"
    for item in "${items_to_copy[@]}"; do
      if [ -e "$LOCAL_PACKAGE/$item" ]; then
        cp -r "$LOCAL_PACKAGE/$item" "$VERSION_DIR/"
      fi
    done
  else
    # Download and extract from npm
    local main_url="${NPM_REGISTRY}/vite-plus-cli/-/vite-plus-cli-${VITE_PLUS_VERSION}.tgz"

    # Create temp directory for extraction
    local temp_dir
    temp_dir=$(mktemp -d)
    download_and_extract "$main_url" "$temp_dir" 1

    for item in "${items_to_copy[@]}"; do
      if [ -e "$temp_dir/$item" ]; then
        cp -r "$temp_dir/$item" "$VERSION_DIR/"
      fi
    done
    rm -rf "$temp_dir"
  fi

  # Skip dependency installation for local package (deps already bundled or available)
  if [ -z "$LOCAL_PACKAGE" ]; then
    # Remove devDependencies and optionalDependencies from package.json
    # (temporary solution until deps are fully bundled)
    local pkg_file="$VERSION_DIR/package.json"
    awk '
      /"(devDependencies|optionalDependencies)"[[:space:]]*:[[:space:]]*\{/ {
        skip = 1
        depth = 1
        next
      }
      skip {
        for (i = 1; i <= length($0); i++) {
          c = substr($0, i, 1)
          if (c == "{") depth++
          else if (c == "}") depth--
        }
        if (depth <= 0) skip = 0
        next
      }
      { print }
    ' "$pkg_file" > "$pkg_file.tmp" && mv "$pkg_file.tmp" "$pkg_file"

    # Install production dependencies
    (cd "$VERSION_DIR" && CI=true "$BIN_DIR/vp" install --silent)
  fi

  # Create/update current symlink (use relative path for portability)
  ln -sfn "$VITE_PLUS_VERSION" "$CURRENT_LINK"

  # Cleanup old versions
  cleanup_old_versions

  # Setup PATH (sets SYMLINK_CREATED, SHELL_CONFIG_UPDATED, PATH_ALREADY_CONFIGURED)
  setup_path

  # Ask user if they want shims and set them up
  setup_shims_path "$BIN_DIR"

  # Determine display location based on how PATH was configured
  local display_location
  if [ "$SYMLINK_CREATED" = "true" ]; then
    display_location="~/.local/bin/vp"
  else
    # Use ~ shorthand if install dir is under HOME, otherwise show full path
    local display_dir="${INSTALL_DIR/#$HOME/~}"
    display_location="${display_dir}/current/bin"
  fi

  # Print success message
  echo ""
  echo -e "${GREEN}✔${NC} VITE+(⚡︎) successfully installed!"
  echo ""
  echo "  Version: ${VITE_PLUS_VERSION}"
  echo ""
  echo "  Location: ${display_location}"

  if [ "$SHIMS_PATH_ADDED" = "true" ] || [ "$SHIMS_PATH_ADDED" = "already" ]; then
    echo ""
    echo "  Node.js manager: on"
    # Show note about shims if added
    if [ "$SHIMS_PATH_ADDED" = "true" ]; then
      echo "  Restart your terminal and IDE, then run 'vp env doctor' to verify."
    fi
  fi

  echo ""
  echo "  Next: Run 'vp help' to get started"

  # Show note if shell config was updated (not symlink, not already configured)
  if [ "$SYMLINK_CREATED" = "false" ] && [ -n "$SHELL_CONFIG_UPDATED" ] && [ "$PATH_ALREADY_CONFIGURED" = "false" ]; then
    echo ""
    echo "  Note: Run \`source ~/$SHELL_CONFIG_UPDATED\` or restart your terminal."
  fi

  echo ""
}

main "$@"
