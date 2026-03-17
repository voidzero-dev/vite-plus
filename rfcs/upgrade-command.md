# RFC: Self-Update Command

## Status

Draft

## Background

Vite+ is distributed as a standalone Rust binary via bash installation (`curl -fsSL https://vite.plus | bash`). Currently, users must re-run the full install script to update to a new version. This is friction-heavy and unfamiliar to users who expect a built-in update mechanism (like `rustup update`, `volta fetch`, or `brew upgrade`).

A native `vp upgrade` command would allow users to update the CLI in-place with a single command, improving the upgrade experience significantly.

### Current Installation Structure

```
~/.vite-plus/
├── bin/
│   ├── vp → ../current/bin/vp       # Stable symlink (in PATH)
│   ├── node → ../current/bin/node   # Shim symlinks
│   ├── npm → ../current/bin/npm
│   └── npx → ../current/bin/npx
├── current → 0.1.0/                 # Symlink to active version
├── 0.1.0/                           # Version directory
│   ├── bin/vp                       # Actual binary
│   ├── dist/                        # JS bundles + .node files
│   ├── package.json
│   └── node_modules/
├── 0.0.9/                           # Previous version (kept for rollback)
├── env                              # POSIX shell env (sourced by shell config)
├── env.fish                         # Fish shell env
└── env.ps1                          # PowerShell env
```

Key invariant: `~/.vite-plus/bin/vp` is a symlink to `../current/bin/vp` (Unix) or a trampoline `.exe` forwarding to `current\bin\vp.exe` (Windows), and `current` is a symlink (Unix) or junction (Windows) to the active version directory. Upgrading swaps the `current` link — atomic on Unix, near-instant on Windows.

## Goals

1. Provide a fast, reliable `vp upgrade` command that upgrades the CLI to the latest (or specified) version
2. Reuse the same npm-based distribution channel (no new infrastructure)
3. Support atomic upgrades with automatic rollback on failure
4. Keep the last 5 versions for manual rollback
5. Support version pinning and channel selection (latest, test)

## Non-Goals

1. Auto-update on every command invocation (may be a future enhancement)
2. Windows PowerShell install path (covered by `install.ps1`)
3. Migrating away from npm as the distribution channel
4. Updating Node.js versions (already handled by `vp env`)

## User Stories

### Story 1: Quick Update to Latest

A developer sees that a new version of Vite+ is available and wants to update.

```bash
$ vp upgrade
info: checking for updates...
info: found vite-plus-cli@0.2.0 (current: 0.1.0)
info: downloading vite-plus-cli@0.2.0 for darwin-arm64...
info: installing...

✔ Updated vite-plus from 0.1.0 → 0.2.0

  Release notes: https://github.com/voidzero-dev/vite-plus/releases/tag/v0.2.0
```

### Story 2: Already Up to Date

```bash
$ vp upgrade
info: checking for updates...

✔ Already up to date (0.2.0)
```

### Story 3: Update to a Specific Version

```bash
$ vp upgrade 0.1.5
info: checking for updates...
info: found vite-plus-cli@0.1.5 (current: 0.2.0)
info: downloading vite-plus-cli@0.1.5 for darwin-arm64...
info: installing...

✔ Updated vite-plus from 0.2.0 → 0.1.5
```

### Story 4: Install a Test Channel Build

```bash
$ vp upgrade --tag test
info: checking for updates...
info: found vite-plus-cli@0.3.0-beta.1 (current: 0.2.0)
info: downloading vite-plus-cli@0.3.0-beta.1 for darwin-arm64...
info: installing...

✔ Updated vite-plus from 0.2.0 → 0.3.0-beta.1
```

### Story 5: Rollback to Previous Version

```bash
$ vp upgrade --rollback
info: rolling back to previous version...
info: switching from 0.2.0 → 0.1.0

✔ Rolled back to 0.1.0
```

### Story 6: Check for Updates Without Installing

```bash
$ vp upgrade --check
info: checking for updates...
Update available: 0.2.0 → 0.3.0
Run `vp upgrade` to update.
```

### Story 7: CI Environment — Non-interactive

```bash
# In CI, just update silently
$ vp upgrade --silent
```

## Technical Design

### Command Interface

