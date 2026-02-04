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
#   VITE_PLUS_LOCAL_TGZ - Path to local vite-plus-cli.tgz (for development/testing)

set -e

VITE_PLUS_VERSION="${VITE_PLUS_VERSION:-latest}"
INSTALL_DIR="${VITE_PLUS_HOME:-$HOME/.vite-plus}"
# npm registry URL (strip trailing slash if present)
NPM_REGISTRY="${NPM_CONFIG_REGISTRY:-https://registry.npmjs.org}"
NPM_REGISTRY="${NPM_REGISTRY%/}"
# Local tarball for development/testing
LOCAL_TGZ="${VITE_PLUS_LOCAL_TGZ:-}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
BRIGHT_BLUE='\033[0;94m'
BOLD='\033[1m'
DIM='\033[2m'
BOLD_BRIGHT_BLUE='\033[1;94m'
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

# Add bin to shell profile
# Returns: 0 = path added, 1 = file not found, 2 = path already exists
add_bin_to_path() {
  local shell_config="$1"
  local bin_path="$INSTALL_DIR/bin"
  local path_line="export PATH=\"$bin_path:\$PATH\""

  if [ -f "$shell_config" ]; then
    # Check if already has the bin path
    if grep -q "$bin_path" "$shell_config" 2>/dev/null; then
      return 2
    fi
    echo "" >> "$shell_config"
    echo "# Vite+ bin (https://viteplus.dev)" >> "$shell_config"
    echo "$path_line" >> "$shell_config"
    return 0
  fi
  return 1
}

# Configure shell PATH for ~/.vite-plus/bin
# Sets PATH_CONFIGURED and SHELL_CONFIG_UPDATED globals
configure_shell_path() {
  local bin_path="$INSTALL_DIR/bin"
  PATH_CONFIGURED="false"
  SHELL_CONFIG_UPDATED=""

  # Check if already in PATH
  if echo "$PATH" | tr ':' '\n' | grep -qx "$bin_path"; then
    PATH_CONFIGURED="already"
    return 0
  fi

  local result=1  # Default to failure - must explicitly set success
  case "$SHELL" in
    */zsh)
      # Add to both .zshenv (for all shells including IDE) and .zshrc (to ensure PATH is at front)
      local zshenv_result=0 zshrc_result=0
      add_bin_to_path "$HOME/.zshenv" || zshenv_result=$?
      add_bin_to_path "$HOME/.zshrc" || zshrc_result=$?
      # Prioritize .zshrc for user notification (easier to source)
      if [ $zshrc_result -eq 0 ]; then
        result=0
        SHELL_CONFIG_UPDATED=".zshrc"
      elif [ $zshenv_result -eq 0 ]; then
        result=0
        SHELL_CONFIG_UPDATED=".zshenv"
      elif [ $zshenv_result -eq 2 ] || [ $zshrc_result -eq 2 ]; then
        result=2  # already configured in at least one file
      fi
      ;;
    */bash)
      # Add to .bash_profile, .bashrc, AND .profile for maximum compatibility
      # - .bash_profile: login shells (macOS default)
      # - .bashrc: interactive non-login shells (Linux default)
      # - .profile: fallback for systems without .bash_profile (Ubuntu minimal, etc.)
      local bash_profile_result=0 bashrc_result=0 profile_result=0
      add_bin_to_path "$HOME/.bash_profile" || bash_profile_result=$?
      add_bin_to_path "$HOME/.bashrc" || bashrc_result=$?
      add_bin_to_path "$HOME/.profile" || profile_result=$?
      # Prioritize .bashrc for user notification (most commonly edited)
      if [ $bashrc_result -eq 0 ]; then
        result=0
        SHELL_CONFIG_UPDATED=".bashrc"
      elif [ $bash_profile_result -eq 0 ]; then
        result=0
        SHELL_CONFIG_UPDATED=".bash_profile"
      elif [ $profile_result -eq 0 ]; then
        result=0
        SHELL_CONFIG_UPDATED=".profile"
      elif [ $bash_profile_result -eq 2 ] || [ $bashrc_result -eq 2 ] || [ $profile_result -eq 2 ]; then
        result=2  # already configured in at least one file
      fi
      ;;
    */fish)
      local fish_config="$HOME/.config/fish/config.fish"
      if [ -f "$fish_config" ]; then
        if grep -q "$bin_path" "$fish_config" 2>/dev/null; then
          result=2
        else
          echo "" >> "$fish_config"
          echo "# Vite+ bin (https://viteplus.dev)" >> "$fish_config"
          echo "set -gx PATH $bin_path \$PATH" >> "$fish_config"
          result=0
          SHELL_CONFIG_UPDATED="config.fish"
        fi
      fi
      ;;
  esac

  if [ $result -eq 0 ]; then
    PATH_CONFIGURED="true"
  elif [ $result -eq 2 ]; then
    PATH_CONFIGURED="already"
  fi
  # If result is still 1, PATH_CONFIGURED remains "false" (set at function start)
}

