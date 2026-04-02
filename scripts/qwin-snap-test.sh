#!/usr/bin/env bash
# Cross-compile Windows binaries on macOS and run snap tests inside a qwin QEMU Windows VM.
#
# Prerequisites:
#   brew install qemu cdrtools llvm
#   cargo install cargo-xwin
#   rustup target add x86_64-pc-windows-msvc
#   cd tools/qwin && cp .env.example .env  # set WIN_ISO_URL
#   cd tools/qwin && ./build.sh --host     # one-time Windows VM build (~60+ min on macOS)
#
# First-time VM setup (after build.sh completes):
#   ./scripts/qwin-snap-test.sh --setup    # installs Node.js, Git, pnpm, VC++ Runtime
#
# Usage:
#   ./scripts/qwin-snap-test.sh [options] [filter]
#
# Options:
#   --shard=N/TOTAL   Run only shard N of TOTAL (e.g. --shard=1/3)
#   --reset           Reset the VM overlay before starting
#   --skip-build      Skip cross-compilation (use existing binaries)
#   --setup           Run first-time VM setup (install Node.js, Git, pnpm, etc.)
#   --local           Run only local snap tests (snap-tests/)
#   --global          Run only global snap tests (snap-tests-global/)
#   -h, --help        Show this help message

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
QWIN_DIR="$PROJECT_ROOT/tools/qwin"
TARGET="x86_64-pc-windows-msvc"
SSH_PORT="${QWIN_SSH_PORT:-2222}"
SSH_OPTS="-p $SSH_PORT -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR"
SSH_USER="administrator"
SSH_HOST="localhost"
REMOTE_PROJECT="C:\\Users\\Administrator\\vite-plus"
BUNDLE_FILE="/tmp/vp-win-test-bundle.tar.gz"

NODE_VERSION="22.18.0"
GIT_VERSION="2.50.1"

# --- Parse arguments ---
SHARD=""
RESET=false
SKIP_BUILD=false
SETUP=false
RUN_LOCAL=true
RUN_GLOBAL=true
FILTER=""

for arg in "$@"; do
  case "$arg" in
    --shard=*)    SHARD="${arg#--shard=}" ;;
    --reset)      RESET=true ;;
    --skip-build) SKIP_BUILD=true ;;
    --setup)      SETUP=true ;;
    --local)      RUN_GLOBAL=false ;;
    --global)     RUN_LOCAL=false ;;
    -h|--help)
      sed -n '2,/^$/p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *)            FILTER="$arg" ;;
  esac
done

# --- Helpers ---
info()  { echo ":: $*"; }
error() { echo "!! $*" >&2; }
step()  { echo ""; echo "==> $*"; }

ssh_cmd() {
  # shellcheck disable=SC2086
  ssh $SSH_OPTS "$SSH_USER@$SSH_HOST" "$@"
}

scp_to_vm() {
  # shellcheck disable=SC2086
  scp $SSH_OPTS "$@" "$SSH_USER@$SSH_HOST:" 2>&1
}

check_prerequisites() {
  local missing=false

  if ! command -v qemu-system-x86_64 &>/dev/null; then
    error "qemu not found. Install with: brew install qemu"
    missing=true
  fi

  if ! command -v cargo-xwin &>/dev/null && [[ "$SKIP_BUILD" == false ]]; then
    error "cargo-xwin not found. Install with: cargo install cargo-xwin"
    missing=true
  fi

  if ! rustup target list --installed 2>/dev/null | grep -q "$TARGET" && [[ "$SKIP_BUILD" == false ]]; then
    error "Rust target $TARGET not installed. Run: rustup target add $TARGET"
    missing=true
  fi

  if [[ ! -d "$QWIN_DIR" ]]; then
    error "qwin not found at $QWIN_DIR. Run: git submodule update --init tools/qwin"
    missing=true
  fi

  if [[ "$missing" == true ]]; then
    exit 1
  fi
}