```
vp upgrade [VERSION] [OPTIONS]
vp upgrade [VERSION] [OPTIONS]       # alias

Arguments:
  [VERSION]    Target version (e.g., "0.2.0"). Defaults to "latest"

Options:
  --tag <TAG>      npm dist-tag to install (default: "latest", also: "test")
  --check          Check for updates without installing
  --rollback       Revert to the previously active version
  --force          Force reinstall even if already on the target version
  --silent         Suppress output (useful in CI)
  --registry <URL> Custom npm registry URL (overrides NPM_CONFIG_REGISTRY)
```

### Architecture

The upgrade command is implemented entirely in Rust within the `vite_global_cli` crate, mirroring the logic of `install.sh` but running as a native subprocess workflow.

```
┌─────────────────────────────────────────────────┐
│                vp upgrade                   │
├─────────────────────────────────────────────────┤
│  1. Resolve version (npm registry query)        │
│  2. Check if already installed                  │
│  3. Download platform binary (.tgz)             │
│  4. Download main JS bundle (.tgz)              │
│  5. Extract to ~/.vite-plus/{version}/          │
│  6. Install production dependencies             │
│  7. Atomic swap: current → {version}            │
│  8. Refresh shims (non-fatal)                   │
│  9. Cleanup old versions (non-fatal, keep 5)    │
└─────────────────────────────────────────────────┘
```

### Implementation Flow

#### Step 1: Version Resolution

Query the npm registry for the target version:

```
GET {registry}/vite-plus-cli/{version_or_tag}
```

- If `VERSION` arg is provided, use it directly
- If `--tag` is provided, resolve that dist-tag (e.g., `latest`, `test`)
- Default to `latest`

Parse the JSON response to extract:

- `version`: the resolved semver version
- `optionalDependencies`: to find the platform-specific package name

#### Step 2: Version Comparison

Compare the resolved version against the currently running binary's version (`env!("CARGO_PKG_VERSION")`).

- If same version and `--force` is not set: print "already up to date" and exit
- If target is older: proceed (allows deliberate downgrade)

#### Step 3: Download and Verify

Download two tarballs from the npm registry:

1. **Platform binary**: `{registry}/@voidzero-dev/vite-plus-cli-{platform_suffix}/-/vite-plus-cli-{suffix}-{version}.tgz`
   - Contains: `vp` binary + `.node` NAPI files
2. **Main package**: `{registry}/vite-plus-cli/-/vite-plus-cli-{version}.tgz`
   - Contains: `dist/` (JS bundles), `package.json`, `templates/`, `rules/`, `AGENTS.md`

