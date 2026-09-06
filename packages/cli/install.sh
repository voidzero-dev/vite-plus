#!/bin/bash
# Vite+ CLI Installer
# https://vite.plus
#
# Usage:
#   curl -fsSL https://vite.plus | bash
#
# Environment variables:
#   VP_VERSION - Version to install (default: latest)
#   VP_HOME - Optional pin for the monolithic layout. If unset, Vite+ reuses an
#             existing ~/.vite-plus install. Otherwise, the complete VP_*_DIR
#             group, XDG_*, or platform defaults select the split roots.
#   VP_BIN_DIR / VP_DATA_DIR / VP_CACHE_DIR - Complete group of absolute
#                                             category overrides
#   XDG_DATA_HOME / XDG_CONFIG_HOME / … - Unix split defaults
#   NPM_CONFIG_REGISTRY - Custom npm registry URL (default: https://registry.npmjs.org)
#   VP_NODE_MANAGER - Set to "yes" or "no" to skip interactive prompt (for CI/devcontainers)
#   VP_LOCAL_TGZ - Path to local vite-plus.tgz (for development/testing)
#   VP_PR_VERSION - PR number or commit SHA to install from the registry bridge
#                   (for temporary testing of unreleased builds, e.g. VP_PR_VERSION=1569).
#                   When set, overrides VP_VERSION and installs the clearly-defined
#                   0.0.0-commit.<sha> build through the bridge instead of npm.

# When sourced, returns INSTALL_DIR, SHIM_DIR, CACHE_DIR, CONFIG_DIR, and STATE_DIR.
# These are resolved paths, not VP_* overrides for subsequent commands.
set -e

VP_VERSION="${VP_VERSION:-latest}"
# npm registry URL (strip trailing slash if present)
NPM_REGISTRY="${NPM_CONFIG_REGISTRY:-https://registry.npmjs.org}"
NPM_REGISTRY="${NPM_REGISTRY%/}"
# Local tarball for development/testing
LOCAL_TGZ="${VP_LOCAL_TGZ:-}"
# Local binary path (set by install-global-cli.ts for local dev)
LOCAL_BINARY="${VP_LOCAL_BINARY:-}"
# PR number or commit SHA to install as a test build (registry bridge mode)
PR_VERSION="${VP_PR_VERSION:-}"
# Registry bridge that serves PR preview builds as clearly-versioned packages.
# The pkg.pr.new-style download URL (BRIDGE_DOWNLOAD_BASE) 302-redirects to a
# canonical 0.0.0-commit.<sha> tarball; the registry (BRIDGE_REGISTRY) resolves
# those commit versions (and proxies everything else to npmjs) so a full install
# pulls a coherent, clearly-defined test build.
BRIDGE_DOWNLOAD_BASE="https://registry-bridge.viteplus.dev/voidzero-dev/vite-plus"
BRIDGE_REGISTRY="https://registry-bridge.viteplus.dev/"

RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'
PACKAGE_METADATA=""
# Legacy is published beside this bootstrap; preview builds rewrite this origin.
LEGACY_INSTALLER_URL="${VP_LEGACY_INSTALLER_URL:-https://viteplus.dev/install-legacy.sh}"
INSTALLER_PATH="${BASH_SOURCE[0]:-}"

info() {
  echo -e "${BLUE}info${NC}: $1" >&2
}

error() {
  echo -e "${RED}error${NC}: $1" >&2
  exit 1
}

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

fetch_package_metadata() {
  if [ -z "$PACKAGE_METADATA" ]; then
    local version_path metadata_url
    version_path="$VP_VERSION"
    metadata_url="${NPM_REGISTRY}/vite-plus/${version_path}"
    PACKAGE_METADATA=$(curl_with_error_handling -s "$metadata_url")
    if [ -z "$PACKAGE_METADATA" ]; then
      error "Failed to fetch package metadata from: $metadata_url"
    fi
    # Check for npm registry error response
    # npm can return either {"error":"..."} or a plain JSON string like "version not found: test"
    if echo "$PACKAGE_METADATA" | grep -q '"error"'; then
      local error_msg
      error_msg=$(echo "$PACKAGE_METADATA" | grep -o '"error" *: *"[^"]*"' | cut -d'"' -f4)
      error "Failed to fetch version '${version_path}': ${error_msg:-unknown error}\n  URL: $metadata_url"
    fi
    # Check if response is a plain error string (not a valid package object)
    # Use '"version":' to match JSON property, not just the word "version"
    if ! echo "$PACKAGE_METADATA" | grep -q '"version" *:'; then
      # Remove surrounding quotes from the error message if present
      local error_msg
      error_msg=$(echo "$PACKAGE_METADATA" | sed 's/^"//;s/"$//')
      error "Failed to fetch version '${version_path}': ${error_msg:-unknown error}\n  URL: $metadata_url"
    fi
  fi
  # PACKAGE_METADATA is set as a global variable, no need to echo
}