# --- First-time VM setup ---
setup_vm() {
  step "Setting up Windows VM with development tools..."

  # Enable long paths (required for node_modules)
  info "Enabling long paths..."
  ssh_cmd 'cmd /c "reg add HKLM\SYSTEM\CurrentControlSet\Control\FileSystem /v LongPathsEnabled /t REG_DWORD /d 1 /f"'

  # Install VC++ Redistributable (required for MSVC-compiled Rust binaries)
  info "Installing VC++ Redistributable..."
  curl -fSL -o /tmp/vc_redist.x64.exe "https://aka.ms/vs/17/release/vc_redist.x64.exe"
  scp_to_vm /tmp/vc_redist.x64.exe
  ssh_cmd 'cmd /c "C:\Users\Administrator\vc_redist.x64.exe /install /quiet /norestart && del C:\Users\Administrator\vc_redist.x64.exe && echo VCREDIST_DONE"'

  # Install Node.js
  info "Installing Node.js v${NODE_VERSION}..."
  curl -fSL -o /tmp/node-setup.msi "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-x64.msi"
  scp_to_vm /tmp/node-setup.msi
  ssh_cmd 'cmd /c "msiexec /i C:\Users\Administrator\node-setup.msi /qn /norestart && del C:\Users\Administrator\node-setup.msi && echo NODE_DONE"'

  # Install MinGit
  info "Installing Git v${GIT_VERSION}..."
  curl -fSL -o /tmp/MinGit.zip "https://github.com/git-for-windows/git/releases/download/v${GIT_VERSION}.windows.1/MinGit-${GIT_VERSION}-64-bit.zip"
  scp_to_vm /tmp/MinGit.zip
  ssh_cmd 'cmd /c "mkdir C:\Git 2>nul & tar xf C:\Users\Administrator\MinGit.zip -C C:\Git & del C:\Users\Administrator\MinGit.zip & echo GIT_DONE"'

  # Add to system PATH
  info "Updating system PATH..."
  ssh_cmd 'cmd /c "setx /M PATH \"C:\Git\cmd;%PATH%\""'

  # Install pnpm via corepack
  info "Installing pnpm..."
  ssh_cmd 'cmd /c "corepack enable && corepack prepare pnpm@latest --activate && pnpm --version"'

  # Install tsx for running TypeScript snap test runner
  info "Installing tsx..."
  ssh_cmd 'cmd /c "npm install -g tsx && echo TSX_DONE"'

  # Verify everything
  step "Verifying setup..."
  ssh_cmd 'cmd /c "node --version && git --version && pnpm --version && tsx --version"'
  info "VM setup complete!"
}

# --- Cross-compile Windows binaries ---
cross_compile() {
  if [[ "$SKIP_BUILD" == true ]]; then
    info "Skipping cross-compilation (--skip-build)"
    return
  fi

  step "Cross-compiling Windows binaries..."

  # cargo-xwin needs llvm-lib from LLVM for archiving static libraries.
  # On macOS, LLVM is keg-only (not in PATH by default).
  local llvm_prefix
  llvm_prefix="$(brew --prefix llvm 2>/dev/null || true)"
  if [[ -n "$llvm_prefix" && -d "$llvm_prefix/bin" ]]; then
    export PATH="$llvm_prefix/bin:$PATH"
  fi

  cd "$PROJECT_ROOT"

  info "Building NAPI binding..."
  CARGO=cargo-xwin pnpm --filter=vite-plus build-native --target "$TARGET"

  info "Building vp.exe..."
  cargo xwin build --release --target "$TARGET" -p vite_global_cli

  info "Building vp-shim.exe..."
  cargo xwin build --release --target "$TARGET" -p vite_trampoline

  info "Building TypeScript packages..."
  pnpm --filter @rolldown/pluginutils build
  pnpm --filter rolldown build-node
  pnpm --filter vite build-types
  pnpm --filter "@voidzero-dev/*" build
  pnpm --filter vite-plus build-ts
}

# --- Ensure VM is running ---
ensure_vm_running() {
  step "Checking Windows VM status..."

  if [[ "$RESET" == true ]]; then
    info "Resetting VM overlay..."
    (cd "$QWIN_DIR" && ./run.sh --host --reset) &
  elif ssh_cmd "echo ok" 2>/dev/null; then
    info "VM is already running and accessible via SSH."
    return
  else
    info "Starting Windows VM..."
    (cd "$QWIN_DIR" && ./run.sh --host) &
  fi

  # Wait for SSH to become available
  info "Waiting for SSH (this may take 2-5 minutes on macOS)..."
  local retries=0
  local max_retries=120  # 10 minutes max
  while ! ssh_cmd "echo ok" 2>/dev/null; do
    retries=$((retries + 1))
    if [[ $retries -ge $max_retries ]]; then
      error "Timed out waiting for SSH after $((max_retries * 5)) seconds."
      exit 1
    fi
    sleep 5
  done
  info "SSH is ready."
}

