# RFC: Standalone Windows `.exe` Installer

## Status

Implemented

## Summary

Add a standalone `vp-setup.exe` Windows installer binary, distributed via GitHub Releases, that installs the vp CLI without requiring PowerShell. This complements the existing `irm https://vite.plus/ps1 | iex` script-based installer. Modeled after `rustup-init.exe`.

## Motivation

### The Problem

The current Windows installation requires running a PowerShell command:

```powershell
irm https://vite.plus/ps1 | iex
```

This has several friction points:

1. **Execution policy barriers**: Many corporate/enterprise Windows machines restrict PowerShell script execution (`Set-ExecutionPolicy` changes required).
2. **No cmd.exe support**: Users in `cmd.exe` or Git Bash cannot use the `irm | iex` idiom without first opening PowerShell.
3. **No double-click install**: Users following documentation cannot simply download-and-run an installer.
4. **CI friction**: GitHub Actions using `shell: cmd` or `shell: bash` on Windows need workarounds to invoke PowerShell.
5. **PowerShell version fragmentation**: PowerShell 5.1 (built-in) and PowerShell 7+ (pwsh) have subtle differences that the script must handle.

### rustup Reference

rustup provides `rustup-init.exe` — a single console binary that users download and run from any shell or by double-clicking. Key characteristics:

- Console-only (no GUI), interactive prompts with numbered menu
- Silent mode via `-y` flag for CI
- Single binary that is both installer and main tool (detects behavior from `argv[0]`)
- Modifies Windows User PATH via registry
- Registers in Add/Remove Programs
- DLL security mitigations for download-folder execution

## Goals

1. Provide a single `.exe` that installs vp from any Windows shell or double-click
2. Support silent/unattended installation for CI environments
3. Reuse existing installation logic from the `vp upgrade` command
4. Keep the installer binary small (target: 3-5 MB)
5. Replicate the exact same installation result as `install.ps1`

## Non-Goals

1. GUI installer (MSI, NSIS, Inno Setup) — console-only like rustup
2. Cross-platform installer binary (Linux/macOS are well-served by `install.sh`)
3. winget/chocolatey/scoop package submission (future work)
4. Code signing (required for GA, but out of scope for this RFC)

## Architecture Decision: Single Binary vs. Separate Crate

### Option A: Single Binary (rustup model)

rustup uses one binary for everything — `rustup-init.exe` copies itself to `~/.cargo/bin/rustup.exe` and changes behavior based on `argv[0]`. This works because rustup IS the toolchain manager.

**Not suitable for vp** because:

- `vp.exe` is downloaded from the npm registry as a platform-specific package
- The installer cannot copy itself as `vp.exe` — they are fundamentally different binaries
- `vp.exe` links `vite_js_runtime`, `vite_workspace`, `oxc_resolver` (~15-20 MB) — the installer needs none of these

### Option B: Separate Crate with Shared Library (recommended)

Create two new crates:

```
crates/vite_setup/     — shared installation logic (library)
crates/vite_installer/      — standalone installer binary
```

`vite_setup` extracts the reusable installation logic currently in `vite_global_cli/src/commands/upgrade/`. Both `vp upgrade` and `vp-setup.exe` call into `vite_setup`.

**Benefits:**

- Installer binary stays small (3-5 MB)
- `vp upgrade` and `vp-setup.exe` share identical installation logic — no drift
- Clear separation of concerns

## Code Sharing: The `vite_setup` Library

### What Gets Extracted

| Original location in `upgrade/` | Extracted to `vite_setup::` | Purpose                                                                                                                              |
| ------------------------------- | --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `platform.rs`                   | `platform`                  | OS/arch detection                                                                                                                    |
| `registry.rs`                   | `registry`                  | npm registry queries                                                                                                                 |
| `integrity.rs`                  | `integrity`                 | SHA-512 verification                                                                                                                 |
| `install.rs` (all functions)    | `install`                   | Tarball extraction, package.json generation, .npmrc overrides, dep install, symlink/junction swap, version cleanup, rollback support |

### What Stays in `vite_global_cli`

