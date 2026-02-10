# RFC: `vpx` Command

## Summary

Add `vpx` command that runs a command from a local or remote npm package (like `npx`), with local `node_modules/.bin` lookup before falling back to remote download.

The existing `vp dlx` command remains unchanged — it always downloads from the registry without checking local packages (like `pnpm dlx`).

## Motivation

Currently, `vp dlx` always downloads packages from the remote registry, even when the desired binary already exists in `node_modules/.bin`. There is no way to run a locally installed package binary with automatic remote fallback.

Every major package manager provides this capability:

```bash
# npm - checks local, falls back to remote
npx eslint .

# pnpm - local only (no remote fallback)
pnpm exec eslint .

# bun - checks local, falls back to remote
bunx eslint .
```

### Current Pain Points

```bash
# Developer has eslint installed locally, but vp dlx always downloads it again
vp dlx eslint .                     # Downloads from registry (slow, wasteful)

# To run local binary, developer must use full path
./node_modules/.bin/eslint .        # Verbose, not portable

# Or use the underlying package manager
pnpm exec eslint .                  # Defeats the purpose of vp
```

### Proposed Solution

```bash
# Uses local eslint if installed, otherwise downloads
vpx eslint .

# Always downloads from registry (unchanged)
vp dlx eslint .
```

## Command Syntax

```bash
vpx <pkg>[@<version>] [args...]
vpx --package=<pkg>[@<version>] <cmd> [args...]
vpx -c '<cmd> [args...]'
```

All flags must come before positional arguments (like `npx`).

**Options:**

- `--package, -p <name>`: Specifies which package(s) to install if not found locally. Can be specified multiple times.
- `--shell-mode, -c`: Executes the command within a shell environment (`/bin/sh` on UNIX, `cmd.exe` on Windows).
- `--silent, -s`: Suppresses all output except the executed command's output.

### Usage Examples

```bash
# Run locally installed binary (or download if not found)
vpx eslint .

# Run specific version (always remote — version doesn't match local)
vpx typescript@5.5.4 tsc --version

# Separate package and command (when binary name differs from package name)
vpx --package @pnpm/meta-updater meta-updater --help

# Multiple packages
vpx --package yo --package generator-webapp yo webapp

# Shell mode (pipe commands)
vpx -p cowsay -p lolcatjs -c 'echo "hi vp" | cowsay | lolcatjs'

# Silent mode
vpx -s create-vue my-app
```

## Lookup Order

When `vpx` is invoked:

1. **Walk up from cwd** looking for `node_modules/.bin/<cmd>`
   - Check `./node_modules/.bin/<cmd>`
   - Check `../node_modules/.bin/<cmd>`
   - Continue until reaching the filesystem root
2. **Check vp global packages** (installed via `vp install -g`)
   - Uses `BinConfig` for O(1) lookup of which package provides the binary
   - Executes with the Node.js version used at install time
3. **Check system PATH** (excluding vite-plus bin directory)
   - Filters out `~/.vite-plus/bin/` to avoid finding vite-plus shims
   - Finds commands like `git`, `cargo`, etc. without downloading
4. **Fall back to remote download** via `vp dlx` behavior (remote download via detected package manager)

Before executing any found binary, `vpx` prepends all `node_modules/.bin` directories (from cwd upward) to PATH so that sub-processes also resolve local binaries first.

### Special Cases

- When a version is specified (e.g., `vpx eslint@9`), local/global/PATH lookup is skipped — always use remote
- When only a package name is specified without a version (e.g., `vpx eslint`), prefer local if available
- Shell mode (`-c`) skips local/global/PATH lookup and delegates directly to `vp dlx`
- `--package` flag skips local/global/PATH lookup and delegates directly to `vp dlx`

## Relationship Between Commands

| Command  | Local lookup | Global lookup | PATH lookup | Remote download | Use case                                          |
| -------- | ------------ | ------------- | ----------- | --------------- | ------------------------------------------------- |
| `vpx`    | Yes (1st)    | Yes (2nd)     | Yes (3rd)   | Yes (fallback)  | Run local, global, PATH, or remote package binary |
| `vp dlx` | No           | No            | No          | Always          | Always fetch latest from registry                 |

### When to use which