**Integrity verification**: Each tarball is verified against the `integrity` field from the npm registry metadata. The npm registry provides SHA-512 hashes in the [Subresource Integrity](https://w3c.github.io/webappsec-subresource-integrity/) format:

```json
{
  "dist": {
    "tarball": "https://registry.npmjs.org/vite-plus-cli/-/vite-plus-cli-0.0.0-xxx.tgz",
    "integrity": "sha512-Z3se9k/NTRf8s5eSmuSoMOFFB/TUGBHIoeWDU5VoHV...",
    "shasum": "3399579218148ae410011bde8934e12209743ef3"
  }
}
```

Verification flow:

1. Download tarball to temp file
2. Compute SHA-512 hash of the downloaded file
3. Base64-encode and compare against `integrity` field (format: `sha512-{base64}`)
4. If mismatch: delete temp file, report error, abort update

```rust
use sha2::{Sha512, Digest};
use base64::{Engine as _, engine::general_purpose::STANDARD};

fn verify_integrity(data: &[u8], expected: &str) -> Result<(), Error> {
    // Parse "sha512-{base64}" format
    let expected_hash = expected.strip_prefix("sha512-")
        .ok_or(Error::UnsupportedIntegrity(expected.into()))?;

    let mut hasher = Sha512::new();
    hasher.update(data);
    let actual_hash = STANDARD.encode(hasher.finalize());

    if actual_hash != expected_hash {
        return Err(Error::IntegrityMismatch {
            expected: expected.into(),
            actual: format!("sha512-{}", actual_hash),
        });
    }
    Ok(())
}
```

To get the `integrity` field for the platform package, we need to query its metadata separately:

- Main package metadata: `{registry}/vite-plus-cli/{version}` → contains `dist.integrity`
- Platform package metadata: `{registry}/@voidzero-dev/vite-plus-cli-{suffix}/{version}` → contains `dist.integrity`

Platform detection reuses existing logic from `vite_js_runtime` or mirrors the bash script's approach:

- `uname -s` → os (darwin, linux)
- `uname -m` → arch (x64, arm64)
- Linux: detect gnu vs musl libc

#### Step 4: Extract and Install

1. Create `~/.vite-plus/{version}/` with `bin/` and `dist/` subdirectories
2. Extract platform binary to `{version}/bin/vp`, set executable permissions
3. Extract `.node` files to `{version}/dist/`
4. Extract JS bundle, templates, rules, package.json to `{version}/`
5. Strip `devDependencies` and `optionalDependencies` from package.json
6. Run `vp install --silent` in the version directory to install production dependencies

#### Step 5: Version Swap

**Unix (macOS/Linux)** — Atomic symlink swap:

```rust
// Atomic symlink swap using rename
let temp_link = install_dir.join("current.new");
std::os::unix::fs::symlink(version, &temp_link)?;
std::fs::rename(&temp_link, install_dir.join("current"))?;
```

This is atomic on POSIX systems because `rename()` on a symlink is an atomic operation.

**Windows** — Junction swap (non-atomic, matching `install.ps1`):

```rust
// Windows uses junctions (mklink /J) — no admin privileges required
let current_link = install_dir.join("current");

// Remove existing junction
if current_link.exists() {
    junction::delete(&current_link)?;
}

// Create new junction pointing to version directory
junction::create(version_dir, &current_link)?;
```

Key differences on Windows:

- **Junctions** (`mklink /J`) are used instead of symlinks — junctions don't require admin privileges
- Junctions only work for directories (which `current` is), and use absolute paths internally
- The swap is **not atomic** — there's a brief window (~milliseconds) where `current` doesn't exist
- `bin/vp.exe` is a trampoline (not a symlink) that resolves through `current`, so it doesn't need updating during upgrade
- This matches the existing `install.ps1` behavior exactly

#### Step 6: Post-Update (Non-Fatal)

After the symlink swap (the **point of no return**), post-update operations are treated as non-fatal. Errors are printed to stderr as warnings but do not trigger the outer error handler (which would delete the now-active version directory).

1. **Refresh shims**: Run the equivalent of `vp env setup --refresh` to ensure node/npm/npx shims point to the new version. If this fails, the user can run it manually.
2. **Cleanup old versions**: Remove old version directories, keeping the 5 most recent by **creation time** (matching `install.sh` behavior). The new version and the previous version are always protected from cleanup, even if they fall outside the top 5 (e.g., after a downgrade via `--rollback`).

#### Step 7: Running Binary Consideration

The running `vp` process is **not** the binary being replaced. The flow is:

```
# Unix
~/.vite-plus/bin/vp  →  ../current/bin/vp  →  {old_version}/bin/vp

# Windows
~/.vite-plus/bin/vp.exe (trampoline)  →  current\bin\vp.exe  →  {old_version}\bin\vp.exe
```

After the `current` link swap, any **new** invocation of `vp` will use the new binary. The currently running process continues to execute from the old version's binary file on disk:

- **Unix**: The old binary remains valid because Unix doesn't delete open files until all file descriptors are closed
- **Windows**: The old `.exe` file is locked while running, but since we install to a **new version directory** (not overwriting in-place), there's no conflict. The old version directory is preserved (kept in the "last 5" cleanup policy)

### Rollback Design

The `--rollback` flag switches the `current` symlink to the previously active version.

To track the previous version, we can:

1. Read the `current` symlink target before updating
2. After the update, write the previous version to `~/.vite-plus/.previous-version`

For `--rollback`:

1. Read `~/.vite-plus/.previous-version`
2. Verify that version directory still exists
3. Swap `current` symlink to point to it
4. Update `.previous-version` to point to the version we just rolled back from

### Error Handling

| Error                           | Recovery                                                      |
| ------------------------------- | ------------------------------------------------------------- |
| Network failure during download | Clean up partial temp files, exit with helpful message        |
| Integrity mismatch (SHA-512)    | Delete downloaded file, report expected vs actual hash, abort |
| Corrupted tarball               | Verify extraction success, clean up version dir if partial    |
| `vp install` fails              | Remove the version dir, keep current version unchanged        |
| Disk full                       | Detect and report, clean up partial state                     |
| Permission denied               | Report with suggestion to check directory ownership           |
| Registry returns error          | Parse npm error JSON, show human-readable message             |

Key principle: **The `current` symlink is only swapped after all pre-swap steps succeed.** If any pre-swap step fails, the existing installation is untouched. Post-swap operations (shim refresh, old version cleanup) are non-fatal — their errors are printed to stderr as warnings but do not roll back the update.

### File Structure

```
crates/vite_global_cli/
├── src/
│   ├── commands/
│   │   ├── upgrade/
│   │   │   ├── mod.rs        # Module root, public execute() function
│   │   │   ├── registry.rs   # npm registry client (version resolution, tarball URLs)
│   │   │   ├── platform.rs   # Platform detection (os, arch, libc)
│   │   │   ├── download.rs   # HTTP download + tarball extraction
│   │   │   └── install.rs    # Extract, dependency install, symlink swap, cleanup
│   │   ├── mod.rs            # Add upgrade module
│   │   └── ...
│   └── cli.rs                # Add Upgrade command variant
```

### Platform Detection

```rust
fn detect_platform() -> Result<String, Error> {
    let os = std::env::consts::OS;       // "macos", "linux", "windows"
    let arch = std::env::consts::ARCH;   // "x86_64", "aarch64"

    let os_name = match os {
        "macos" => "darwin",
        "linux" => "linux",
        "windows" => "win32",
        _ => return Err(Error::UnsupportedPlatform(os.into())),
    };

    let arch_name = match arch {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        _ => return Err(Error::UnsupportedArch(arch.into())),
    };

    if os_name == "linux" {
        let libc = detect_libc(); // "gnu" or "musl"
        Ok(format!("{os_name}-{arch_name}-{libc}"))
    } else if os_name == "win32" {
        Ok(format!("{os_name}-{arch_name}-msvc"))
    } else {
        Ok(format!("{os_name}-{arch_name}"))
    }
}
```

### Registry Client

Uses `reqwest` (already a dependency via `vite_js_runtime`) for HTTP requests:

```rust
async fn resolve_version(registry: &str, version_or_tag: &str) -> Result<PackageMetadata, Error> {
    let url = format!("{}/vite-plus-cli/{}", registry, version_or_tag);
    let response = reqwest::get(&url).await?.json::<PackageMetadata>().await?;
    Ok(response)
}
```

### CLI Integration

Add `Upgrade` to the `Commands` enum in `cli.rs`:

```rust
/// Update vp itself to the latest version
#[command(name = "upgrade", visible_alias = "upgrade")]
Upgrade {
    /// Target version (default: latest)
    version: Option<String>,

    /// npm dist-tag (default: "latest")
    #[arg(long, default_value = "latest")]
    tag: String,

    /// Check for updates without installing
    #[arg(long)]
    check: bool,

    /// Revert to previous version
    #[arg(long)]
    rollback: bool,

    /// Force reinstall even if up to date
    #[arg(long)]
    force: bool,

    /// Suppress output
    #[arg(long)]
    silent: bool,

    /// Custom npm registry URL
    #[arg(long)]
    registry: Option<String>,
},
```

## Design Decisions

### 1. Command Name: `upgrade`

**Decision**: Use `vp upgrade` (with hyphen).

**Alternatives considered**:

- `vp upgrade` — used by Deno, Bun, proto; shorter but ambiguous with `vp update` (packages)
- `vp self upgrade` — used by rustup (`rustup self update`); requires subcommand group

**Rationale**:

- Matches pnpm (`pnpm upgrade`) and mise (`mise upgrade`) conventions
- Zero ambiguity with `vp update` (which updates npm packages)
- The hyphen is consistent with `list-remote` in `vp env`
- Tools without upgrade (fnm, volta, nvm) require re-running install scripts — worse UX
- `upgrade` is registered as a visible alias, so `vp upgrade` also works (matches Deno/Bun/proto users' expectations)

### 2. Pure Rust Implementation (No Shell Script Re-execution)

**Decision**: Implement the update logic entirely in Rust.

**Rationale**:

- No dependency on bash or curl being installed
- Better error handling and progress reporting
- Consistent behavior across platforms
- The install.sh script remains for first-time installation only

### 3. Reuse npm Distribution Channel

**Decision**: Download tarballs from the same npm registry used by `install.sh`.

**Rationale**:

- No new infrastructure needed
- Same release pipeline, same artifacts
- Supports custom registries and mirrors via `--registry` or `NPM_CONFIG_REGISTRY`
- Users behind corporate proxies already have npm registry access configured

### 4. No Automatic Update Checks

**Decision**: Do not check for updates on every `vp` invocation.

**Rationale**:

- Avoids unexpected network requests that slow down commands
- Avoids privacy concerns (phoning home on every run)
- Users can opt into periodic checks via their own cron/launchd if desired
- This can be revisited as a future enhancement with proper opt-in

### 5. Keep 5 Versions for Rollback

**Decision**: Maintain the same cleanup policy as `install.sh` (keep 5 most recent versions by creation time, with protected versions).

**Rationale**:

- Consistent with existing `install.sh` behavior (sorts by creation time, not semver)
- Provides rollback safety net without unbounded disk usage
- Each version is ~20-30MB, so 5 versions is ~100-150MB total
- The active version and previous version are always protected from cleanup, preventing accidental deletion after a downgrade

## Implementation Phases

### Phase 0 (P0): Core Self-Update

**Scope:**

- `vp upgrade` — downloads and installs the latest version
- `vp upgrade <version>` — installs a specific version
- `--tag`, `--force`, `--silent` flags
- Platform detection, npm registry query, download, extract, symlink swap
- Version cleanup (keep 5)
- Error handling with clean rollback

**Files to create/modify:**

- `crates/vite_global_cli/src/commands/upgrade/mod.rs` (new)
- `crates/vite_global_cli/src/commands/upgrade/registry.rs` (new)
- `crates/vite_global_cli/src/commands/upgrade/platform.rs` (new)
- `crates/vite_global_cli/src/commands/upgrade/download.rs` (new)
- `crates/vite_global_cli/src/commands/upgrade/install.rs` (new)
- `crates/vite_global_cli/src/commands/mod.rs` (add module)
- `crates/vite_global_cli/src/cli.rs` (add command variant + routing)

**Success Criteria:**

- [ ] `vp upgrade` downloads and installs the latest version
- [ ] `vp upgrade 0.x.y` installs a specific version
- [ ] Downloaded tarballs are verified against npm registry `integrity` (SHA-512)
- [ ] Running binary is not affected during update
- [ ] Failed update leaves the current installation untouched
- [ ] Old versions are cleaned up (max 5 retained)
- [ ] Works on macOS, Linux, and Windows

### Phase 1 (P1): Rollback and Check

**Scope:**

- `--rollback` flag with `.previous-version` tracking
- `--check` flag for update availability check

**Success Criteria:**

- [ ] `vp upgrade --rollback` reverts to previous version
- [ ] `vp upgrade --check` shows available update without installing

### Phase 2 (P2): Enhanced UX

**Scope:**

- Progress bar for downloads (using `indicatif` or similar)
- Release notes URL in update success message
- `--registry` flag for custom npm registry

**Success Criteria:**

- [ ] Download progress is visible for large binaries
- [ ] Release notes link is shown after successful update

## Testing Strategy

### Unit Tests

- Version comparison logic (semver parsing, equality, ordering)
- Platform detection (mock `std::env::consts`)
- Registry URL construction
- Symlink swap atomicity

### Integration Tests

- Download and extract a real package from the test npm tag
- Verify version directory structure after install
- Verify `current` symlink points to new version
- Verify old version cleanup

### Snap Tests

```bash
# Test: upgrade check (mock registry response)
pnpm -F vite-plus-cli snap-test upgrade-check

# Test: upgrade to specific version
pnpm -F vite-plus-cli snap-test upgrade-version
```

### Manual Testing

```bash
# Build and install current version
pnpm bootstrap-cli

# Run upgrade to latest published version
vp upgrade

# Verify version changed
vp -V

# Test rollback
vp upgrade --rollback
vp -V
```

## Future Enhancements

- **Automatic update check**: Periodic background check with opt-in notification (e.g., once per day, cached result)
- **Update channels**: Allow pinning to a channel (stable, beta, nightly) via config file
- **Delta updates**: Download only changed files instead of full tarballs
- **Windows support**: Extend to PowerShell-based update mechanism for Windows native installs

## References

- [RFC: Global CLI (Rust Binary)](./global-cli-rust-binary.md)
- [RFC: Split Global CLI](./split-global-cli.md)
- [RFC: Env Command](./env-command.md)
- [Install Script](../packages/cli/install.sh)
- [Release Workflow](../.github/workflows/release.yml)
