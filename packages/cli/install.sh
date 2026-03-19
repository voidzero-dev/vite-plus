#!/bin/bash
# Vite+ CLI Installer
# https://vite.plus
#
# Usage:
#   curl -fsSL https://vite.plus | bash
#
# Environment variables:
#   VITE_PLUS_VERSION - Version to install (default: latest)
#   VITE_PLUS_HOME - Installation directory (default: ~/.vite-plus)
#   NPM_CONFIG_REGISTRY - Custom npm registry URL (default: https://registry.npmjs.org)
#   VITE_PLUS_LOCAL_TGZ - Path to local vite-plus.tgz (for development/testing)

set -e

VITE_PLUS_VERSION="${VITE_PLUS_VERSION:-latest}"
INSTALL_DIR="${VITE_PLUS_HOME:-$HOME/.vite-plus}"
# Use $HOME-relative path for shell config references (portable across sessions)
if case "$INSTALL_DIR" in "$HOME"/*) true;; *) false;; esac; then
  INSTALL_DIR_REF="\$HOME${INSTALL_DIR#"$HOME"}"
else
  INSTALL_DIR_REF="$INSTALL_DIR"
fi
# npm registry URL (strip trailing slash if present)
NPM_REGISTRY="${NPM_CONFIG_REGISTRY:-https://registry.npmjs.org}"
NPM_REGISTRY="${NPM_REGISTRY%/}"
# Local tarball for development/testing
LOCAL_TGZ="${VITE_PLUS_LOCAL_TGZ:-}"
# Local binary path (set by install-global-cli.ts for local dev)
LOCAL_BINARY="${VITE_PLUS_LOCAL_BINARY:-}"

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
  # Prefer positive glibc detection first.
  # This avoids false musl detection on systems where musl is installed
  # but the distro itself is glibc-based (common on WSL/Ubuntu).
  if command -v getconf &> /dev/null; then
    if getconf GNU_LIBC_VERSION > /dev/null 2>&1; then
      echo "gnu"
      return
    fi
  fi

  # Check ldd output for musl/glibc
  if command -v ldd &> /dev/null; then
    ldd_out="$(ldd --version 2>&1 || true)"
    if echo "$ldd_out" | grep -qi musl; then
      echo "musl"
      return
    fi
    if echo "$ldd_out" | grep -qi 'gnu libc'; then
      echo "gnu"
      return
    fi
    if echo "$ldd_out" | grep -qi 'glibc'; then
      echo "gnu"
      return
    fi
  fi

  # Final fallback: musl loader present usually indicates musl-based distro,
  # but only check this after glibc detection to avoid false positives.
  if [ -e /lib/ld-musl-x86_64.so.1 ] || [ -e /lib/ld-musl-aarch64.so.1 ]; then
    echo "musl"
  else
    echo "gnu"
  fi
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
    metadata_url="${NPM_REGISTRY}/vite-plus/${version_path}"
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

# Get platform suffix for CLI package download
# Sets PLATFORM_SUFFIX global variable
# Platform format from detect_platform(): darwin-arm64, darwin-x64, linux-x64-gnu, linux-arm64-gnu, win32-x64, etc.
# CLI package format: @voidzero-dev/vite-plus-cli-darwin-arm64, @voidzero-dev/vite-plus-cli-linux-x64-gnu, etc.
get_platform_suffix() {
  local platform="$1"
  case "$platform" in
    win32-*) PLATFORM_SUFFIX="${platform}-msvc" ;;  # Windows needs -msvc suffix
    *) PLATFORM_SUFFIX="$platform" ;;               # macOS/Linux map directly
  esac
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

# Add bin to shell profile by sourcing the env file
# Returns: 0 = path added, 1 = file not found, 2 = path already exists
add_bin_to_path() {
  local shell_config="$1"
  local env_file="$INSTALL_DIR_REF/env"
  # Escape both absolute and $HOME-relative forms for grep (backward compat)
  local abs_pattern ref_pattern
  abs_pattern=$(printf '%s' "$INSTALL_DIR" | sed 's/[.[\*^$()+?{|]/\\&/g')
  ref_pattern=$(printf '%s' "$INSTALL_DIR_REF" | sed 's/[.[\*^$()+?{|]/\\&/g')

  if [ -f "$shell_config" ]; then
    if [ ! -w "$shell_config" ]; then
      warn "Cannot write to $shell_config (permission denied), skipping."
      return 1
    fi
    if grep -q "${abs_pattern}/env" "$shell_config" 2>/dev/null || \
       grep -q "${ref_pattern}/env" "$shell_config" 2>/dev/null; then
      return 2
    fi
    echo "" >> "$shell_config"
    echo "# Vite+ bin (https://viteplus.dev)" >> "$shell_config"
    echo ". \"$env_file\"" >> "$shell_config"
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

  local result=1  # Default to failure - must explicitly set success
  case "$SHELL" in
    */zsh)
      # Add to both .zshenv (for all shells including IDE) and .zshrc (to ensure PATH is at front)
      # Create .zshenv if missing — it's the canonical place for PATH in zsh
      # and is sourced by all session types (interactive, non-interactive, IDE)
      local zsh_dir="${ZDOTDIR:-$HOME}"
      mkdir -p "$zsh_dir"
      [ -f "$zsh_dir/.zshenv" ] || touch "$zsh_dir/.zshenv"
      local zshenv_result=0 zshrc_result=0
      add_bin_to_path "$zsh_dir/.zshenv" || zshenv_result=$?
      add_bin_to_path "$zsh_dir/.zshrc" || zshrc_result=$?
      # Prioritize .zshrc for user notification (easier to source)
      if [ $zshrc_result -eq 0 ]; then
        result=0
        SHELL_CONFIG_UPDATED="$zsh_dir/.zshrc"
      elif [ $zshenv_result -eq 0 ]; then
        result=0
        SHELL_CONFIG_UPDATED="$zsh_dir/.zshenv"
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
        SHELL_CONFIG_UPDATED="$HOME/.bashrc"
      elif [ $bash_profile_result -eq 0 ]; then
        result=0
        SHELL_CONFIG_UPDATED="$HOME/.bash_profile"
      elif [ $profile_result -eq 0 ]; then
        result=0
        SHELL_CONFIG_UPDATED="$HOME/.profile"
      elif [ $bash_profile_result -eq 2 ] || [ $bashrc_result -eq 2 ] || [ $profile_result -eq 2 ]; then
        result=2  # already configured in at least one file
      fi
      ;;
    */fish)
      local fish_dir="${XDG_CONFIG_HOME:-$HOME/.config}/fish/conf.d"
      local fish_config="$fish_dir/vite-plus.fish"
      if [ -f "$fish_config" ]; then
        result=2
      else
        mkdir -p "$fish_dir"
        echo "# Vite+ bin (https://viteplus.dev)" >> "$fish_config"
        echo "source \"$INSTALL_DIR_REF/env.fish\"" >> "$fish_config"
        result=0
        SHELL_CONFIG_UPDATED="$fish_config"
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

