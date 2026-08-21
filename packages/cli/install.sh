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

set -e

VP_VERSION="${VP_VERSION:-latest}"
# After these helper definitions, the selected payload resolves category roots
# through VP_DUMP_DIRS. Pre-split payloads use the legacy layout.
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

trace() {
  [ "${VP_LOG:-}" = "trace" ] || return 0
  echo -e "${DIM}trace${NC}: $1"
}

report_shell_config_error() {
  if [ "${CI:-}" = "true" ]; then
    trace "$1"
  else
    warn "$1"
  fi
}

error() {
  echo -e "${RED}error${NC}: $1"
  exit 1
}

is_release_age_error() {
  local log_file="$1"
  [ -f "$log_file" ] || return 1

  # This wrapper install path is pinned to pnpm via packageManager, so this
  # detection follows pnpm's resolver/reporter output rather than npm/yarn.
  #
  # pnpm's PnpmError prefixes internal codes with ERR_PNPM_, so
  # NO_MATURE_MATCHING_VERSION is normally printed as
  # ERR_PNPM_NO_MATURE_MATCHING_VERSION. npm-resolver emits that code with the
  # "does not meet the minimumReleaseAge constraint" message when
  # publishedBy/minimumReleaseAge rejects a matching version.
  # https://github.com/pnpm/pnpm/blob/16cfde66ec71125d692ea828eba2a5f9b3cc54fc/core/error/src/index.ts#L18-L20
  # https://github.com/pnpm/pnpm/blob/16cfde66ec71125d692ea828eba2a5f9b3cc54fc/resolving/npm-resolver/src/index.ts#L76-L84
  #
  # default-reporter may append guidance mentioning minimumReleaseAgeExclude
  # when the error has an immatureVersion, so that token is also a useful
  # release-age signal. minimum-release-age is pnpm's .npmrc key; npm's
  # min-release-age is intentionally not treated as a pnpm signal here.
  # https://github.com/pnpm/pnpm/blob/16cfde66ec71125d692ea828eba2a5f9b3cc54fc/cli/default-reporter/src/reportError.ts#L163-L164
  # https://github.com/pnpm/pnpm/blob/16cfde66ec71125d692ea828eba2a5f9b3cc54fc/config/reader/src/types.ts#L73-L74
  grep -Eqi 'ERR_PNPM_NO_MATURE_MATCHING_VERSION|NO_MATURE_MATCHING_VERSION|does not meet the minimumReleaseAge constraint|minimumReleaseAge|minimumReleaseAgeExclude|minimum release age|minimum-release-age' "$log_file" && return 0

  # pnpm can also surface ERR_PNPM_NO_MATCHING_VERSION when minimumReleaseAge
  # filters out all candidates. That code is also used for real missing
  # versions, so require age-gate context before prompting for a bypass.
  # https://github.com/pnpm/pnpm/blob/16cfde66ec71125d692ea828eba2a5f9b3cc54fc/deps/inspection/outdated/src/createManifestGetter.ts#L66-L76
  if grep -Eq 'ERR_PNPM_NO_MATCHING_VERSION' "$log_file"; then
    grep -Eqi 'minimumReleaseAge|minimumReleaseAgeExclude|minimum release age|minimum-release-age' "$log_file"
    return $?
  fi

  return 1
}

confirm_release_age_override() {
  [ -e /dev/tty ] && [ -t 1 ] || return 1

  echo "" > /dev/tty
  echo -e "${YELLOW}warn${NC}: Your minimumReleaseAge setting prevented installing vite-plus@${VP_VERSION}." > /dev/tty
  echo "This setting helps protect against newly published compromised packages." > /dev/tty
  echo "Proceeding will disable this protection for this Vite+ install only." > /dev/tty
  printf "Do you want to proceed? (y/N): " > /dev/tty

  local response
  read -r response < /dev/tty || return 1
  case "$response" in
    y|Y|yes|YES) return 0 ;;
    *) return 1 ;;
  esac
}

write_release_age_override() {
  # Append idempotently so a bridge registry line written for PR builds survives.
  if [ ! -f "$VERSION_DIR/.npmrc" ] || ! grep -q '^minimum-release-age=' "$VERSION_DIR/.npmrc" 2>/dev/null; then
    printf 'minimum-release-age=0\n' >> "$VERSION_DIR/.npmrc"
  fi
}

