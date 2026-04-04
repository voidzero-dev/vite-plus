# RFC: Standalone Windows `.exe` Installer

## Status

Draft

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

| Current location in `upgrade/` | Extracted to `vite_setup::` | Purpose |
|---|---|---|
| `platform.rs` → `detect_platform_suffix()` | `platform` | OS/arch detection |
| `registry.rs` → `resolve_version()`, `resolve_platform_package()` | `registry` | npm registry queries |
| `integrity.rs` → `verify_integrity()` | `integrity` | SHA-512 verification |
| `install.rs` → `extract_platform_package()` | `extract` | Tarball extraction |
| `install.rs` → `generate_wrapper_package_json()` | `package_json` | Wrapper package.json |
| `install.rs` → `write_release_age_overrides()` | `npmrc` | .npmrc overrides |
| `install.rs` → `install_production_deps()` | `deps` | Run `vp install --silent` |
| `install.rs` → `swap_current_link()` | `link` | Symlink/junction swap |
| `install.rs` → `cleanup_old_versions()` | `cleanup` | Old version cleanup |

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
  ├── vite_setup (new library)
  ├── vite_install (HTTP client)
  ├── vite_shared (home dir, output)
  ├── clap (CLI parsing)
  ├── tokio (async runtime)
  ├── indicatif (progress bars)
  └── junction (Windows junctions)

vite_global_cli (existing, unchanged)
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
  Node.js manager:   auto-detect

1) Proceed with installation (default)
2) Customize installation
3) Cancel