# Run vp env setup --refresh, showing output only on failure
# Arguments: vp_bin - path to the vp binary
refresh_shims() {
  local vp_bin="$1"
  local setup_output
  if ! setup_output=$("$vp_bin" env setup --refresh 2>&1); then
    warn "Failed to refresh shims:"
    echo "$setup_output" >&2
  fi
}

# Setup Node.js version manager (node/npm/npx shims)
# Sets NODE_MANAGER_ENABLED global
# Arguments: bin_dir - path to the version's bin directory containing vp
setup_node_manager() {
  local bin_dir="$1"
  local bin_path="$INSTALL_DIR/bin"
  NODE_MANAGER_ENABLED="false"

  # Resolve vp binary name (vp on Unix, vp.exe on Windows)
  local vp_bin="$bin_dir/vp"
  if [ -f "$bin_dir/vp.exe" ]; then
    vp_bin="$bin_dir/vp.exe"
  fi

  # Check if Vite+ is already managing Node.js (bin/node or bin/node.exe exists)
  if [ -e "$bin_path/node" ] || [ -e "$bin_path/node.exe" ]; then
    refresh_shims "$vp_bin"
    NODE_MANAGER_ENABLED="already"
    return 0
  fi

  # Auto-enable on CI environment
  if [ -n "$CI" ]; then
    refresh_shims "$vp_bin"
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
    refresh_shims "$vp_bin"
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
      refresh_shims "$vp_bin"
      NODE_MANAGER_ENABLED="true"
    fi
  fi
}