get_version_from_metadata() {
  # Call fetch_package_metadata to populate PACKAGE_METADATA global
  # Don't use command substitution as it would swallow the exit from error()
  fetch_package_metadata
  RESOLVED_VERSION=$(echo "$PACKAGE_METADATA" | grep -o '"version" *: *"[^"]*"' | head -1 | cut -d'"' -f4)
  if [ -z "$RESOLVED_VERSION" ]; then
    error "Failed to extract version from package metadata"
  fi
}

get_platform_suffix() {
  local platform="$1"
  case "$platform" in
    win32-*) PLATFORM_SUFFIX="${platform}-msvc" ;;  # Windows needs -msvc suffix
    *) PLATFORM_SUFFIX="$platform" ;;               # macOS/Linux map directly
  esac
}

download_and_extract() (
  local url="$1"
  local dest_dir="$2"

  # Download to temp file (silent mode)
  local temp_file
  temp_file=$(mktemp)
  trap 'rm -f "$temp_file"' EXIT

  # Run curl and capture exit code for error handling
  set +e
  curl -sL "$url" -o "$temp_file"
  local exit_code=$?
  set -e

  if [ $exit_code -ne 0 ]; then
    rm -f "$temp_file"
    print_curl_error "$exit_code" "$url"
  fi

  tar xzf "$temp_file" -C "$dest_dir" --strip-components=1

)

resolve_bridge_commit_version() {
  local ref="$1"
  local sha="$ref"
  if [[ ! "$ref" =~ ^[0-9a-fA-F]{40}$ ]]; then
    sha="$(curl -fsSIL "${BRIDGE_DOWNLOAD_BASE}@${ref}" 2>/dev/null | tr -d '\r' | awk -F ': ' '
      tolower($1) == "x-commit-key" { count = split($2, parts, ":"); print parts[count]; exit }')"
  fi
  case "$sha" in
    '' | *[!0-9a-fA-F]*) return 1 ;;
  esac
  [ "${#sha}" -eq 40 ] || return 1
  printf '0.0.0-commit.%s' "$sha"
}

is_windows_uname() {
  case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) return 0 ;;
    *) return 1 ;;
  esac
}

resolution_home_dir() {
  if is_windows_uname; then
    printf '%s\n' "${USERPROFILE:-$HOME}"
  else
    printf '%s\n' "${HOME:-$USERPROFILE}"
  fi
}

# Released setup-vp versions add ~/.vite-plus/bin to the GitHub Actions or
# GitLab CI/CD PATH. They do this after the installer exits. Use the monolithic
# layout until setup-vp declares support for VP_DUMP_DIRS.
enable_setup_vp_legacy_compatibility() {
  if [ "${GITHUB_ACTION_REPOSITORY:-}" != "voidzero-dev/setup-vp" ]; then
    [ "${GITLAB_CI:-}" = "true" ] || return 0
    [ -n "${SETUP_VP_SETUP_REF:-}" ] || return 0
  fi
  [ "${VP_VPDIRS_AWARE:-}" != "1" ] || return 0
  [ -z "${VP_HOME:-}" ] || return 0
  [ -z "${VP_BIN_DIR:-}" ] || return 0
  [ -z "${VP_DATA_DIR:-}" ] || return 0
  [ -z "${VP_CACHE_DIR:-}" ] || return 0

  local resolution_home
  resolution_home="$(resolution_home_dir)"
  [ -n "$resolution_home" ] || error "Vite+ could not resolve the user home directory."
  VP_HOME="$resolution_home/.vite-plus"
  export VP_HOME
}

main() {
  enable_setup_vp_legacy_compatibility

  if [ -n "$PR_VERSION" ] && [ -n "$LOCAL_TGZ" ]; then
    error "VP_PR_VERSION and VP_LOCAL_TGZ cannot be used together"
  fi

  check_requirements

  # Local development mode: use local tgz
  if [ -n "$LOCAL_TGZ" ]; then
    # Validate local tgz
    if [ ! -f "$LOCAL_TGZ" ]; then
      error "Local tarball not found: $LOCAL_TGZ"
    fi
    # Use version as-is (default to "local-dev")
    if [ "$VP_VERSION" = "latest" ] || [ "$VP_VERSION" = "test" ]; then
      VP_VERSION="local-dev"
    fi
    if [ -z "$LOCAL_BINARY" ] || [ ! -f "$LOCAL_BINARY" ]; then
      error "Set VP_LOCAL_BINARY when you use VP_LOCAL_TGZ."
    fi
  elif [ -n "$PR_VERSION" ]; then
    # Registry bridge mode: resolve the requested PR/SHA to the bridge's
    # immutable commit version (0.0.0-commit.<sha>), the clearly-defined test
    # version we install. Legacy receives the full SHA as its preview ref.
    # `|| true` keeps `set -e` from aborting this assignment when resolution
    # fails (unregistered ref / transient bridge error), so the actionable
    # error below is reachable instead of the installer exiting silently.
    PR_COMMIT_VERSION="$(resolve_bridge_commit_version "$PR_VERSION" || true)"
    if [ -z "$PR_COMMIT_VERSION" ]; then
      error "Could not resolve a registry bridge build for ${PR_VERSION}"
    fi
    VP_VERSION="$PR_COMMIT_VERSION"
    info "Using registry bridge build: ${PR_COMMIT_VERSION}"
  else
    # Fetch package metadata and resolve version from npm
    get_version_from_metadata
    VP_VERSION="$RESOLVED_VERSION"
  fi

  local platform
  platform=$(detect_platform)
  local result
  result="$(set -e; acquire_and_handoff "$platform")" || return $?
  # setup-vp reads these assignments in the shell that sourced this installer.
  eval "$result"
}