- **`vpx eslint .`** — "Run eslint, preferring my local version"
- **`vp dlx create-vue my-app`** — "Download and run create-vue from the registry"
- **`vpx create-vue my-app`** — Same as `vp dlx` in practice, since `create-vue` is never installed locally

## Binary Implementation

### Symlink Approach

`vpx` is delivered as a symlink to `vp`, detected via `argv[0]`:

```
~/.vite-plus/bin/vpx → ~/.vite-plus/bin/vp   (symlink)
```

This follows the same pattern already used for `node`, `npm`, and `npx` shims.

### Detection

In `shim/mod.rs`, when `argv[0]` resolves to `vpx`:

```rust
let argv0_tool = extract_tool_name(argv0);
if argv0_tool == "vpx" {
    // Handle vpx: local lookup + dlx fallback
    return Some("vpx".to_string());
}
```

In `shim/dispatch.rs`, handle the `vpx` tool:

```rust
if tool == "vpx" {
    // 1. Parse vpx flags (--package, -c, -s) and positional args
    // 2. Try local node_modules/.bin lookup
    // 3. If not found, delegate to DlxCommand
    return handle_vpx(remaining_args, cwd).await;
}
```

### Windows

On Windows, `vpx.cmd` is a wrapper script (consistent with existing `node.cmd`, `npm.cmd`, `npx.cmd` wrappers):

```cmd
@echo off
set "VITE_PLUS_SHIM_TOOL=vpx"
"%~dp0..\current\bin\vp.exe" %*
```

### Setup

The `vp env setup` command creates the `vpx` symlink/wrapper alongside existing shims:

```
~/.vite-plus/bin/
├── vp          → ../current/bin/vp
├── vpx         → vp                   ← NEW
├── node        → vp
├── npm         → vp
└── npx         → vp
```

## Comparison with npx

| Behavior            | `npx`                                      | `vpx`                                      |
| ------------------- | ------------------------------------------ | ------------------------------------------ |
| Local lookup        | Walk up `node_modules/.bin`                | Walk up `node_modules/.bin`                |
| Remote fallback     | Download to npm cache                      | Delegate to `vp dlx` (uses detected PM)    |
| Confirmation prompt | Prompts before installing unknown packages | Auto-confirms (like `vp dlx` with `--yes`) |
| `--package` flag    | Specifies additional packages              | Same                                       |
| Shell mode (`-c`)   | Runs in shell with packages in PATH        | Same                                       |
| Cache               | npm cache                                  | Package manager's cache (via `vp dlx`)     |

### Key Difference: Auto-confirm