# Cleanup old versions, keeping only the most recent ones
cleanup_old_versions() {
  local max_versions=5
  local versions=()

  # List version directories (semver format like 0.1.0, 1.2.3-beta.1, 0.0.0-f48af939.20260205-0533)
  # This excludes 'current' symlink and non-semver directories like 'local-dev'
  local semver_regex='^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9._-]+)?$'
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
  echo -e "Setting up VITE+..."

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
  CURRENT_LINK="$INSTALL_DIR/current"

  local binary_name="vp"
  if [[ "$platform" == win32* ]]; then
    binary_name="vp.exe"
  fi

  # Create bin directory
  mkdir -p "$BIN_DIR"

  if [ -n "$LOCAL_TGZ" ]; then
    # Local development mode: only need the binary
    info "Using local tarball: $LOCAL_TGZ"

    # Copy binary from LOCAL_BINARY env var (set by install-global-cli.ts)
    if [ -n "$LOCAL_BINARY" ]; then
      cp "$LOCAL_BINARY" "$BIN_DIR/$binary_name"
      # On Windows, also copy the trampoline shim binary if available
      if [[ "$platform" == win32* ]]; then
        local shim_src
        shim_src="$(dirname "$LOCAL_BINARY")/vp-shim.exe"
        if [ -f "$shim_src" ]; then
          cp "$shim_src" "$BIN_DIR/vp-shim.exe"
        fi
      fi
    else
      error "VITE_PLUS_LOCAL_BINARY must be set when using VITE_PLUS_LOCAL_TGZ"
    fi
    chmod +x "$BIN_DIR/$binary_name"
  else
    # Download from npm registry — extract only the vp binary from CLI platform package
    get_platform_suffix "$platform"
    local package_name="@voidzero-dev/vite-plus-cli-${PLATFORM_SUFFIX}"
    local platform_url="${NPM_REGISTRY}/${package_name}/-/vite-plus-cli-${PLATFORM_SUFFIX}-${VITE_PLUS_VERSION}.tgz"

    # Create temp directory for extraction
    local platform_temp_dir
    platform_temp_dir=$(mktemp -d)
    download_and_extract "$platform_url" "$platform_temp_dir" 1

    # Copy binary to BIN_DIR
    cp "$platform_temp_dir/$binary_name" "$BIN_DIR/"
    chmod +x "$BIN_DIR/$binary_name"
    # On Windows, also copy the trampoline shim binary if present in the package
    if [[ "$platform" == win32* ]] && [ -f "$platform_temp_dir/vp-shim.exe" ]; then
      cp "$platform_temp_dir/vp-shim.exe" "$BIN_DIR/"
    fi
    rm -rf "$platform_temp_dir"
  fi

  # Generate wrapper package.json that declares vite-plus as a dependency.
  # npm will install vite-plus and all transitive deps via `vp install`.
  cat > "$VERSION_DIR/package.json" <<WRAPPER_EOF
{
  "name": "vp-global",
  "version": "$VITE_PLUS_VERSION",
  "private": true,
  "dependencies": {
    "vite-plus": "$VITE_PLUS_VERSION"
  }
}
WRAPPER_EOF

  # Isolate from user's global package manager config that may block
  # installing recently-published packages (e.g. pnpm's minimumReleaseAge,
  # npm's min-release-age) by creating a local .npmrc in the version directory.
  cat > "$VERSION_DIR/.npmrc" <<NPMRC_EOF
minimum-release-age=0
min-release-age=0
NPMRC_EOF

  # Install production dependencies (skip if VITE_PLUS_SKIP_DEPS_INSTALL is set,
  # e.g. during local dev where install-global-cli.ts handles deps separately)
  if [ -z "${VITE_PLUS_SKIP_DEPS_INSTALL:-}" ]; then
    local install_log="$VERSION_DIR/install.log"
    local vp_install_bin="$BIN_DIR/vp"
    if [ -f "$BIN_DIR/vp.exe" ]; then
      vp_install_bin="$BIN_DIR/vp.exe"
    fi
    if ! (cd "$VERSION_DIR" && CI=true "$vp_install_bin" install --silent > "$install_log" 2>&1); then
      error "Failed to install dependencies. See log for details: $install_log"
      exit 1
    fi
  fi

  # Create/update current symlink (use relative path for portability)
  ln -sfn "$VITE_PLUS_VERSION" "$CURRENT_LINK"

  # Create bin directory and vp entrypoint (always done)
  mkdir -p "$INSTALL_DIR/bin"
  if [[ "$platform" == win32* ]]; then
    # Windows: copy trampoline as vp.exe (matching install.ps1)
    if [ -f "$INSTALL_DIR/current/bin/vp-shim.exe" ]; then
      cp "$INSTALL_DIR/current/bin/vp-shim.exe" "$INSTALL_DIR/bin/vp.exe"
    fi
  else
    # Unix: symlink to current/bin/vp
    ln -sf "../current/bin/vp" "$INSTALL_DIR/bin/vp"
  fi

  # Cleanup old versions
  cleanup_old_versions

  # Create env files with PATH guard (prevents duplicate PATH entries)
  # Use current/bin/vp directly (the real binary) instead of bin/vp (trampoline)
  # to avoid the self-overwrite issue on Windows during --refresh
  local vp_bin="$INSTALL_DIR/current/bin/vp"
  if [[ "$platform" == win32* ]]; then
    vp_bin="$INSTALL_DIR/current/bin/vp.exe"
  fi
  "$vp_bin" env setup --env-only > /dev/null

  # Configure shell PATH (always attempted)
  configure_shell_path

  # Setup Node.js version manager (shims) - separate component
  setup_node_manager "$BIN_DIR"

  # Use ~ shorthand if install dir is under HOME, otherwise show full path
  local display_dir="${INSTALL_DIR/#$HOME/~}"
  local display_location="${display_dir}/bin"

  # Print success message
  echo ""
  echo -e "${GREEN}✔${NC} ${BOLD_BRIGHT_BLUE}VITE+${NC} successfully installed!"
  echo ""
  echo "  The Unified Toolchain for the Web."
  echo ""
  echo -e "  ${BOLD}Get started:${NC}"
  echo -e "    ${BRIGHT_BLUE}vp create${NC}       Create a new project"
  echo -e "    ${BRIGHT_BLUE}vp env${NC}          Manage Node.js versions"
  echo -e "    ${BRIGHT_BLUE}vp install${NC}      Install dependencies"
  echo -e "    ${BRIGHT_BLUE}vp migrate${NC}      Migrate to Vite+"

  if [ "$NODE_MANAGER_ENABLED" = "true" ] || [ "$NODE_MANAGER_ENABLED" = "already" ]; then
    echo ""
    echo -e "  Vite+ is now managing Node.js via ${BRIGHT_BLUE}vp env${NC}."
    echo -e "  Run ${BRIGHT_BLUE}vp env doctor${NC} to verify your setup, or ${BRIGHT_BLUE}vp env off${NC} to opt out."
  fi

  echo ""
  echo -e "  Run ${BRIGHT_BLUE}vp help${NC} to see available commands."

  # Show restart note if PATH was added to shell config
  if [ "$PATH_CONFIGURED" = "true" ] && [ -n "$SHELL_CONFIG_UPDATED" ]; then
    local display_config
    if [ "${SHELL_CONFIG_UPDATED#"$HOME"}" != "$SHELL_CONFIG_UPDATED" ]; then
      display_config="~${SHELL_CONFIG_UPDATED#"$HOME"}"
    else
      display_config="$SHELL_CONFIG_UPDATED"
    fi
    echo ""
    if [ "${display_config#"~"}" != "$display_config" ]; then
      echo "  Note: Run \`source $display_config\` or restart your terminal."
    else
      echo "  Note: Run \`source \"$display_config\"\` or restart your terminal."
    fi
  fi

  # Show warning if PATH could not be automatically configured
  if [ "$PATH_CONFIGURED" = "false" ]; then
    echo ""
    echo -e "  ${YELLOW}note${NC}: Could not automatically add vp to your PATH."
    echo ""
    echo "  To use vp, add this line to your shell config file:"
    echo ""
    echo "    . \"$INSTALL_DIR_REF/env\""
    echo ""
    echo "  Common config files:"
    echo "    - Bash: ~/.bashrc or ~/.bash_profile"
    echo "    - Zsh:  ~/.zshrc"
    echo "    - Fish: source \"$INSTALL_DIR_REF/env.fish\" in ~/.config/fish/config.fish"
  fi

  echo ""
}

main "$@"