>
```

Customization submenu:

```
  Install directory  [C:\Users\alice\.vite-plus]
  Version            [latest]
  npm registry       [https://registry.npmjs.org]
  Node.js manager    [auto]
  Modify PATH        [yes]

Enter option number to change, or press Enter to go back:
>
```

### Silent Mode (CI)

```bash
# Accept all defaults
vp-setup.exe -y

# Customize
vp-setup.exe -y --version 0.3.0 --no-node-manager --registry https://registry.npmmirror.com
```

### CLI Flags

| Flag | Description | Default |
|---|---|---|
| `-y` / `--yes` | Accept defaults, no prompts | interactive |
| `-q` / `--quiet` | Suppress output except errors | false |
| `--version <VER>` | Install specific version | latest |
| `--tag <TAG>` | npm dist-tag | latest |
| `--install-dir <PATH>` | Installation directory | `%USERPROFILE%\.vite-plus` |
| `--registry <URL>` | npm registry URL | `https://registry.npmjs.org` |
| `--no-node-manager` | Skip Node.js manager setup | auto-detect |
| `--no-modify-path` | Don't modify User PATH | modify |

### Environment Variables (compatible with `install.ps1`)

| Variable | Maps to |
|---|---|
| `VP_VERSION` | `--version` |
| `VP_HOME` | `--install-dir` |
| `NPM_CONFIG_REGISTRY` | `--registry` |
| `VP_NODE_MANAGER=yes\|no` | `--no-node-manager` |

CLI flags take precedence over environment variables.

## Installation Flow

The installer performs the exact same steps as `install.ps1`, in Rust:

```
1. Detect platform          → vite_setup::platform::detect_platform_suffix()
                              (win32-x64-msvc or win32-arm64-msvc)

2. Resolve version          → vite_setup::registry::resolve_version()
                              Query npm registry for latest/specified version

3. Check existing install   → Read %VP_HOME%\current target, compare versions
                              Skip if already at target version

4. Download tarball          → vite_install::HttpClient::get_bytes()
                              With progress bar via indicatif

5. Verify integrity         → vite_setup::integrity::verify_integrity()
                              SHA-512 SRI hash from npm metadata

6. Create version dir       → %VP_HOME%\{version}\bin\

7. Extract binary           → vite_setup::extract::extract_platform_package()
                              Extracts vp.exe and vp-shim.exe

8. Generate package.json    → vite_setup::package_json::generate()
                              Wrapper package.json in version dir

9. Write .npmrc             → vite_setup::npmrc::write_release_age_overrides()
                              minimum-release-age=0

10. Install deps            → Spawn: {version_dir}\bin\vp.exe install --silent

11. Swap current junction   → vite_setup::link::swap_current_link()
                              mklink /J current → {version}

12. Create bin shims        → Copy vp-shim.exe → %VP_HOME%\bin\vp.exe

13. Setup Node.js manager   → Prompt or auto-detect, then:
                              Spawn: vp.exe env setup --refresh

14. Cleanup old versions    → vite_setup::cleanup::cleanup_old_versions()
                              Keep last 5

15. Modify User PATH        → Registry: HKCU\Environment\Path
                              Add %VP_HOME%\bin if not present
                              Broadcast WM_SETTINGCHANGE

16. Create env files        → Spawn: vp.exe env setup --env-only

17. Print success           → Show getting-started commands
```

## Windows-Specific Details

### PATH Modification via Registry

Same approach as rustup and `install.ps1`:

```rust
use winreg::RegKey;
use winreg::enums::*;

fn add_to_path(bin_dir: &str) -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;

    let current_path: String = env.get_value("Path")?;
    if !current_path.split(';').any(|p| p.eq_ignore_ascii_case(bin_dir)) {
        let new_path = format!("{bin_dir};{current_path}");
        env.set_value("Path", &new_path)?;
        // Broadcast WM_SETTINGCHANGE so other processes pick up the change
        broadcast_settings_change();
    }
    Ok(())
}
```

### DLL Security (for download-folder execution)

Following rustup's approach — when the `.exe` is downloaded to `Downloads/` and double-clicked, malicious DLLs in the same folder could be loaded. Mitigations:

```rust
// In build.rs — linker flags
#[cfg(windows)]
println!("cargo:rustc-link-arg=/DEPENDENTLOADFLAG:0x800");

// In main() — runtime mitigation
#[cfg(windows)]
unsafe {
    windows_sys::Win32::System::LibraryLoader::SetDefaultDllDirectories(
        LOAD_LIBRARY_SEARCH_SYSTEM32,
    );
}
```

### Console Allocation

The binary uses the console subsystem (default for Rust binaries on Windows). When double-clicked, Windows allocates a console window automatically. No special handling needed.

### Existing Installation Handling

| Scenario | Behavior |
|---|---|
| No existing install | Fresh install |
| Same version installed | Print "already up to date", exit 0 |
| Different version installed | Upgrade to target version |
| Corrupt/partial install (broken junction) | Recreate directory structure |
| Running `vp.exe` in bin/ | Rename to `.old`, copy new (same as trampoline pattern) |

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

### Phase 2: Direct Download URL

Host at `https://vite.plus/vp-setup.exe` with architecture auto-detection (default x64).

Update installation docs:

```
**Windows:**
  Download and run: https://vite.plus/vp-setup.exe
  Or via PowerShell: irm https://vite.plus/ps1 | iex
```

### Phase 3: Package Managers

Submit to winget, chocolatey, scoop. Each has its own manifest format and review process.

## CI/Build Changes

### Release Workflow Additions

```yaml
# In build-rust job matrix (already has windows targets)
- name: Build installer (Windows only)
  if: contains(matrix.settings.target, 'windows')
  run: cargo build --release --target ${{ matrix.settings.target }} -p vite_installer

- name: Upload installer artifact
  if: contains(matrix.settings.target, 'windows')
  uses: actions/upload-artifact@v4
  with:
    name: vite-init-${{ matrix.settings.target }}
    path: ./target/${{ matrix.settings.target }}/release/vp-setup.exe
```

### Test Workflow

Extend `test-standalone-install.yml` with new jobs:

```yaml
test-init-exe:
  strategy:
    matrix:
      shell: [cmd, pwsh, powershell, bash]
  runs-on: windows-latest
  steps:
    - name: Download vp-setup.exe
      run: # download from artifacts or latest release
    - name: Install (silent)
      run: vp-setup.exe -y
    - name: Verify installation
      run: |
        vp --version
        vp --help
```

## Code Signing

Windows Defender SmartScreen flags unsigned executables downloaded from the internet. This is a significant UX problem for a download-and-run installer.

**Recommendation**: Obtain an EV (Extended Validation) code signing certificate before GA release. EV certificates immediately remove SmartScreen warnings (no reputation building period needed).

This is an organizational decision (cost: ~$300-500/year) and out of scope for the implementation, but critical for user experience.

## Binary Size Budget

Target: 3-5 MB (release, stripped, LTO).

Key dependencies and their approximate contribution:

| Dependency | Purpose | Size impact |
|---|---|---|
| `reqwest` + `native-tls-vendored` | HTTP + TLS | ~1.5 MB |
| `flate2` + `tar` | Tarball extraction | ~200 KB |
| `clap` | CLI parsing | ~300 KB |
| `tokio` (minimal features) | Async runtime | ~400 KB |
| `indicatif` | Progress bars | ~100 KB |
| `sha2` | Integrity verification | ~50 KB |
| `serde_json` | Registry JSON parsing | ~200 KB |
| `winreg` | Windows registry | ~50 KB |
| Rust std + overhead | | ~500 KB |

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

### 4. Use `winreg` vs PowerShell for PATH (Decision: `winreg`)

- `winreg` crate: Direct registry API, no subprocess, reliable
- PowerShell subprocess: Proven in `install.ps1` but adds process spawn overhead and PowerShell dependency
- Decision: Use `winreg` for direct registry access — the whole point of the exe installer is to not depend on PowerShell

## Implementation Phases

### Phase 1: Extract `vite_setup` Library

- Create `crates/vite_setup/Cargo.toml`
- Move shared code from `vite_global_cli/src/commands/upgrade/` into `vite_setup`
- Update `vite_global_cli` to import from `vite_setup`
- Run existing tests to verify no regressions

### Phase 2: Create `vite_installer` Binary

- Create `crates/vite_installer/` with `[[bin]] name = "vp-setup"`
- Implement CLI argument parsing (clap)
- Implement installation flow calling `vite_setup`
- Implement Windows PATH modification via `winreg`
- Implement interactive prompts
- Implement progress bar for downloads
- Add DLL security mitigations

### Phase 3: CI Integration

- Add init binary build to release workflow
- Add artifact upload and GitHub Release attachment
- Add test jobs for `vp-setup.exe` across shell types

### Phase 4: Documentation & Distribution

- Update installation docs
- Host on `vite.plus/vp-setup.exe`
- Update release body template with download link

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
- **Minimum Windows version**: Windows 10 1809+ (same as Rust's MSVC target)

## References

- [rustup-init.exe source](https://github.com/rust-lang/rustup/blob/master/src/bin/rustup-init.rs) — single-binary installer model
- [rustup self_update.rs](https://github.com/rust-lang/rustup/blob/master/src/cli/self_update.rs) — installation flow
- [rustup windows.rs](https://github.com/rust-lang/rustup/blob/master/src/cli/self_update/windows.rs) — Windows PATH/registry handling
- [RFC: Windows Trampoline](./trampoline-exe-for-shims.md) — existing Windows .exe shim approach
- [RFC: Self-Update Command](./upgrade-command.md) — existing upgrade logic to share