# Setup Node.js version manager (node/npm/npx shims)
# Sets NODE_MANAGER_ENABLED global
# Arguments: bin_dir - path to the version's bin directory containing vp
setup_node_manager() {
  local bin_dir="$1"
  local bin_path="$INSTALL_DIR/bin"
  NODE_MANAGER_ENABLED="false"

  # Check if Vite+ is already managing Node.js (bin/node exists)
  if [ -e "$bin_path/node" ]; then
    # Already managing Node.js, just refresh shims
    "$bin_dir/vp" env setup --refresh > /dev/null
    NODE_MANAGER_ENABLED="already"
    return 0
  fi

  # Auto-enable on CI environment
  if [ -n "$CI" ]; then
    "$bin_dir/vp" env setup --refresh > /dev/null
    NODE_MANAGER_ENABLED="true"
    return 0
  fi

  # Check if node is available on the system
  local node_available="false"
  if command -v node &> /dev/null; then
    node_available="true"
  fi

  # Auto-enable if no node available on system
  if [ "$node_available" = "false" ]; then
    "$bin_dir/vp" env setup --refresh > /dev/null
    NODE_MANAGER_ENABLED="true"
    return 0
  fi

  # Prompt user in interactive mode
  if [ -e /dev/tty ] && [ -t 1 ]; then
    echo ""
    echo "Would you want Vite+ to manage Node.js versions?"
    echo -n "Press Enter to accept (Y/n): "
    read -r response < /dev/tty

    if [ -z "$response" ] || [ "$response" = "y" ] || [ "$response" = "Y" ]; then
      "$bin_dir/vp" env setup --refresh > /dev/null
      NODE_MANAGER_ENABLED="true"
    fi
  fi
}