`npx` prompts the user before downloading unknown packages. `vpx` always auto-confirms (aligns with `vp dlx` behavior and pnpm's approach). This avoids inconsistent behavior across package managers.

## Design Decisions

### 1. Why Walk Up Directories

**Decision**: Walk up from cwd to project root looking for `node_modules/.bin`, like `npx`.

**Rationale**:

- In monorepos, a command may be installed at the workspace root, not the current package
- `npx` walks up directories — matching this behavior meets developer expectations
- `pnpm exec` only looks in `./node_modules/.bin` — too restrictive for monorepos

### 2. Why `vpx` is Separate from `vp dlx`

**Decision**: Keep `vpx` (local-first) and `vp dlx` (remote-only) as separate commands.

**Rationale**:

- Different mental models: "run what I have" vs "download and run"
- `vp dlx` already exists with well-defined remote-only behavior — changing it would break expectations
- Explicit is better than implicit — developers should choose their intent

### 3. Why `vpx` is a Symlink

**Decision**: `vpx` is a symlink to `vp`, not a separate binary.

**Rationale**:

- Zero additional binary size
- Same pattern used for `node`/`npm`/`npx` shims — proven approach
- `argv[0]` detection is already implemented in `shim/mod.rs`
- Single binary to update when upgrading

### 4. Why Not Add `vp exec` Subcommand

**Decision**: Only provide `vpx` as a standalone command, no `vp exec` subcommand for now.

**Rationale**:

- `vpx` covers the primary use case — quick execution of local/remote binaries
- Adding `vp exec` introduces complexity (argument parsing with `--` separator, potential confusion with `vp env exec`)
- `vp exec` can be added later as a follow-up if needed
- Keeps the initial implementation simple and focused

## Edge Cases

### Monorepo Sub-packages

When running `vpx eslint` from `packages/app/`:

```
monorepo/
├── node_modules/.bin/eslint    ← found here (workspace root)
├── packages/
│   └── app/
│       └── node_modules/.bin/  ← checked first (empty)
└── package.json
```

The walker stops at the workspace root (detected by the presence of a workspace-defining `package.json`).

### Native vs JS Binaries

Both native (compiled) and JS binaries in `node_modules/.bin` are supported. The lookup only checks for file existence and executability, not file type.

### Platform Differences

- **Unix**: `node_modules/.bin/<cmd>` is typically a symlink to the package's bin script
- **Windows**: `node_modules/.bin/<cmd>.cmd` wrapper scripts — lookup checks for `.cmd` extension

### Version Mismatch

```bash
# Local eslint is v8, but user wants v9
vpx eslint@9 .
# → Version specified, so local lookup is skipped → delegates to vp dlx
```

When a version is explicitly specified in the package spec, the command skips local lookup and always uses remote download.

## Implementation Architecture

### 1. Shim Detection

**File**: `crates/vite_global_cli/src/shim/mod.rs`

Add `vpx` recognition to `detect_shim_tool()`:

```rust
let argv0_tool = extract_tool_name(argv0);
if argv0_tool == "vp" {
    return None;
}
if argv0_tool == "vpx" {
    return Some("vpx".to_string());
}
```

### 2. Dispatch Handler

**File**: `crates/vite_global_cli/src/shim/dispatch.rs`

Handle `vpx` in the dispatch logic (delegates to `commands/vpx.rs`):

```rust
if tool == "vpx" {
    return crate::commands::vpx::execute_vpx(args, &cwd).await;
}
```

The dispatch module also exposes helper functions as `pub(crate)` for vpx to reuse:

- `find_package_for_binary()` — looks up which globally installed package provides a binary
- `locate_package_binary()` — locates the actual binary path inside a package
- `ensure_installed()` — ensures a Node.js version is downloaded
- `locate_tool()` — locates a tool binary within a Node.js installation

### 3. Binary Resolution (`commands/vpx.rs`)

**File**: `crates/vite_global_cli/src/commands/vpx.rs`

Resolution order (when no version spec, no --package flag, and not shell mode):

```rust
// 1. Local node_modules/.bin — walk up from cwd
if let Some(local_bin) = find_local_binary(cwd, &cmd_name) { ... }

// 2. Global vp packages — uses dispatch::find_package_for_binary()
if let Some(global_bin) = find_global_binary(&cmd_name).await { ... }

// 3. System PATH — uses which::which_in() with filtered PATH
if let Some(path_bin) = find_on_path(&cmd_name) { ... }

// 4. Remote download — delegates to DlxCommand
```

Before executing any found binary, `prepend_node_modules_bin_to_path()` walks up from cwd and prepends all existing `node_modules/.bin` directories to PATH.

### 4. Setup

**File**: `crates/vite_global_cli/src/commands/env/setup.rs`

Add `vpx` to the shim creation:

```rust
// After creating vp symlink, also create vpx
create_symlink(&bin_dir.join("vpx"), &bin_dir.join("vp")).await?;
```

### 5. Reuses Existing `DlxCommand`

The remote fallback path delegates entirely to the existing `DlxCommand`, which handles package manager detection, command resolution, and execution. No changes needed to `vp dlx` behavior.

## CLI Help Output

```bash
$ vpx --help
Execute a command from a local or remote npm package

Usage: vpx [OPTIONS] <pkg[@version]> [args...]

Arguments:
  <pkg[@version]>  Package binary to execute
  [args...]        Arguments to pass to the command

Options:
  -p, --package <NAME>  Package(s) to install if not found locally
  -c, --shell-mode      Execute the command within a shell environment
  -s, --silent          Suppress all output except the command's output
  -h, --help            Print help

Examples:
  vpx eslint .                                           # Run local eslint (or download)
  vpx create-vue my-app                                  # Download and run create-vue
  vpx typescript@5.5.4 tsc --version                     # Run specific version
  vpx -p cowsay -c 'echo "hi" | cowsay'                  # Shell mode with package
```

## Error Handling

### Missing Command

```bash
$ vpx
Error: vpx requires a command to run

Usage: vpx <pkg[@version]> [args...]

Examples:
  vpx eslint .
  vpx create-vue my-app
```

### Local Binary Not Found (Falls Back to Remote)

```bash
$ vpx some-tool --version
# No local binary found, downloading via vp dlx...
Running: pnpm dlx some-tool --version
some-tool v1.2.3
```

### Remote Package Not Found

```bash
$ vpx non-existent-package-xyz
# No local binary found, downloading via vp dlx...
Running: pnpm dlx non-existent-package-xyz
 ERR_PNPM_NO_IMPORTER_MANIFEST_FOUND  No package.json was found
Exit code: 1
```

## Security Considerations

1. **Local-first is safer**: `vpx` prefers local binaries, reducing the risk of running unexpected remote code for packages that are already project dependencies.

2. **Auto-confirm for remote**: When falling back to remote download, `vpx` auto-confirms (like `vp dlx`). This means unknown packages are downloaded without prompting — consistent with `vp dlx` behavior.

3. **Version pinning**: Specifying an explicit version (e.g., `vpx eslint@9`) bypasses local lookup and always downloads from the registry, ensuring the exact requested version is used.

## Backward Compatibility

This is a new feature with no breaking changes:

- `vp dlx` behavior is completely unchanged
- `vpx` binary is a new symlink created by `vp env setup`
- Existing `node`/`npm`/`npx` shims are unaffected
- No changes to configuration format

## Implementation Plan

### Phase 1: Local Binary Lookup

1. Implement `find_local_binary()` — walk up directories checking `node_modules/.bin`
2. Implement `has_version_spec()` — detect version in package spec
3. Implement `run_local_binary()` — execute the found binary with args

### Phase 2: `vpx` Dispatch

1. Add `vpx` detection in `shim/mod.rs` (`detect_shim_tool`)
2. Add `vpx` flag parsing and dispatch in `shim/dispatch.rs`
3. Create `commands/vpx.rs` with local lookup + dlx fallback logic

### Phase 3: Setup

1. Add `vpx` symlink/wrapper creation in `commands/env/setup.rs`

### Phase 4: Testing

1. Unit tests for local binary lookup (walk-up behavior)
2. Unit tests for version spec detection
3. Unit tests for `vpx` flag parsing
4. Integration tests with mock `node_modules/.bin` directories
5. Snap tests for `vpx` CLI output

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_find_local_binary_in_cwd() {
    // Create temp dir with node_modules/.bin/eslint
    // Assert find_local_binary returns the path
}

#[test]
fn test_find_local_binary_walks_up() {
    // Create temp dir structure:
    //   root/node_modules/.bin/eslint
    //   root/packages/app/
    // Run from root/packages/app/
    // Assert find_local_binary returns root's binary
}

#[test]
fn test_find_local_binary_not_found() {
    // Create temp dir with no node_modules
    // Assert find_local_binary returns None
}

#[test]
fn test_has_version_spec() {
    assert!(!has_version_spec("eslint"));
    assert!(has_version_spec("eslint@9"));
    assert!(has_version_spec("typescript@5.5.4"));
    assert!(!has_version_spec("@vue/cli")); // scoped package, not version
    assert!(has_version_spec("@vue/cli@5.0.0")); // scoped with version
}
```

## Future Enhancements

### 1. `vp exec` Subcommand

Add `vp exec` as an alternative way to invoke `vpx` from within `vp`, using `--` separator for argument parsing (like `npm exec`).

### 2. Workspace-aware Lookup

```bash
vpx --workspace=app eslint .    # Look in app's node_modules first
```

### 3. Local-only / Remote-only Modes

```bash
vpx --prefer-local eslint .     # Only use local, never download
vpx --prefer-remote eslint .    # Always download, ignore local
```

## Conclusion

This RFC proposes adding `vpx` to complete the package execution story in Vite+:

- `vp dlx` — always remote (like `pnpm dlx`)
- `vpx` — local-first with remote fallback (like `npx`)

The design:

- Follows established `npx` conventions for familiar developer experience
- Reuses existing `vp dlx` infrastructure for the remote fallback path
- Uses the proven symlink + `argv[0]` detection pattern for delivery
- Maintains clear separation between local-first (`vpx`) and remote-only (`vp dlx`)
- Is purely additive with no breaking changes to existing behavior