acquire_and_handoff() (
  local platform="$1"
  local binary_name="vp"
  if [[ "$platform" == win32* ]]; then
    binary_name="vp.exe"
  fi

  # Keep acquisition separate from permanent installation. The bootstrap owns cleanup.
  local binary_source platform_temp_dir=""
  if [ -z "$LOCAL_TGZ" ]; then
    # npm registry or registry bridge (when PR_VERSION is set)
    get_platform_suffix "$platform"
    local platform_url
    if [ -n "$PR_VERSION" ]; then
      # The registry bridge redirects this URL to the platform tarball for the
      # matching commit build (0.0.0-commit.<sha>).
      platform_url="${BRIDGE_DOWNLOAD_BASE}/@voidzero-dev/vite-plus-cli-${PLATFORM_SUFFIX}@${PR_COMMIT_VERSION#0.0.0-commit.}"
    else
      local package_name="@voidzero-dev/vite-plus-cli-${PLATFORM_SUFFIX}"
      platform_url="${NPM_REGISTRY}/${package_name}/-/vite-plus-cli-${PLATFORM_SUFFIX}-${VP_VERSION}.tgz"
    fi

    # Create temp directory for extraction
    platform_temp_dir=$(mktemp -d)
    platform_temp_dir=$(cd "$platform_temp_dir" && pwd -P)
    trap "rm -rf -- $(printf '%q' "$platform_temp_dir")" EXIT
    download_and_extract "$platform_url" "$platform_temp_dir" || exit $?
    binary_source="$platform_temp_dir/$binary_name"
    [ -f "$binary_source" ] || error "Downloaded package does not contain $binary_name"
    chmod +x "$binary_source"
  else
    binary_source="$(cd "$(dirname "$LOCAL_BINARY")" && pwd -P)/$(basename "$LOCAL_BINARY")"
  fi

  if supports_self_setup "$binary_source"; then
    handoff_install "$binary_source"
  else
    run_legacy_installer "$binary_source"
  fi
)

supports_self_setup() {
  local response
  # Old binaries must exit with help rather than opening an interactive picker.
  response=$(VP_SELF_SETUP_SUPPORT_CHECK=1 "$1" --help 2>/dev/null && printf '.') || return 1
  [ "$response" = $'vite-plus-self-setup-v1\n.' ]
}

run_legacy_installer() (
  local binary_source="$1"
  local legacy_script=""
  if [ -n "$INSTALLER_PATH" ] && [ -f "$INSTALLER_PATH" ]; then
    legacy_script="$(dirname "$INSTALLER_PATH")/install-legacy.sh"
  fi
  if [ -z "$legacy_script" ] || [ ! -f "$legacy_script" ]; then
    legacy_script=$(mktemp)
    trap "rm -f -- $(printf '%q' "$legacy_script")" EXIT
    curl_with_error_handling -fsSL "$LEGACY_INSTALLER_URL" -o "$legacy_script" >&2
  fi
  # Preserve the child status explicitly, including when a caller disables errexit.
  local status=0
  source "$legacy_script" "$binary_source" "$VP_VERSION" "$PR_VERSION" >&2 || status=$?
  [ "$status" -eq 0 ] || exit "$status"
  local name
  for name in INSTALL_DIR SHIM_DIR CACHE_DIR CONFIG_DIR STATE_DIR; do
    printf '%s=%q\n' "$name" "${!name}"
  done
)

handoff_install() (
  unset VP_SELF_SETUP_SUPPORT_CHECK
  # Preview dependencies must use the same registry as the downloaded binary.
  if [ -n "$PR_VERSION" ]; then
    export NPM_CONFIG_REGISTRY="$BRIDGE_REGISTRY"
  fi
  # curl | bash leaves stdin on the script pipe; setup consent must read from the terminal.
  if [ -t 2 ] && [ -z "${CI+x}" ]; then
    exec < /dev/tty
  fi
  local status=0
  VP_SELF_SETUP_SHELL=sh "$1" || status=$?
  exit "$status"
)

main "$@"