# --- Transfer artifacts ---
transfer_artifacts() {
  step "Creating transfer bundle..."

  cd "$PROJECT_ROOT"

  # Create a slim bundle without node_modules (4 MB vs 1.4 GB).
  # Dependencies are installed on the VM via npm/tsx.
  tar czf "$BUNDLE_FILE" \
    --exclude='.git' \
    --exclude='target' \
    --exclude='rolldown' \
    --exclude='vite' \
    --exclude='tools/qwin' \
    --exclude='bench' \
    --exclude='docs' \
    --exclude='crates' \
    --exclude='node_modules' \
    --exclude='*/node_modules' \
    --exclude='*.qcow2' \
    --exclude='*.iso' \
    --exclude='*.node' \
    packages/ \
    package.json

  local bundle_size
  bundle_size=$(du -h "$BUNDLE_FILE" | cut -f1)
  info "Bundle size: $bundle_size"

  step "Transferring to VM..."

  # Transfer bundle + binaries
  scp_to_vm \
    "$BUNDLE_FILE" \
    "target/$TARGET/release/vp.exe" \
    "target/$TARGET/release/vp-shim.exe" \
    "packages/cli/binding/vite-plus.win32-x64-msvc.node"

  # Download and transfer @oxc-node/core Windows native binary
  local oxc_node_version
  oxc_node_version=$(node -p "require('./packages/tools/node_modules/@oxc-node/core/package.json').version" 2>/dev/null || echo "0.1.0")
  local oxc_node_tgz="/tmp/oxc-node-core-win32-x64-msvc-${oxc_node_version}.tgz"
  if [[ ! -f "$oxc_node_tgz" ]]; then
    npm pack "@oxc-node/core-win32-x64-msvc@${oxc_node_version}" --pack-destination /tmp
  fi
  local oxc_tmp="/tmp/oxc-node-win-$$"
  mkdir -p "$oxc_tmp" && tar xzf "$oxc_node_tgz" -C "$oxc_tmp"
  scp_to_vm "$oxc_tmp/package/oxc-node.win32-x64-msvc.node"
  rm -rf "$oxc_tmp"

  step "Extracting on VM..."

  ssh_cmd 'cmd /c "rd /s /q C:\Users\Administrator\vite-plus 2>nul"' || true
  ssh_cmd 'cmd /c "mkdir C:\Users\Administrator\vite-plus & tar xzf C:\Users\Administrator\vp-win-test-bundle.tar.gz -C C:\Users\Administrator\vite-plus & del C:\Users\Administrator\vp-win-test-bundle.tar.gz"'

  # Place binaries
  ssh_cmd 'cmd /c "mkdir C:\Users\Administrator\vite-plus\target\x86_64-pc-windows-msvc\release"'
  ssh_cmd 'cmd /c "move C:\Users\Administrator\vp.exe C:\Users\Administrator\vite-plus\target\x86_64-pc-windows-msvc\release\vp.exe"'
  ssh_cmd 'cmd /c "move C:\Users\Administrator\vp-shim.exe C:\Users\Administrator\vite-plus\target\x86_64-pc-windows-msvc\release\vp-shim.exe"'
  ssh_cmd 'cmd /c "move C:\Users\Administrator\vite-plus.win32-x64-msvc.node C:\Users\Administrator\vite-plus\packages\cli\binding\vite-plus.win32-x64-msvc.node"'

  # Install vp globally
  ssh_cmd 'cmd /c "mkdir %USERPROFILE%\.vite-plus\bin 2>nul"'
  ssh_cmd 'cmd /c "copy C:\Users\Administrator\vite-plus\target\x86_64-pc-windows-msvc\release\vp.exe %USERPROFILE%\.vite-plus\bin\vp.exe"'
  ssh_cmd 'cmd /c "copy C:\Users\Administrator\vite-plus\target\x86_64-pc-windows-msvc\release\vp-shim.exe %USERPROFILE%\.vite-plus\bin\vp-shim.exe"'

  # Install snap test runner dependencies (tools package)
  # Resolve catalog: versions to real npm versions to avoid pnpm workspace coupling
  info "Installing snap test runner dependencies..."
  local tools_pkg="$PROJECT_ROOT/packages/tools/package.json"
  local resolved_pkg
  resolved_pkg=$(node -e "
    const pkg = require('$tools_pkg');
    const ws = require('$PROJECT_ROOT/pnpm-workspace.yaml'.replace(/\.yaml$/, ''));
    // Simple catalog resolution - read from pnpm-workspace catalog
    const resolve = (deps) => {
      const out = {};
      for (const [k, v] of Object.entries(deps || {})) {
        if (v === 'catalog:' || v.startsWith('catalog:')) out[k] = '*';
        else if (v.startsWith('workspace:')) continue; // skip workspace deps
        else out[k] = v;
      }
      return out;
    };
    const result = { ...pkg, dependencies: resolve(pkg.dependencies), devDependencies: resolve(pkg.devDependencies) };
    delete result.devDependencies; // skip dev deps to keep it lean
    console.log(JSON.stringify(result, null, 2));
  " 2>/dev/null || cat "$tools_pkg")

  # Write resolved package.json and install on VM
  echo "$resolved_pkg" > /tmp/tools-package.json
  scp_to_vm /tmp/tools-package.json
  ssh_cmd 'cmd /c "copy C:\Users\Administrator\tools-package.json C:\Users\Administrator\vite-plus\packages\tools\package.json /y & del C:\Users\Administrator\tools-package.json"'
  ssh_cmd 'cmd /c "cd C:\Users\Administrator\vite-plus\packages\tools && npm install --ignore-scripts"'

  # Place @oxc-node Windows native binary
  ssh_cmd 'cmd /c "move C:\Users\Administrator\oxc-node.win32-x64-msvc.node C:\Users\Administrator\vite-plus\packages\tools\node_modules\@oxc-node\core\oxc-node.win32-x64-msvc.node"'

  # Create junction for pirates (needed by @oxc-node/core)
  ssh_cmd 'cmd /c "if not exist C:\Users\Administrator\vite-plus\packages\tools\node_modules\pirates (mklink /J C:\Users\Administrator\vite-plus\packages\tools\node_modules\pirates C:\Users\Administrator\vite-plus\packages\tools\node_modules\@oxc-node\core\node_modules\pirates 2>nul)"' || true

  # Verify vp works
  ssh_cmd 'cmd /c "%USERPROFILE%\.vite-plus\bin\vp.exe --version"'
  info "Transfer complete."
}

# --- Run snap tests ---
run_snap_tests() {
  step "Running snap tests..."

  local shard_arg=""
  [[ -n "$SHARD" ]] && shard_arg="--shard=$SHARD"

  local filter_arg=""
  [[ -n "$FILTER" ]] && filter_arg="$FILTER"

  local tsx_cmd="tsx"
  local snap_test_ts="C:\\Users\\Administrator\\vite-plus\\packages\\tools\\src\\snap-test.ts"

  if [[ "$RUN_LOCAL" == true ]]; then
    info "Running local snap tests..."
    ssh_cmd "cmd /c \"set PATH=%USERPROFILE%\\.vite-plus\\bin;C:\\Git\\cmd;%PATH% && cd C:\\Users\\Administrator\\vite-plus\\packages\\cli && $tsx_cmd $snap_test_ts $shard_arg $filter_arg 2>&1\"" || true
  fi

  if [[ "$RUN_GLOBAL" == true ]]; then
    info "Running global snap tests..."
    ssh_cmd "cmd /c \"set PATH=%USERPROFILE%\\.vite-plus\\bin;C:\\Git\\cmd;%PATH% && cd C:\\Users\\Administrator\\vite-plus\\packages\\cli && $tsx_cmd $snap_test_ts --dir snap-tests-global --bin-dir %USERPROFILE%\\.vite-plus\\bin $shard_arg $filter_arg 2>&1\"" || true
  fi
}

# --- Retrieve results ---
retrieve_results() {
  step "Retrieving updated snap.txt files..."

  cd "$PROJECT_ROOT"

  for dir in snap-tests snap-tests-global; do
    local remote_path="$REMOTE_PROJECT\\packages\\cli\\$dir"
    local local_path="packages/cli/$dir"

    # Get list of snap.txt files on the remote
    local snap_files
    snap_files=$(ssh_cmd "cmd /c \"dir /b /s C:\\Users\\Administrator\\vite-plus\\packages\\cli\\$dir\\snap.txt 2>nul\"" 2>/dev/null || true)

    if [[ -n "$snap_files" ]]; then
      while IFS= read -r remote_file; do
        remote_file=$(echo "$remote_file" | tr -d '\r')
        if [[ -n "$remote_file" ]]; then
          # Convert Windows path to relative path
          local rel_path="${remote_file#*\\packages\\cli\\$dir\\}"
          rel_path=$(echo "$rel_path" | tr '\\' '/')
          local local_file="$local_path/$rel_path"
          mkdir -p "$(dirname "$local_file")"
          # shellcheck disable=SC2086
          scp $SSH_OPTS "$SSH_USER@$SSH_HOST:\"$remote_file\"" "$local_file" 2>/dev/null || true
        fi
      done <<< "$snap_files"
    fi
  done

  step "Results retrieved. Checking for differences..."
  git diff --stat -- 'packages/cli/snap-tests*/*/snap.txt' || true
}

# --- Main ---
main() {
  check_prerequisites
  ensure_vm_running

  if [[ "$SETUP" == true ]]; then
    setup_vm
    exit 0
  fi

  cross_compile
  transfer_artifacts
  run_snap_tests
  retrieve_results

  step "Done!"
  info "Review snap.txt changes with: git diff -- 'packages/cli/snap-tests*/*/snap.txt'"
}

main