is_absolute_path() {
  case "$1" in
    /*) return 0 ;;
    [A-Za-z]:[\\/]*) return 0 ;;
    *) return 1 ;;
  esac
}

# Print $1 when it is a non-empty absolute path; otherwise print nothing.
absolute_override() {
  local val="$1"
  if [ -n "$val" ] && is_absolute_path "$val"; then
    printf '%s\n' "$val"
  fi
}

validate_vp_dir_overrides() {
  local count=0 value
  for value in "${VP_BIN_DIR:-}" "${VP_DATA_DIR:-}" "${VP_CACHE_DIR:-}"; do
    [ -z "$value" ] || count=$((count + 1))
  done
  if [ "$count" -ne 0 ] && [ "$count" -ne 3 ]; then
    error "Set VP_BIN_DIR, VP_DATA_DIR, and VP_CACHE_DIR together, or leave all three unset."
  fi
  if [ "$count" -eq 3 ]; then
    is_absolute_path "$VP_BIN_DIR" || error "VP_BIN_DIR must be an absolute path."
    is_absolute_path "$VP_DATA_DIR" || error "VP_DATA_DIR must be an absolute path."
    is_absolute_path "$VP_CACHE_DIR" || error "VP_CACHE_DIR must be an absolute path."
  fi
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

# Escape a path fragment for a Bash/Zsh double-quoted string. `$HOME` is
# added separately when the config directory is under the user home.
escape_posix_double_quoted() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\$/\\\$}"
  value="${value//\`/\\\`}"
  value="${value//\"/\\\"}"
  printf '%s' "$value"
}

# Fish double-quoted strings do not evaluate backticks, but `$`, `"`, and
# backslashes still need escaping.
escape_fish_double_quoted() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\$/\\\$}"
  value="${value//\"/\\\"}"
  printf '%s' "$value"
}

# Nushell expands values only in interpolated strings (`$"..."`). In a plain
# double-quoted string only backslashes and double quotes need escaping.
escape_nu_double_quoted() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  printf '%s' "$value"
}

set_config_dir_refs() {
  local dir="$1"
  local shell_home="$2"
  local suffix
  if [ -n "$shell_home" ] && case "$dir" in "$shell_home"/*) true;; *) false;; esac; then
    suffix="${dir#"$shell_home"}"
    CONFIG_DIR_REF_POSIX="\$HOME$(escape_posix_double_quoted "$suffix")"
    CONFIG_DIR_REF_FISH="\$HOME$(escape_fish_double_quoted "$suffix")"
    CONFIG_DIR_REF_NU="~$(escape_nu_double_quoted "$suffix")"
  else
    CONFIG_DIR_REF_POSIX="$(escape_posix_double_quoted "$dir")"
    CONFIG_DIR_REF_FISH="$(escape_fish_double_quoted "$dir")"
    CONFIG_DIR_REF_NU="$(escape_nu_double_quoted "$dir")"
  fi
}

# Monolithic mapping: every category on one root.
set_monolithic_layout() {
  LAYOUT_KIND="single-root"
  INSTALL_DIR="$1"
  SHIM_DIR="$1/bin"
  CACHE_DIR="$1/cache"
  CONFIG_DIR="$1"
  STATE_DIR="$1"
}

# Pre-split releases resolve all paths from VP_HOME, which defaults to
# ~/.vite-plus. Install them in this monolithic root. This keeps environment
# setup, shims, upgrades, and installer paths consistent.
use_legacy_layout() {
  local resolution_home vp_home
  resolution_home="$(resolution_home_dir)"
  [ -n "$resolution_home" ] || error "Vite+ could not resolve the user home directory."
  vp_home="$(absolute_override "${VP_HOME:-}")"
  set_monolithic_layout "${vp_home:-$resolution_home/.vite-plus}"
  set_config_dir_refs "$CONFIG_DIR" "${HOME:-}"
}

normalize_existing_dir() {
  local dir="${1%/}"
  if [ -z "$dir" ]; then
    dir="/"
  fi

  if [ -d "$dir" ]; then
    (cd "$dir" 2>/dev/null && pwd -P) || printf '%s\n' "$dir"
  else
    local base parent_dir
    base="$(basename "$dir")"
    parent_dir="$(cd "$(dirname "$dir")" 2>/dev/null && pwd -P)" || parent_dir=""
    if [ -z "$parent_dir" ]; then
      printf '%s\n' "$dir"
    elif [ "$parent_dir" = "/" ]; then
      printf '/%s\n' "$base"
    else
      printf '%s/%s\n' "$parent_dir" "$base"
    fi
  fi
}

is_safe_install_dir_to_remove() {
  local dir="$1"
  [ -n "$dir" ] || return 1

  case "$dir" in
    "/" | "$HOME" | "/bin" | "/opt" | "/usr" | "/usr/bin" | "/usr/local" | "/usr/local/bin")
      return 1
      ;;
  esac

  return 0
}

is_vite_plus_install_dir() {
  local dir="$1"
  [ -d "$dir" ] || return 1
  [ -d "$dir/bin" ] || return 1
  [ -e "$dir/current" ] || return 1
  [ -e "$dir/bin/vp" ] || [ -e "$dir/bin/vp.exe" ] || [ -e "$dir/bin/vp.cmd" ]
}

detect_previous_install_dir() {
  [ -n "${VP_HOME:-}" ] || return 1

  local vp_path
  vp_path="$(command -v vp 2>/dev/null || true)"
  [ -n "$vp_path" ] || return 1

  case "$(basename "$vp_path")" in
    vp | vp.exe | vp.cmd) ;;
    *) return 1 ;;
  esac

  local old_dir install_dir
  old_dir="$(normalize_existing_dir "$(dirname "$(dirname "$vp_path")")")"
  install_dir="$(normalize_existing_dir "$INSTALL_DIR")"
  [ "$old_dir" != "$install_dir" ] || return 1

  is_safe_install_dir_to_remove "$old_dir" || return 1
  is_vite_plus_install_dir "$old_dir" || return 1

  printf '%s\n' "$old_dir"
}

is_nested_install_dir() {
  [ -n "$1" ] && [ -n "$2" ] || return 1

  local old_dir install_dir
  old_dir="$(normalize_existing_dir "$1")"
  install_dir="$(normalize_existing_dir "$2")"

  [ "$old_dir" != "$install_dir" ] || return 1
  if [ "$old_dir" = "/" ] || [ "$install_dir" = "/" ]; then
    return 0
  fi

  case "$old_dir" in
    "$install_dir"/*) return 0 ;;
  esac
  case "$install_dir" in
    "$old_dir"/*) return 0 ;;
  esac

  return 1
}

prompt_remove_previous_install_dir() {
  local old_dir="$1"
  [ -n "$old_dir" ] || return 0
  [ -z "${CI:-}" ] || return 0
  [ -e /dev/tty ] && [ -t 1 ] || return 0

  echo "" > /dev/tty
  echo -e "${YELLOW}warn${NC}: Found a previous Vite+ install at $old_dir." > /dev/tty
  echo "The new VP_HOME is $INSTALL_DIR." > /dev/tty
  printf "Remove the previous install directory? (y/N): " > /dev/tty

  local response
  read -r response < /dev/tty || return 0
  case "$response" in
    y | Y | yes | YES)
      local vp_bin="$old_dir/current/bin/vp"
      if [ ! -f "$vp_bin" ]; then
        vp_bin="$old_dir/current/bin/vp.exe"
      fi
      if [ ! -f "$vp_bin" ]; then
        warn "Could not remove previous Vite+ install at $old_dir: vp binary not found."
        return 0
      fi

      local implode_output
      if implode_output=$(VP_HOME="$old_dir" "$vp_bin" implode --yes 2>&1); then
        success "Removed previous Vite+ install at $old_dir."
      else
        warn "Could not remove previous Vite+ install at $old_dir."
        if [ -n "$implode_output" ]; then
          printf '%s\n' "$implode_output" >&2
        fi
      fi
      ;;
  esac
}

# Resolve a PR number or commit SHA to the registry bridge's immutable commit
# version (0.0.0-commit.<sha>). A full commit SHA maps directly to the bridge's
# deterministic version; a PR number (or short ref) is resolved via the bridge
# download URL's `x-commit-key: <owner>:<repo>:<sha>` header (HEAD).
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

print_install_failure() {
  local install_log="$1"
  if [ "${CI:-}" = "true" ]; then
    echo -e "${RED}error${NC}: Failed to install dependencies. Log output:"
    cat "$install_log"
  else
    echo -e "${RED}error${NC}: Failed to install dependencies. See log for details: $install_log"
  fi
}

print_release_age_failure() {
  local install_log="$1"
  if [ "${CI:-}" = "true" ]; then
    echo -e "${RED}error${NC}: Install blocked by your minimumReleaseAge setting. Log output:"
    cat "$install_log"
  else
    echo -e "${RED}error${NC}: Install blocked by your minimumReleaseAge setting. Wait until the package is old enough or adjust your package manager configuration explicitly. See log for details: $install_log"
  fi
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
# Uses VP_VERSION to fetch the correct version's metadata
PACKAGE_METADATA=""
fetch_package_metadata() {
  if [ -z "$PACKAGE_METADATA" ]; then
    local version_path metadata_url
    if [ "$VP_VERSION" = "latest" ]; then
      version_path="latest"
    else
      version_path="$VP_VERSION"
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

# Get the version from package metadata
# Sets RESOLVED_VERSION global variable
get_version_from_metadata() {
  # Call fetch_package_metadata to populate PACKAGE_METADATA global
  # Don't use command substitution as it would swallow the exit from error()
  fetch_package_metadata
  RESOLVED_VERSION=$(echo "$PACKAGE_METADATA" | grep -o '"version" *: *"[^"]*"' | head -1 | cut -d'"' -f4)
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

join_by() {
  local separator="$1"
  shift
  local result=""
  local item

  for item in "$@"; do
    if [ -z "$result" ]; then
      result="$item"
    else
      result="${result}${separator}${item}"
    fi
  done

  printf '%s\n' "$result"
}

abbreviate_path() {
  local path="$1"
  if [ "${path#"$HOME"}" != "$path" ]; then
    printf '~%s\n' "${path#"$HOME"}"
  else
    printf '%s\n' "$path"
  fi
}

record_shell_summary() {
  local shell_name="$1"
  local status="$2"
  SHELL_CONFIG_SUMMARY+=("    - ${shell_name}: ${status}")
}

# Add a sourcing line to an existing shell config file.
# Returns: 0 = line added, 1 = file missing, 2 = already configured, 3 = failed
append_source_to_file() {
  local shell_config="$1"
  local source_line="$2"
  shift 2
  local search_patterns=("$@")
  local pattern

  if [ ! -f "$shell_config" ]; then
    return 1
  fi

  if [ ! -w "$shell_config" ]; then
    report_shell_config_error "Cannot write to $shell_config (permission denied), skipping."
    return 3
  fi

  for pattern in "${search_patterns[@]}"; do
    if grep -Fq "$pattern" "$shell_config" 2>/dev/null; then
      return 2
    fi
  done

  {
    printf '\n'
    printf '%s\n' "# Vite+ bin (https://viteplus.dev)"
    printf '%s\n' "$source_line"
  } >> "$shell_config"
  return 0
}

# Create or update an installer-managed snippet file.
# Returns: 0 = written, 2 = already configured, 3 = failed
write_managed_snippet() {
  local snippet_file="$1"
  local snippet_content="$2"
  local snippet_dir

  snippet_dir=$(dirname "$snippet_file")
  if ! mkdir -p "$snippet_dir" 2>/dev/null; then
    report_shell_config_error "Cannot create $snippet_dir, skipping."
    return 3
  fi

  if [ -f "$snippet_file" ] && [ ! -w "$snippet_file" ]; then
    report_shell_config_error "Cannot write to $snippet_file (permission denied), skipping."
    return 3
  fi

  if [ -f "$snippet_file" ] && printf '%s' "$snippet_content" | cmp -s - "$snippet_file"; then
    return 2
  fi

  if ! printf '%s' "$snippet_content" > "$snippet_file"; then
    report_shell_config_error "Cannot write to $snippet_file, skipping."
    return 3
  fi
  return 0
}

# Discover Nushell's preferred user-local vendor autoload directory.
# Nushell puts the user-local directory at the end of the list.
discover_nushell_vendor_autoload_dir() {
  command -v nu > /dev/null 2>&1 || return 1

  local nu_dirs_output
  nu_dirs_output=$(nu -c '$nu.vendor-autoload-dirs | reverse | each {|dir| $dir } | str join (char nl)' 2>/dev/null) || return 1

  while IFS= read -r dir; do
    [ -n "$dir" ] || continue
    printf '%s\n' "$dir"
    return 0
  done <<EOF
$nu_dirs_output
EOF

  return 1
}

configure_zsh_path() {
  local zsh_dir="${ZDOTDIR:-$HOME}"
  local zshenv="$zsh_dir/.zshenv"
  local zshrc="$zsh_dir/.zshrc"
  local updated=()
  local already=()
  local failed=()
  local result

  if ! mkdir -p "$zsh_dir" 2>/dev/null; then
    report_shell_config_error "Cannot create $zsh_dir, skipping zsh."
    SHELL_CONFIG_HAS_FAILURE="true"
    SHELL_CONFIG_FAILED_SHELLS+=("zsh")
    record_shell_summary "zsh" "failed (could not create $(abbreviate_path "$zsh_dir"))"
    return
  fi

  if [ ! -f "$zshenv" ] && ! touch "$zshenv" 2>/dev/null; then
    report_shell_config_error "Cannot create $zshenv, skipping zsh."
    SHELL_CONFIG_HAS_FAILURE="true"
    SHELL_CONFIG_FAILED_SHELLS+=("zsh")
    record_shell_summary "zsh" "failed (could not create $(abbreviate_path "$zshenv"))"
    return
  fi

  result=0
  append_source_to_file "$zshenv" ". \"$CONFIG_DIR_REF_POSIX/env\"" "$CONFIG_DIR/env" "$CONFIG_DIR_REF_POSIX/env" || result=$?
  case "$result" in
    0) updated+=("$(abbreviate_path "$zshenv")") ;;
    2) already+=("$(abbreviate_path "$zshenv")") ;;
    3) failed+=("$(abbreviate_path "$zshenv")") ;;
  esac

  if [ -f "$zshrc" ]; then
    result=0
    append_source_to_file "$zshrc" ". \"$CONFIG_DIR_REF_POSIX/env\"" "$CONFIG_DIR/env" "$CONFIG_DIR_REF_POSIX/env" || result=$?
    case "$result" in
      0) updated+=("$(abbreviate_path "$zshrc")") ;;
      2) already+=("$(abbreviate_path "$zshrc")") ;;
      3) failed+=("$(abbreviate_path "$zshrc")") ;;
    esac
  fi

  local details=()
  if [ ${#updated[@]} -gt 0 ]; then
    SHELL_CONFIG_HAS_UPDATED="true"
    SHELL_CONFIG_HAS_CONFIGURED="true"
    details+=("updated $(join_by ', ' "${updated[@]}")")
  fi
  if [ ${#already[@]} -gt 0 ]; then
    SHELL_CONFIG_HAS_CONFIGURED="true"
    details+=("already configured $(join_by ', ' "${already[@]}")")
  fi
  if [ ${#failed[@]} -gt 0 ]; then
    SHELL_CONFIG_HAS_FAILURE="true"
    SHELL_CONFIG_FAILED_SHELLS+=("zsh")
    details+=("failed $(join_by ', ' "${failed[@]}")")
  fi

  if [ ${#details[@]} -eq 0 ]; then
    record_shell_summary "zsh" "skipped"
  else
    record_shell_summary "zsh" "$(join_by '; ' "${details[@]}")"
  fi
}

configure_bash_path() {
  local updated=()
  local already=()
  local failed=()
  local existing=0
  local file result

  for file in "$HOME/.bash_profile" "$HOME/.bashrc" "$HOME/.profile"; do
    if [ ! -f "$file" ]; then
      continue
    fi
    existing=1
    result=0
    append_source_to_file "$file" ". \"$CONFIG_DIR_REF_POSIX/env\"" "$CONFIG_DIR/env" "$CONFIG_DIR_REF_POSIX/env" || result=$?
    case "$result" in
      0) updated+=("$(abbreviate_path "$file")") ;;
      2) already+=("$(abbreviate_path "$file")") ;;
      3) failed+=("$(abbreviate_path "$file")") ;;
    esac
  done

  if [ "$existing" -eq 0 ]; then
    record_shell_summary "bash" "skipped (no existing rc files)"
    return
  fi

  local details=()
  if [ ${#updated[@]} -gt 0 ]; then
    SHELL_CONFIG_HAS_UPDATED="true"
    SHELL_CONFIG_HAS_CONFIGURED="true"
    details+=("updated $(join_by ', ' "${updated[@]}")")
  fi
  if [ ${#already[@]} -gt 0 ]; then
    SHELL_CONFIG_HAS_CONFIGURED="true"
    details+=("already configured $(join_by ', ' "${already[@]}")")
  fi
  if [ ${#failed[@]} -gt 0 ]; then
    SHELL_CONFIG_HAS_FAILURE="true"
    SHELL_CONFIG_FAILED_SHELLS+=("bash")
    details+=("failed $(join_by ', ' "${failed[@]}")")
  fi

  record_shell_summary "bash" "$(join_by '; ' "${details[@]}")"
}

configure_fish_path() {
  local fish_config="${XDG_CONFIG_HOME:-$HOME/.config}/fish/conf.d/vite-plus.fish"
  local fish_content="# Vite+ bin (https://viteplus.dev)
source \"$CONFIG_DIR_REF_FISH/env.fish\"
"

  local result=0
  write_managed_snippet "$fish_config" "$fish_content" || result=$?
  case "$result" in
    0)
      SHELL_CONFIG_HAS_UPDATED="true"
      SHELL_CONFIG_HAS_CONFIGURED="true"
      record_shell_summary "fish" "updated $(abbreviate_path "$fish_config")"
      ;;
    2)
      SHELL_CONFIG_HAS_CONFIGURED="true"
      record_shell_summary "fish" "already configured $(abbreviate_path "$fish_config")"
      ;;
    *)
      SHELL_CONFIG_HAS_FAILURE="true"
      SHELL_CONFIG_FAILED_SHELLS+=("fish")
      record_shell_summary "fish" "failed $(abbreviate_path "$fish_config")"
      ;;
  esac
}

configure_nushell_path() {
  local nushell_dir
  nushell_dir=$(discover_nushell_vendor_autoload_dir 2>/dev/null) || true
  if [ -z "$nushell_dir" ]; then
    SHELL_CONFIG_HAS_FAILURE="true"
    SHELL_CONFIG_FAILED_SHELLS+=("nushell")
    record_shell_summary "nushell" "failed (could not determine vendor autoload dir)"
    return
  fi

  local nushell_autoload="$nushell_dir/vite-plus.nu"
  local nushell_content="# Vite+ bin (https://viteplus.dev)
source \"$CONFIG_DIR_REF_NU/env.nu\"
"

  local result=0
  write_managed_snippet "$nushell_autoload" "$nushell_content" || result=$?
  case "$result" in
    0)
      SHELL_CONFIG_HAS_UPDATED="true"
      SHELL_CONFIG_HAS_CONFIGURED="true"
      record_shell_summary "nushell" "updated $(abbreviate_path "$nushell_autoload")"
      ;;
    2)
      SHELL_CONFIG_HAS_CONFIGURED="true"
      record_shell_summary "nushell" "already configured $(abbreviate_path "$nushell_autoload")"
      ;;
    *)
      SHELL_CONFIG_HAS_FAILURE="true"
      SHELL_CONFIG_FAILED_SHELLS+=("nushell")
      record_shell_summary "nushell" "failed $(abbreviate_path "$nushell_autoload")"
      ;;
  esac
}

# Configure supported shell PATH integrations for all installed shells.
configure_shell_path() {
  SHELL_CONFIG_SUMMARY=()
  SHELL_CONFIG_FAILED_SHELLS=()
  SHELL_CONFIG_HAS_UPDATED="false"
  SHELL_CONFIG_HAS_CONFIGURED="false"
  SHELL_CONFIG_HAS_FAILURE="false"

  if command -v zsh > /dev/null 2>&1; then
    configure_zsh_path
  else
    record_shell_summary "zsh" "skipped (not installed)"
  fi

  if command -v bash > /dev/null 2>&1; then
    configure_bash_path
  else
    record_shell_summary "bash" "skipped (not installed)"
  fi

  if command -v fish > /dev/null 2>&1; then
    configure_fish_path
  else
    record_shell_summary "fish" "skipped (not installed)"
  fi

  if command -v nu > /dev/null 2>&1; then
    configure_nushell_path
  else
    record_shell_summary "nushell" "skipped (not installed)"
  fi
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

# Return success only if this Vite+ install owns the existing Node entry. A bin
# from an explicit override group can be shared. Entry existence does not permit
# replacement.
is_vite_plus_node_shim() {
  local bin_path="$1"
  local vp_bin="$2"

  # Unix shims are symlinks to the active vp binary. `-ef` follows the link. It
  # accepts the old relative target and the absolute split-layout target.
  if [ -L "$bin_path/node" ] && [ "$bin_path/node" -ef "$vp_bin" ]; then
    return 0
  fi

  # install.sh can also run under Git Bash/MSYS. Windows trampolines carry a
  # per-executable sidecar that records the owning data root.
  if [ -f "$bin_path/node.exe" ] && [ -f "$bin_path/node.shim" ]; then
    local pointer=""
    pointer="$(shim_pointer_data "$bin_path/node.shim")" || return 1
    [ "$pointer" = "$INSTALL_DIR" ] && return 0
  fi

  return 1
}

shim_pointer_data() {
  local file="$1" first="" line=""
  IFS= read -r first < "$file" || [ -n "$first" ] || return 1
  first="${first%$'\r'}"
  if [ "$first" != "vite-plus-shim-v1" ]; then
    return 1
  fi
  while IFS= read -r line || [ -n "$line" ]; do
    line="${line%$'\r'}"
    case "$line" in
      data=*) printf '%s\n' "${line#data=}"; return 0 ;;
    esac
  done < "$file"
  return 1
}

# Setup Node.js version manager (node/npm/npx/corepack shims)
# Sets NODE_MANAGER_ENABLED global
# Arguments: bin_dir - path to the version's bin directory containing vp
setup_node_manager() {
  local bin_dir="$1"
  local bin_path="$SHIM_DIR"
  NODE_MANAGER_ENABLED="false"

  # Resolve vp binary name (vp on Unix, vp.exe on Windows)
  local vp_bin="$bin_dir/vp"
  if [ -f "$bin_dir/vp.exe" ]; then
    vp_bin="$bin_dir/vp.exe"
  fi

  # Explicit override via environment variable
  if [ "$VP_NODE_MANAGER" = "yes" ]; then
    refresh_shims "$vp_bin"
    NODE_MANAGER_ENABLED="true"
    return 0
  elif [ "$VP_NODE_MANAGER" = "no" ]; then
    NODE_MANAGER_ENABLED="false"
    return 0
  fi

  # Check if an existing Node entry is a Vite+ shim. A foreign entry in a custom
  # bin directory prevents automatic enablement. The prompt below can get
  # permission to replace the entry.
  local unmanaged_node_in_bin="false"
  if [ -e "$bin_path/node" ] || [ -L "$bin_path/node" ] || [ -e "$bin_path/node.exe" ]; then
    if is_vite_plus_node_shim "$bin_path" "$vp_bin"; then
      refresh_shims "$vp_bin"
      NODE_MANAGER_ENABLED="already"
      return 0
    fi
    unmanaged_node_in_bin="true"
  fi

  # Auto-enable on CI or devcontainer environments
  # CI: standard CI environment variable (GitHub Actions, Travis, CircleCI, etc.)
  # CODESPACES: set by GitHub Codespaces (https://docs.github.com/en/codespaces)
  # REMOTE_CONTAINERS: set by VS Code Dev Containers extension
  # DEVPOD: set by DevPod (https://devpod.sh)
  if [ "$unmanaged_node_in_bin" = "false" ] && { [ -n "$CI" ] || [ -n "$CODESPACES" ] || [ -n "$REMOTE_CONTAINERS" ] || [ -n "$DEVPOD" ]; }; then
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
  if [ "$node_available" = "false" ] && [ "$unmanaged_node_in_bin" = "false" ]; then
    refresh_shims "$vp_bin"
    NODE_MANAGER_ENABLED="true"
    return 0
  fi

  # Prompt user in interactive mode
  if [ -e /dev/tty ] && [ -t 1 ]; then
    echo ""
    echo "Would you like Vite+ to manage your Node.js versions?"
    echo "Vite+ adds \`node\`, \`npm\`, \`npx\`, and \`corepack\` shims to $(abbreviate_path "$SHIM_DIR")."
    echo "It selects the required version automatically."
    echo "Opt out anytime with \`vp env off\`."
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
  local max_versions=3
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

  if [ -n "$PR_VERSION" ] && [ -n "$LOCAL_TGZ" ]; then
    error "VP_PR_VERSION and VP_LOCAL_TGZ cannot be used together"
  fi

  validate_vp_dir_overrides
  enable_setup_vp_legacy_compatibility
  check_requirements

  local previous_install_dir=""

  local platform
  platform=$(detect_platform)

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
    if ! apply_dirs_from_vp "$LOCAL_BINARY"; then
      use_legacy_layout
      info "The local vite-plus binary does not support the split directory layout. Vite+ will install it in $(abbreviate_path "$INSTALL_DIR")."
    fi
  elif [ -n "$PR_VERSION" ]; then
    # Registry bridge mode: resolve the requested PR/SHA to the bridge's
    # immutable commit version (0.0.0-commit.<sha>), the clearly-defined test
    # version we install. The directory label stays non-semver so it keeps out
    # of cleanup_old_versions and makes the PR build obvious in `~/.vite-plus/`.
    # `|| true` keeps `set -e` from aborting this assignment when resolution
    # fails (unregistered ref / transient bridge error), so the actionable
    # error below is reachable instead of the installer exiting silently.
    PR_COMMIT_VERSION="$(resolve_bridge_commit_version "$PR_VERSION" || true)"
    if [ -z "$PR_COMMIT_VERSION" ]; then
      error "Could not resolve a registry bridge build for ${PR_VERSION}"
    fi
    VP_VERSION="pkg-pr-new-${PR_VERSION}"
    info "Using registry bridge build: ${PR_COMMIT_VERSION}"
  else
    # Fetch package metadata and resolve version from npm
    get_version_from_metadata
    VP_VERSION="$RESOLVED_VERSION"
  fi

  local binary_name="vp"
  if [[ "$platform" == win32* ]]; then
    binary_name="vp.exe"
  fi

  # Download the CLI platform tarball before Vite+ selects the final layout.
  # The downloaded binary reports the layout that it supports.
  local platform_temp_dir=""
  if [ -z "$LOCAL_TGZ" ]; then
    # npm registry or registry bridge (when PR_VERSION is set)
    get_platform_suffix "$platform"
    local platform_url
    if [ -n "$PR_VERSION" ]; then
      # The registry bridge redirects this URL to the platform tarball for the
      # matching commit build (0.0.0-commit.<sha>).
      platform_url="${BRIDGE_DOWNLOAD_BASE}/@voidzero-dev/vite-plus-cli-${PLATFORM_SUFFIX}@${PR_VERSION}"
    else
      local package_name="@voidzero-dev/vite-plus-cli-${PLATFORM_SUFFIX}"
      platform_url="${NPM_REGISTRY}/${package_name}/-/vite-plus-cli-${PLATFORM_SUFFIX}-${VP_VERSION}.tgz"
    fi

    # Create temp directory for extraction
    platform_temp_dir=$(mktemp -d)
    download_and_extract "$platform_url" "$platform_temp_dir" 1
    chmod +x "$platform_temp_dir/$binary_name"

    # Ask the downloaded binary for its layout through VP_DUMP_DIRS. A pre-split
    # release cannot report a layout. Give that release the monolithic root so
    # the installed PATH commands work.
    if ! apply_dirs_from_vp "$platform_temp_dir/$binary_name"; then
      use_legacy_layout
      info "vite-plus ${VP_VERSION} does not support the split directory layout. Vite+ will install it in $(abbreviate_path "$INSTALL_DIR")."
    fi
  fi

  # Run layout migration checks after the payload resolves the category roots.
  # A pre-split payload selects the legacy layout first.
  previous_install_dir="$(detect_previous_install_dir || true)"
  if [ -n "$previous_install_dir" ] && is_nested_install_dir "$previous_install_dir" "$INSTALL_DIR"; then
    error "The previous Vite+ install at $previous_install_dir overlaps with VP_HOME $INSTALL_DIR. Set VP_HOME to a directory that does not overlap. Alternatively, remove the previous install."
  fi

  # Set up version-specific directories
  VERSION_DIR="$INSTALL_DIR/$VP_VERSION"
  BIN_DIR="$VERSION_DIR/bin"
  CURRENT_LINK="$INSTALL_DIR/current"

  # Create bin directory
  mkdir -p "$BIN_DIR"

  if [ -n "$LOCAL_TGZ" ]; then
    # Local development mode: only need the binary
    info "Vite+ uses the local tarball: $LOCAL_TGZ"

    # Copy binary from LOCAL_BINARY env var (set by install-global-cli.ts)
    cp "$LOCAL_BINARY" "$BIN_DIR/$binary_name"
    # On Windows, also copy the trampoline shim binary if available
    if [[ "$platform" == win32* ]]; then
      local shim_src
      shim_src="$(dirname "$LOCAL_BINARY")/vp-shim.exe"
      if [ -f "$shim_src" ]; then
        cp "$shim_src" "$BIN_DIR/vp-shim.exe"
      fi
    fi
    chmod +x "$BIN_DIR/$binary_name"
  else
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
  # pnpm will install vite-plus and all transitive deps via `vp install`.
  # The packageManager field pins pnpm to a known-good version, ensuring
  # consistent behavior regardless of the user's global pnpm version.
  # In PR mode, pin vite-plus to the bridge's clearly-defined commit version and
  # resolve it (plus its platform binaries and transitive deps) through the
  # bridge registry written to .npmrc below. The bridge rewrites a preview
  # tarball's transitive deps to versions, not self-contained URLs, so a full
  # install must go through the registry rather than the bare download URL.
  local vite_plus_spec="$VP_VERSION"
  if [ -n "$PR_VERSION" ]; then
    vite_plus_spec="$PR_COMMIT_VERSION"
    # Resolve the commit version + platform binaries through the bridge. Drop any
    # stale wrapper lockfile: the pkg-pr-new-<ref> dir is reused across a PR's
    # commits and install.sh rewrites this package.json each run, so a leftover
    # lockfile pinning a prior spec would fail `vp install` with
    # ERR_PNPM_OUTDATED_LOCKFILE under CI's frozen-lockfile default. Removing it
    # lets the install regenerate a lockfile matching the spec we just wrote.
    printf 'registry=%s\n' "$BRIDGE_REGISTRY" > "$VERSION_DIR/.npmrc"
    rm -f "$VERSION_DIR/pnpm-lock.yaml"
  fi
  cat > "$VERSION_DIR/package.json" <<WRAPPER_EOF
{
  "name": "vp-global",
  "version": "$VP_VERSION",
  "private": true,
  "packageManager": "pnpm@10.33.0",
  "dependencies": {
    "vite-plus": "$vite_plus_spec"
  }
}
WRAPPER_EOF

  # Install production dependencies (skip if VP_SKIP_DEPS_INSTALL is set,
  # e.g. during local dev where install-global-cli.ts handles deps separately)
  if [ -z "${VP_SKIP_DEPS_INSTALL:-}" ]; then
    local install_log="$VERSION_DIR/install.log"
    local vp_install_bin="$BIN_DIR/vp"
    if [ -f "$BIN_DIR/vp.exe" ]; then
      vp_install_bin="$BIN_DIR/vp.exe"
    fi
    # Do not pass --silent to the inner install: pnpm suppresses the
    # release-age error body in silent mode, which would leave install.log
    # empty and make the release-age gate impossible to detect. Output is
    # already redirected to install.log here.
    if ! (cd "$VERSION_DIR" && CI=true "$vp_install_bin" install > "$install_log" 2>&1); then
      if is_release_age_error "$install_log"; then
        if confirm_release_age_override; then
          # Write the override only after explicit consent, then retry once.
          write_release_age_override
          if ! (cd "$VERSION_DIR" && CI=true "$vp_install_bin" install > "$install_log" 2>&1); then
            print_install_failure "$install_log"
            exit 1
          fi
        else
          print_release_age_failure "$install_log"
          exit 1
        fi
      else
        print_install_failure "$install_log"
        exit 1
      fi
    fi
  fi

  # Create/update current symlink (use relative path for portability)
  ln -sfn "$VP_VERSION" "$CURRENT_LINK"

  # Create user bin directory and vp entrypoint (always done)
  mkdir -p "$SHIM_DIR"
  if [[ "$platform" == win32* ]]; then
    # Windows: copy trampoline as vp.exe (matching install.ps1)
    if [ -f "$INSTALL_DIR/current/bin/vp-shim.exe" ]; then
      cp "$INSTALL_DIR/current/bin/vp-shim.exe" "$SHIM_DIR/vp.exe"
      # For a complete split override group, the trampoline reads <name>.shim
      # instead of inherited environment variables.
      printf 'vite-plus-shim-v1\nlayout=%s\ndata=%s\ncache=%s\n' \
        "$LAYOUT_KIND" "$INSTALL_DIR" "$CACHE_DIR" >"$SHIM_DIR/vp.shim"
    fi
  else
    ln -sfn "$INSTALL_DIR/current/bin/vp" "$SHIM_DIR/vp"
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

  # Setup Node.js version manager (shims) - separate component
  setup_node_manager "$BIN_DIR"

  prompt_remove_previous_install_dir "$previous_install_dir"

  # Configure shell PATH after the install is otherwise complete.
  configure_shell_path

  # Use ~ when an install location is under HOME. Otherwise, show the full path.
  local display_data_dir display_bin_dir
  display_data_dir="$(abbreviate_path "$INSTALL_DIR")"
  display_bin_dir="$(abbreviate_path "$SHIM_DIR")"

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

  echo ""
  echo -e "  ${BOLD}Install locations:${NC}"
  echo "    Data directory: $display_data_dir"
  echo "    Bin directory:  $display_bin_dir"

  # CI jobs configure PATH through the runner.
  # Shell files do not change PATH for later steps.
  # Do not print shell details in normal CI output.
  if [ "${CI:-}" = "true" ]; then
    echo ""
    return
  fi

  echo ""
  echo "  Shell configuration:"
  local summary_line
  for summary_line in "${SHELL_CONFIG_SUMMARY[@]}"; do
    echo "$summary_line"
  done

  # Show restart note if any shell config was updated
  if [ "$SHELL_CONFIG_HAS_UPDATED" = "true" ]; then
    echo ""
    echo "  Note: Restart your terminal to load updated shell configuration."
  fi

  # Show manual PATH instructions if no shell was configured or any shell failed
  if [ "$SHELL_CONFIG_HAS_CONFIGURED" = "false" ] || [ "$SHELL_CONFIG_HAS_FAILURE" = "true" ]; then
    echo ""
    echo -e "  ${YELLOW}note${NC}: Some shells still need manual setup."
    echo ""
    echo "  Manual setup instructions:"
    echo "    - Bash/Zsh: add the following to your shell config (~/.bashrc, ~/.zshrc, etc.):"
    printf '        . "%s/env"\n' "$CONFIG_DIR_REF_POSIX"
    echo "    - Fish: create ${XDG_CONFIG_HOME:-$HOME/.config}/fish/conf.d/vite-plus.fish with:"
    printf '        source "%s/env.fish"\n' "$CONFIG_DIR_REF_FISH"
    echo "    - Nushell: create a vendor autoload file with:"
    printf '        source "%s/env.nu"\n' "$CONFIG_DIR_REF_NU"
    echo ""
    echo "  Or run vp directly:"
    echo ""
    echo -e "    ${display_bin_dir}/vp"
  fi

  echo ""
}

apply_dirs_from_vp() {
  local vp="$1"
  local out
  out="$(VP_DUMP_DIRS=1 "$vp" 2>/dev/null)" || return 1
  INSTALL_DIR="$(printf '%s\n' "$out" | awk -F '\t' '$1 == "data" { print $2; exit }')"
  SHIM_DIR="$(printf '%s\n' "$out" | awk -F '\t' '$1 == "bin" { print $2; exit }')"
  CACHE_DIR="$(printf '%s\n' "$out" | awk -F '\t' '$1 == "cache" { print $2; exit }')"
  CONFIG_DIR="$(printf '%s\n' "$out" | awk -F '\t' '$1 == "config" { print $2; exit }')"
  STATE_DIR="$(printf '%s\n' "$out" | awk -F '\t' '$1 == "state" { print $2; exit }')"
  LAYOUT_KIND="$(printf '%s\n' "$out" | awk -F '\t' '$1 == "layout" { print $2; exit }')"
  [ -n "$INSTALL_DIR" ] && [ -n "$SHIM_DIR" ] && [ -n "$CACHE_DIR" ] && [ -n "$CONFIG_DIR" ] && [ -n "$STATE_DIR" ] || return 1
  if [ "$LAYOUT_KIND" != "single-root" ] && [ "$LAYOUT_KIND" != "split" ]; then
    if [ "$SHIM_DIR" = "$INSTALL_DIR/bin" ] && [ "$CACHE_DIR" = "$INSTALL_DIR/cache" ] \
      && [ "$CONFIG_DIR" = "$INSTALL_DIR" ] && [ "$STATE_DIR" = "$INSTALL_DIR" ]; then
      LAYOUT_KIND="single-root"
    else
      LAYOUT_KIND="split"
    fi
  fi
  set_config_dir_refs "$CONFIG_DIR" "${HOME:-}"
}

main "$@"