# Cleanup old versions, keeping only the most recent ones
cleanup_old_versions() {
  local max_versions=5
  local versions=()

  # List version directories (only semver format like 0.1.0, 1.2.3-beta.1)
  # This excludes 'current' symlink and non-semver directories like 'local-dev'
  local semver_regex='^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.-]+)?$'
  for dir in "$INSTALL_DIR"/*/; do
    local name
    name=$(basename "$dir")
    if [ -d "$dir" ] && [[ "$name" =~ $semver_regex ]]; then
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

main() {
  echo ""
  echo -e "Setting up ${BRIGHT_BLUE}VITE+(⚡︎)${NC}..."
  echo ""

  check_requirements

  local platform
  platform=$(detect_platform)

  # Local development mode: use local tgz
  if [ -n "$LOCAL_TGZ" ]; then
    # Validate local tgz
    if [ ! -f "$LOCAL_TGZ" ]; then
      error "Local tarball not found: $LOCAL_TGZ"
    fi
    # Use version as-is (default to "local-dev")
    if [ "$VITE_PLUS_VERSION" = "latest" ] || [ "$VITE_PLUS_VERSION" = "test" ]; then
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
  # Also copy JS bundle and assets
  local items_to_copy=("dist" "templates" "rules" "AGENTS.md" "package.json")

  if [ -n "$LOCAL_TGZ" ]; then
    # Use local tarball for development/testing
    info "Using local tarball: $LOCAL_TGZ"

    # Extract everything from tgz
    local temp_dir
    temp_dir=$(mktemp -d)
    tar xzf "$LOCAL_TGZ" -C "$temp_dir" --strip-components=1

    # Copy binary
    cp "$temp_dir/bin/$binary_name" "$BIN_DIR/"
    chmod +x "$BIN_DIR/$binary_name"

    # Copy .node files if present
    for node_file in "$temp_dir"/dist/*.node; do
      if [ -f "$node_file" ]; then
        rm -f "$DIST_DIR/$(basename "$node_file")"
        cp "$node_file" "$DIST_DIR/"
      fi
    done

    # Copy JS assets
    for item in "${items_to_copy[@]}"; do
      if [ -e "$temp_dir/$item" ]; then
        cp -r "$temp_dir/$item" "$VERSION_DIR/"
      fi
    done

    rm -rf "$temp_dir"
  else
    # Download from npm registry
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

    # Download and extract JS bundle and assets from npm
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

  # Create/update current symlink (use relative path for portability)
  ln -sfn "$VITE_PLUS_VERSION" "$CURRENT_LINK"

  # Create bin directory and vp symlink (always done)
  mkdir -p "$INSTALL_DIR/bin"
  ln -sf "../current/bin/vp" "$INSTALL_DIR/bin/vp"

  # Cleanup old versions
  cleanup_old_versions

  # Configure shell PATH (always attempted)
  configure_shell_path

  # Setup Node.js version manager (shims) - separate component
  setup_node_manager "$BIN_DIR"

  # Use ~ shorthand if install dir is under HOME, otherwise show full path
  local display_dir="${INSTALL_DIR/#$HOME/~}"
  local display_location="${display_dir}/bin"

  # Print success message
  echo ""
  echo -e "${GREEN}✔${NC} ${BOLD_BRIGHT_BLUE}VITE+(⚡︎)${NC} successfully installed!"
  echo ""
  echo "  The Unified Toolchain for the Web."
  echo ""
  echo -e "  ${BOLD}Get started:${NC}"
  echo -e "    ${BRIGHT_BLUE}vp new${NC}          Create a new project"
  echo -e "    ${BRIGHT_BLUE}vp env${NC}          Manage Node.js versions"
  echo -e "    ${BRIGHT_BLUE}vp install${NC}      Install dependencies"
  echo -e "    ${BRIGHT_BLUE}vp dev${NC}          Start dev server"

  if [ "$NODE_MANAGER_ENABLED" = "true" ] || [ "$NODE_MANAGER_ENABLED" = "already" ]; then
    echo ""
    echo -e "  Node.js is now managed by Vite+ (via ${BRIGHT_BLUE}vp env${NC})."
    echo -e "  Run ${BRIGHT_BLUE}vp env doctor${NC} to verify your setup."
  fi

  echo ""
  echo -e "  Run ${BRIGHT_BLUE}vp help${NC} for more information."

  # Show restart note if PATH was added to shell config
  if [ "$PATH_CONFIGURED" = "true" ] && [ -n "$SHELL_CONFIG_UPDATED" ]; then
    echo ""
    echo "  Note: Run \`source ~/$SHELL_CONFIG_UPDATED\` or restart your terminal."
  fi

  # Show warning if PATH could not be automatically configured
  if [ "$PATH_CONFIGURED" = "false" ]; then
    echo ""
    echo -e "  ${YELLOW}note${NC}: Could not automatically add vp to your PATH."
    echo ""
    echo "  To use vp, add this line to your shell config file:"
    echo ""
    echo "    export PATH=\"$INSTALL_DIR/bin:\$PATH\""
    echo ""
    echo "  Common config files:"
    echo "    - Bash: ~/.bashrc or ~/.bash_profile"
    echo "    - Zsh:  ~/.zshrc"
    echo "    - Fish: ~/.config/fish/config.fish"
  fi

  echo ""
}

main "$@"