- CLI argument parsing for `vp upgrade`
- Version comparison (current vs available)
- Rollback logic
- Output formatting specific to upgrade UX

### What's New in `vite_installer`

- Interactive installation prompts (numbered menu)
- Windows User PATH modification via registry
- Node.js version manager setup prompt
- Shell env file creation
- Existing installation detection
- DLL security mitigations (for download-folder execution)

### Dependency Graph

```
vite_installer (binary, ~3-5 MB)
  ├── vite_setup (shared installation logic)
  ├── vite_install (HTTP client)
  ├── vite_shared (home dir resolution)
  ├── vite_path (typed path wrappers)
  ├── clap (CLI parsing)
  ├── tokio (async runtime)
  ├── indicatif (progress bars)
  └── owo-colors (terminal colors)

vite_global_cli (existing)
  ├── vite_setup (replaces inline upgrade code)
  └── ... (all existing deps)
```

## User Experience

### Interactive Mode (default)

When run without flags (double-click or plain `vp-setup.exe`):

```
Welcome to Vite+ Installer!

This will install the vp CLI and monorepo task runner.

    Install directory: C:\Users\alice\.vite-plus
    PATH modification: C:\Users\alice\.vite-plus\bin → User PATH
    Version:           latest
    Node.js manager:   enabled

  1) Proceed with installation (default)
  2) Customize installation
  3) Cancel

  >
```

