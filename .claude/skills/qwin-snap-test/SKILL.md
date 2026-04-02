---
name: qwin-snap-test
description: Cross-compile Windows binaries on macOS and run snap tests on a qwin QEMU Windows VM. Use when setting up or running Windows snap tests locally.
---

# Cross-Compile on macOS + Run Snap Tests on Windows via qwin

## Overview

This workflow enables macOS developers to cross-compile vite-plus Windows binaries and run snap tests inside a QEMU Windows VM (qwin), without needing a real Windows machine.

## Prerequisites

```bash
# Install tools on macOS
brew install qemu cdrtools llvm
cargo install cargo-xwin
rustup target add x86_64-pc-windows-msvc
```

## One-Time VM Setup

### 1. Initialize qwin submodule

```bash
git submodule update --init tools/qwin
```

### 2. Configure and build Windows VM

```bash
cd tools/qwin
cp .env.example .env
# Edit .env: set WIN_ISO_URL to a Windows Server evaluation ISO
# Get free ISO from https://www.microsoft.com/en-us/evalcenter/evaluate-windows-server
./build.sh --host    # Takes ~60+ min on macOS (TCG, no KVM)
```

### 3. Install dev tools in VM

After the VM build completes and SSH is accessible (port 2222):

```bash
./scripts/qwin-snap-test.sh --setup
```

This installs: Node.js, Git (MinGit), pnpm, VC++ Redistributable, tsx, and enables long paths.

## Daily Workflow

### Run snap tests

```bash
# Full workflow: cross-compile + transfer + test
./scripts/qwin-snap-test.sh [filter]

# Skip build (reuse existing binaries)
./scripts/qwin-snap-test.sh --skip-build [filter]

# Run specific test types
./scripts/qwin-snap-test.sh --local command-helper
./scripts/qwin-snap-test.sh --global command-add-npm10

# With sharding
./scripts/qwin-snap-test.sh --shard=1/3
```

### VM management

```bash
cd tools/qwin
./run.sh              # Boot from overlay (~2-5 min on macOS)
./run.sh --reset      # Discard all changes, revert to post-install state
```

## Cross-Compilation Details

### Build commands

```bash
# LLVM must be in PATH for cargo-xwin (needs llvm-lib)
export PATH="$(brew --prefix llvm)/bin:$PATH"

# NAPI binding
CARGO=cargo-xwin pnpm --filter=vite-plus build-native --target x86_64-pc-windows-msvc

# CLI binary (vp.exe)
cargo xwin build --release --target x86_64-pc-windows-msvc -p vite_global_cli

# Trampoline (vp-shim.exe)
cargo xwin build --release --target x86_64-pc-windows-msvc -p vite_trampoline
```

### Output artifacts

| Artifact        | Location                                             | Type                 |
| --------------- | ---------------------------------------------------- | -------------------- |
| `vp.exe`        | `target/x86_64-pc-windows-msvc/release/vp.exe`       | PE32+ console x86-64 |
| `vp-shim.exe`   | `target/x86_64-pc-windows-msvc/release/vp-shim.exe`  | PE32+ console x86-64 |
| `.node` binding | `packages/cli/binding/vite-plus.win32-x64-msvc.node` | PE32+ DLL x86-64     |

### TLS prerequisite

Windows crates must use `rustls-no-provider` instead of `native-tls-vendored` for cross-compilation. Three crates need this change:

- `crates/vite_error/Cargo.toml`
- `crates/vite_install/Cargo.toml`
- `crates/vite_js_runtime/Cargo.toml`

And `crates/vite_shared/Cargo.toml` must have `rustls` as an unconditional dependency (not gated behind `cfg(not(windows))`).

## Artifact Transfer

### What to transfer (~4 MB without node_modules)

The wrapper script creates a slim tarball excluding node_modules, then installs dependencies on the VM via `npm install`.

**Key tar exclude rules:**

- `--exclude='tools/qwin'` (NOT `--exclude='tools'` which would also exclude `packages/tools`)
- `--exclude='node_modules'` and `--exclude='*/node_modules'`
- No `-h` flag (avoid dereferencing pnpm symlinks which bloats to 1.4 GB)

### VM-side dependency installation

Since pnpm's symlink-based node_modules don't transfer across platforms:

1. Transfer project files without node_modules
2. Create a resolved `package.json` for `packages/tools` (replace `catalog:` with real versions, remove `workspace:*` deps)
3. Run `npm install --ignore-scripts` in `packages/tools/` on the VM
4. Place `@oxc-node/core` Windows native binary (`oxc-node.win32-x64-msvc.node`) manually

### Windows-specific requirements

- **VC++ Redistributable**: Required for MSVC-compiled Rust binaries (`VCRUNTIME140.dll`)
- **Long paths**: `reg add HKLM\SYSTEM\CurrentControlSet\Control\FileSystem /v LongPathsEnabled /t REG_DWORD /d 1 /f`
- **tsx**: Use instead of `@oxc-node/core` register hook (which has a target config issue on Windows)
- **@oxc-node Windows binary**: Download via `npm pack @oxc-node/core-win32-x64-msvc@<version>`

## SSH Access

```bash
# Default connection
ssh -p 2222 -o StrictHostKeyChecking=no administrator@localhost

# Default password: P@ssw0rd! (configure SSH_PUBKEY in qwin .env for key auth)

# Run commands
ssh -p 2222 administrator@localhost 'cmd /c "node --version"'
```

## Performance on macOS

| Operation                         | Time                     |
| --------------------------------- | ------------------------ |
| First-time VM build               | ~60+ min (TCG, one-time) |
| VM boot from overlay              | ~2-5 min                 |
| Cross-compilation (all 3 targets) | ~2-3 min                 |
| Artifact transfer (SCP)           | ~30s                     |
| Snap test execution               | ~3-5x slower than native |

## Troubleshooting

### "Cannot find module" errors

Pnpm symlinks don't work across platforms. Use `npm install` on the VM instead of transferring node_modules.

### Exit code 53 from vp.exe

Missing VC++ Redistributable. Install via `--setup` or manually:

```bash
curl -fSL -o /tmp/vc_redist.x64.exe "https://aka.ms/vs/17/release/vc_redist.x64.exe"
scp -P 2222 /tmp/vc_redist.x64.exe administrator@localhost:
ssh -p 2222 administrator@localhost 'cmd /c "vc_redist.x64.exe /install /quiet /norestart"'
```

### "top-level await not available" from @oxc-node

Use `tsx` instead of `@oxc-node/core` register hook on Windows.

### SCP "dest open: Failure"

Use simple paths without backslashes: `scp file administrator@localhost:filename` (puts in home dir).

### Tar excludes too much

Use `--exclude='tools/qwin'` not `--exclude='tools'` to preserve `packages/tools/`.