The Node.js manager value is pre-computed via auto-detection before the menu is shown (see [Node.js Manager Auto-Detection](#nodejs-manager-auto-detection)). The user can override it in the customize submenu before proceeding.

Customization submenu:

```
  Customize installation:

    1) Version:         [latest]
    2) npm registry:    [(default)]
    3) Node.js manager: [enabled]
    4) Modify PATH:     [yes]

  Enter option number to change, or press Enter to go back:
  >
```

### Silent Mode (CI)

The installer auto-detects CI environments (`CI=true`) and skips interactive prompts, so `-y` is not required in CI:

```bash
# CI environments are automatically non-interactive
vp-setup.exe

# Explicit silent mode (outside CI)
vp-setup.exe -y

# Customize
vp-setup.exe --version 0.3.0 --no-node-manager --registry https://registry.npmmirror.com
```

### CLI Flags

| Flag                   | Description                   | Default                      |
| ---------------------- | ----------------------------- | ---------------------------- |
| `-y` / `--yes`         | Accept defaults, no prompts   | interactive                  |
| `-q` / `--quiet`       | Suppress output except errors | false                        |
| `--version <VER>`      | Install specific version      | latest                       |
| `--tag <TAG>`          | npm dist-tag                  | latest                       |
| `--install-dir <PATH>` | Installation directory        | `%USERPROFILE%\.vite-plus`   |
| `--registry <URL>`     | npm registry URL              | `https://registry.npmjs.org` |
| `--no-node-manager`    | Skip Node.js manager setup    | auto-detect                  |
| `--no-modify-path`     | Don't modify User PATH        | modify                       |

### Environment Variables (compatible with `install.ps1`)

| Variable                  | Maps to             |
| ------------------------- | ------------------- |
| `VP_VERSION`              | `--version`         |
| `VP_HOME`                 | `--install-dir`     |
| `NPM_CONFIG_REGISTRY`     | `--registry`        |
| `VP_NODE_MANAGER=yes\|no` | `--no-node-manager` |

CLI flags take precedence over environment variables.

## Installation Flow

The installer replicates the same result as `install.ps1`, implemented in Rust via `vite_setup`.

```
┌─────────────────────────────────────────────────────────────┐
│                      RESOLVE                                │
│                                                             │
│  ┌─ detect platform ──────── win32-x64-msvc                 │
│  │                           win32-arm64-msvc                │
│  │                                                          │
│  ├─ check existing ──────── read %VP_HOME%\current           │
│  │                                                          │
│  └─ resolve version ──────── resolve_version_string()        │
│                              1 HTTP call: "latest" → "0.3.0" │
│                              same version? → skip to         │
│                              CONFIGURE (repair path)         │
└─────────────────────────────────────────────────────────────┘
                              │
                   (only if version differs)
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      DOWNLOAD & VERIFY                      │
│                                                             │
│  ┌─ resolve platform pkg ── resolve_platform_package()       │
│  │                          2nd HTTP call: tarball URL + SRI │
│  │                                                          │
│  ├─ download tarball ─────── GET tarball URL from registry   │
│  │                           progress spinner via indicatif  │
│  │                                                          │
│  └─ verify integrity ─────── SHA-512 SRI hash comparison     │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      INSTALL                                │
│                                                             │
│  ┌─ extract binary ──────── %VP_HOME%\{version}\bin\         │
│  │                          vp.exe + vp-shim.exe             │
│  │                                                          │
│  ├─ generate package.json ─ wrapper with vite-plus dep       │
│  │                          pins pnpm@10.33.0                │
│  │                                                          │
│  ├─ write .npmrc ────────── minimum-release-age=0            │
│  │                                                          │
│  └─ install deps ────────── spawn: vp install --silent       │
│                              installs vite-plus + transitive │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     ACTIVATE              ◄── point of no    │
│                                               return         │
│  ┌─ save previous version ── .previous-version (rollback)    │
│  │                          (only if upgrading existing)     │
│  │                                                          │
│  ├─ swap current ────────── mklink /J current → {version}    │
│  │                          (junction on Windows,            │
│  │                           atomic symlink on Unix)         │
│  │                                                          │
│  └─ cleanup old versions ── keep last 5 by creation time     │
│                              protects new + previous version │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│       CONFIGURE         (best-effort, always runs,           │
│                          even for same-version repair)        │
│                                                             │
│  ┌─ create bin shims ────── copy vp-shim.exe → bin\vp.exe   │
│  │                          (rename-to-.old if running)      │
│  │                                                          │
│  ├─ Node.js manager ────── if enabled (pre-computed):        │
│  │                            spawn: vp env setup --refresh  │
│  │                          if disabled:                     │
│  │                            spawn: vp env setup --env-only │
│  │                                                          │
│  └─ modify User PATH ────── if --no-modify-path not set:     │
│                              HKCU\Environment\Path           │
│                              prepend %VP_HOME%\bin           │
│                              broadcast WM_SETTINGCHANGE      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
                        ✔ Print success
```

Each phase maps to `vite_setup` library functions shared with `vp upgrade`:

| Phase             | Key function                               | Crate            |
| ----------------- | ------------------------------------------ | ---------------- |
| Resolve           | `platform::detect_platform_suffix()`       | `vite_setup`     |
| Resolve           | `install::read_current_version()`          | `vite_setup`     |
| Resolve           | `registry::resolve_version_string()`       | `vite_setup`     |
| Download & Verify | `registry::resolve_platform_package()`     | `vite_setup`     |
| Download & Verify | `HttpClient::get_bytes()`                  | `vite_install`   |
| Download & Verify | `integrity::verify_integrity()`            | `vite_setup`     |
| Install           | `install::extract_platform_package()`      | `vite_setup`     |
| Install           | `install::generate_wrapper_package_json()` | `vite_setup`     |
| Install           | `install::write_release_age_overrides()`   | `vite_setup`     |
| Install           | `install::install_production_deps()`       | `vite_setup`     |
| Activate          | `install::save_previous_version()`         | `vite_setup`     |
| Activate          | `install::swap_current_link()`             | `vite_setup`     |
| Activate          | `install::cleanup_old_versions()`          | `vite_setup`     |
| Configure         | `install::refresh_shims()`                 | `vite_setup`     |
| Configure         | `windows_path::add_to_user_path()`         | `vite_installer` |

**Same-version repair**: When the resolved version matches the installed version, the DOWNLOAD/INSTALL/ACTIVATE phases are skipped entirely (saving 1 HTTP request + all I/O). The CONFIGURE phase always runs to repair shims, env files, and PATH if needed.

**Failure recovery**: Before the **Activate** phase, failures clean up the version directory and leave the existing installation untouched. After **Activate**, all CONFIGURE steps are best-effort — failures log a warning but do not cause exit code 1. Rerunning the installer always retries CONFIGURE.

## Node.js Manager Auto-Detection

The Node.js manager decision (`enabled`/`disabled`) is pre-computed before the interactive menu is shown, so the user sees the resolved value and can override it via the customize submenu. No prompts occur during the installation phase.

The auto-detection logic matches `install.ps1`/`install.sh`:

| Priority | Condition                                 | Result   |
| -------- | ----------------------------------------- | -------- |
| 1        | `--no-node-manager` CLI flag              | disabled |
| 2        | `VP_NODE_MANAGER=yes`                     | enabled  |
| 3        | `VP_NODE_MANAGER=no`                      | disabled |
| 4        | `bin/node.exe` shim already exists        | enabled  |
| 5        | CI / Codespaces / DevContainer / DevPod   | enabled  |
| 6        | No system `node` found                    | enabled  |
| 7        | System `node` present, interactive mode   | enabled  |
| 8        | System `node` present, silent mode (`-y`) | disabled |

In interactive mode (rules 7), the default matches `install.ps1`'s Y/n prompt where pressing Enter enables it. The user can disable it in the customize menu before installation begins. In silent mode (rule 8), shims are not created unless explicitly requested, avoiding silently taking over an existing Node toolchain.

## Windows-Specific Details

### PATH Modification via Registry

Same approach as rustup and `install.ps1`, using the `winreg` crate for registry access:

```rust
let hkcu = RegKey::predef(HKEY_CURRENT_USER);
let env = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;
let current: String = env.get_value("Path").unwrap_or_default();
// ... check if already present (case-insensitive, handles trailing backslash)
// ... prepend bin_dir, write back as REG_EXPAND_SZ
// ... broadcast WM_SETTINGCHANGE via SendMessageTimeoutW (raw FFI, single call)
```

See `crates/vite_installer/src/windows_path.rs` for the full implementation.

### DLL Security (for download-folder execution)

Following rustup's approach — when the `.exe` is downloaded to `Downloads/` and double-clicked, malicious DLLs in the same folder could be loaded. Two mitigations, both using raw FFI (no `windows-sys` crate):

```rust
// build.rs — linker-time: restrict DLL search at load time
#[cfg(windows)]
println!("cargo:rustc-link-arg=/DEPENDENTLOADFLAG:0x800");

// main.rs — runtime: restrict DLL search via Win32 API
#[cfg(windows)]
fn init_dll_security() {
    unsafe extern "system" {
        fn SetDefaultDllDirectories(directory_flags: u32) -> i32;
    }
    const LOAD_LIBRARY_SEARCH_SYSTEM32: u32 = 0x0000_0800;
    unsafe { SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32); }
}
```

### Console Allocation

The binary uses the console subsystem (default for Rust binaries on Windows). When double-clicked, Windows allocates a console window automatically. No special handling needed.

### Existing Installation Handling

| Scenario                                  | Behavior                                                     |
| ----------------------------------------- | ------------------------------------------------------------ |
| No existing install                       | Fresh install                                                |
| Same version installed                    | Skip download, rerun CONFIGURE phase (repair shims/PATH/env) |
| Different version installed               | Upgrade to target version                                    |
| Corrupt/partial install (broken junction) | Recreate directory structure                                 |
| Running `vp.exe` in bin/                  | Rename to `.old`, copy new (same as trampoline pattern)      |

## Add/Remove Programs Registration

**Phase 1: Skip.** `vp implode` already handles full uninstallation.

**Phase 2: Register.** Write to `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\VitePlus`:

```
DisplayName     = "Vite+"
UninstallString = "C:\Users\alice\.vite-plus\current\bin\vp.exe implode --yes"
DisplayVersion  = "0.3.0"
Publisher       = "VoidZero"
InstallLocation = "C:\Users\alice\.vite-plus"
```

## Distribution

### Phase 1: GitHub Releases

Attach installer binaries to each GitHub Release:

- `vp-setup-x86_64-pc-windows-msvc.exe`
- `vp-setup-aarch64-pc-windows-msvc.exe`

The release workflow already creates GitHub Releases. Add build + upload steps for the init binary.

### Phase 2: Direct Download URL (done)

`https://viteplus.dev/vp-setup` redirects (302) to `https://setup.viteplus.dev` via Netlify redirect in `netlify.toml`. Installation docs link to the user-facing `viteplus.dev` URL.

### Phase 3: Package Managers

Submit to winget, chocolatey, scoop. Each has its own manifest format and review process.

## CI/Build Changes

### Release Workflow Additions

In `build-upstream/action.yml`, the installer binary is built and cached alongside the CLI:

```yaml
- name: Build installer binary (Windows only)
  if: contains(inputs.target, 'windows')
  run: cargo build --release --target ${{ inputs.target }} -p vite_installer
```

In `release.yml`, installer artifacts are uploaded per-target, renamed with the target triple, and attached to the GitHub Release:

```yaml
- name: Upload installer binary artifact (Windows only)
  if: contains(matrix.settings.target, 'windows')
  uses: actions/upload-artifact@v4
  with:
    name: vp-setup-${{ matrix.settings.target }}
    path: ./target/${{ matrix.settings.target }}/release/vp-setup.exe
```

### Test Workflow

`test-standalone-install.yml` includes a `test-vp-setup-exe` job that builds the installer from source, installs via pwsh, and verifies from all three shells (pwsh, cmd, bash):

```yaml
test-vp-setup-exe:
  name: Test vp-setup.exe (pwsh)
  runs-on: windows-latest
  steps:
    - uses: actions/checkout@v4
    - uses: oxc-project/setup-rust@v1
    - name: Build vp-setup.exe
      run: cargo build --release -p vite_installer
    - name: Install via vp-setup.exe (silent)
      shell: pwsh
      run: ./target/release/vp-setup.exe
      env:
        VP_VERSION: alpha
    - name: Verify installation (pwsh/cmd/bash)
      # verifies from all three shells after a single install
```

The workflow triggers on changes to `crates/vite_installer/**` and `crates/vite_setup/**`.

## Code Signing

Windows Defender SmartScreen flags unsigned executables downloaded from the internet. This is a significant UX problem for a download-and-run installer.

**Recommendation**: Obtain an EV (Extended Validation) code signing certificate before GA release. EV certificates immediately remove SmartScreen warnings (no reputation building period needed).

This is an organizational decision (cost: ~$300-500/year) and out of scope for the implementation, but critical for user experience.

## Binary Size Budget

Target: 3-5 MB (release, stripped, LTO).

Key dependencies and their approximate contribution:

| Dependency                        | Purpose                | Size impact |
| --------------------------------- | ---------------------- | ----------- |
| `reqwest` + `native-tls-vendored` | HTTP + TLS             | ~1.5 MB     |
| `flate2` + `tar`                  | Tarball extraction     | ~200 KB     |
| `clap`                            | CLI parsing            | ~300 KB     |
| `tokio` (minimal features)        | Async runtime          | ~400 KB     |
| `indicatif`                       | Progress bars          | ~100 KB     |
| `sha2`                            | Integrity verification | ~50 KB      |
| `serde_json`                      | Registry JSON parsing  | ~200 KB     |
| `winreg` + `windows-sys`          | Windows registry       | ~50-100 KB  |
| Rust std + overhead               |                        | ~500 KB     |

Use `opt-level = "z"` (optimize for size) in package profile override, matching the trampoline approach.

## Alternatives Considered

### 1. MSI/NSIS/Inno Setup Installer (Rejected)

Traditional Windows installers provide GUI, Add/Remove Programs, and Start Menu integration. However:

- Adds build-time dependency on external tooling (WiX, NSIS)
- GUI is unnecessary for a developer CLI tool
- MSI has complex authoring requirements
- rustup chose console-only and it works well for the developer audience

### 2. Extend `vp.exe` with Init Mode (Rejected)

Like rustup, make `vp.exe` detect when called as `vp-setup.exe` and switch to installer mode.

- Would bloat the installer to ~15-20 MB (all of vp's dependencies)
- vp.exe is downloaded FROM the installer — circular dependency
- The installation payload (vp.exe) and the installer are fundamentally different

### 3. Static-linked PowerShell in .exe (Rejected)

Embed the PowerShell script in a self-extracting exe. Fragile, still requires PowerShell runtime.

### 4. Use `winreg` Crate vs Raw FFI for PATH (Decision: `winreg`)

- `winreg` crate: Higher-level safe API, ~50-100 KB after LTO, significantly less code (~80 lines vs ~225 lines)
- Raw Win32 FFI: Zero dependencies but 225 lines of unsafe code with manual UTF-16 encoding and registry choreography
- PowerShell subprocess: Proven in `install.ps1` but adds process spawn overhead and PowerShell dependency
- Decision: Use `winreg` for registry access — the zero-dependency pattern makes sense for `vite_trampoline` (copied 5-10 times as shims) but not for a single downloadable installer where readability matters more. `WM_SETTINGCHANGE` broadcast still uses a single raw FFI call since `winreg` doesn't wrap it.

## Implementation Phases

### Phase 1: Extract `vite_setup` Library (done)

- Created `crates/vite_setup/` with `platform`, `registry`, `integrity`, `install` modules
- Moved shared code from `vite_global_cli/src/commands/upgrade/` into `vite_setup`
- Updated `vite_global_cli` to import from `vite_setup`
- All 353 existing tests pass

### Phase 2: Create `vite_installer` Binary (done)

- Created `crates/vite_installer/` with `[[bin]] name = "vp-setup"`
- Implemented CLI argument parsing (clap) with env var merging
- Implemented installation flow calling `vite_setup` with same-version repair path
- Implemented Windows PATH modification via `winreg` crate
- Implemented interactive prompts with customization submenu
- Implemented Node.js manager auto-detection (pre-computed, no mid-install prompts)
- Implemented progress spinner for downloads
- Added DLL security mitigations (build.rs linker flag + runtime `SetDefaultDllDirectories`)
- Post-activation steps are best-effort (non-fatal on error)

### Phase 3: CI Integration (done)

- Added installer binary build to `build-upstream/action.yml` (Windows targets only)
- Added artifact upload and GitHub Release attachment in `release.yml`
- Added `test-vp-setup-exe` job to `test-standalone-install.yml` (cmd, pwsh, bash)
- Updated release body with `vp-setup.exe` download mention

### Phase 4: Documentation & Distribution (done)

- Updated installation docs on website (`docs/guide/index.md`)
- Added `viteplus.dev/vp-setup.exe` redirect via Netlify (`netlify.toml`)
- winget, chocolatey, scoop submission deferred to future work

## Testing Strategy

### Unit Tests

- Platform detection (mock different architectures)
- PATH modification logic (registry read/write)
- Version comparison and existing install detection

### Integration Tests (CI)

- Fresh install from cmd.exe, PowerShell, Git Bash
- Silent mode (`-y`) installation
- Custom registry, custom install dir
- Upgrade over existing installation
- Verify `vp --version` works after install
- Verify PATH is modified correctly

### Manual Tests

- Double-click from Downloads folder
- SmartScreen behavior (signed vs unsigned)
- Windows Defender scan behavior
- ARM64 Windows (if available)

## Decisions

- **Binary name**: `vp-setup.exe`
- **Uninstall**: Rely on `vp implode` — no `--uninstall` flag in the installer
- **Minimum Windows version**: Windows 10 version 1809 (October 2018 Update) or later, same as [Rust's `x86_64-pc-windows-msvc` target requirement](https://doc.rust-lang.org/rustc/platform-support.html)

## References

- [rustup-init.exe source](https://github.com/rust-lang/rustup/blob/master/src/bin/rustup-init.rs) — single-binary installer model
- [rustup self_update.rs](https://github.com/rust-lang/rustup/blob/master/src/cli/self_update.rs) — installation flow
- [rustup windows.rs](https://github.com/rust-lang/rustup/blob/master/src/cli/self_update/windows.rs) — Windows PATH/registry handling
- [RFC: Windows Trampoline](./trampoline-exe-for-shims.md) — existing Windows .exe shim approach
- [RFC: Self-Update Command](./upgrade-command.md) — existing upgrade logic to share
