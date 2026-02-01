# RFC: `vp env` - Shim-Based Node Version Management

## Summary

This RFC proposes adding a `vp env` command that provides system-wide, IDE-safe Node.js version management through a shim-based architecture. The shims intercept `node`, `npm`, and `npx` commands, automatically resolving and executing the correct Node.js version based on project configuration.

> **Note**: Corepack shim is not included as vite-plus has integrated package manager functionality.

## Motivation

### Current Pain Points

1. **IDE Integration Issues**: GUI-launched IDEs (VS Code, Cursor) often don't see shell-configured Node versions because they inherit PATH from the system environment, not shell rc files.

2. **Version Manager Fragmentation**: Users must choose between nvm, fnm, volta, asdf, or mise - each with different setup requirements and shell integrations.

3. **Inconsistent Behavior**: Terminal-launched vs GUI-launched applications may use different Node versions, causing subtle bugs.

4. **Manual Version Switching**: Users must remember to run `nvm use` or similar when entering projects.

### Proposed Solution

A shim-based approach where:

- `VITE_PLUS_HOME/shims/` directory is added to PATH (system-level for IDE reliability)
- Shims (`node`, `npm`, `npx`) are hardlinks/copies of the `vp` binary
- The binary detects invocation via `argv[0]` and dispatches accordingly
- Version resolution and installation leverage existing `vite_js_runtime` infrastructure

## Command Usage

### Setup Commands

```bash
# Initial setup - creates shims and shows PATH configuration instructions
vp env setup

# Force refresh shims (after vp binary upgrade)
vp env setup --refresh

# Set the global default Node.js version (used when no project version file exists)
vp env default 20.18.0
vp env default lts        # Use latest LTS version
vp env default latest     # Use latest version (not recommended for stability)

# Show current default version
vp env default

# Control shim mode
vp env on             # Enable managed mode (shims always use vite-plus Node.js)
vp env off            # Enable system-first mode (shims prefer system Node.js)
```

### Diagnostic Commands

```bash
# Comprehensive system diagnostics
vp env doctor

# Show which node binary would be executed in current directory
vp env which node
vp env which npm

# Output current environment info as JSON
vp env --current --json
# Output: {"version":"20.18.0","source":".node-version","project_root":"/path/to/project","node_path":"/path/to/node"}

# Print shell snippet for current session (fallback for special environments)
vp env --print
```

### Daily Usage (After Setup)

```bash
# These commands are intercepted by shims automatically
node -v           # Uses project-specific version
npm install       # Uses correct npm for the resolved Node version
npx vitest        # Uses correct npx
```

## Architecture Overview

### Single-Binary Multi-Role Design

The `vp` binary serves dual purposes based on `argv[0]`:

```
argv[0] = "vp"        → Normal CLI mode (vp env, vp build, etc.)
argv[0] = "node"      → Shim mode: resolve version, exec node
argv[0] = "npm"       → Shim mode: resolve version, exec npm
argv[0] = "npx"       → Shim mode: resolve version, exec npx
```

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           SHIM DISPATCH FLOW                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  User runs:  $ node app.js                                                  │
│                  │                                                          │
│                  ▼                                                          │
│  ┌──────────────────────────────┐                                           │
│  │  ~/.vite-plus/shims/node     │  ◄── Hardlink to vp binary                │
│  │  (shim intercepts command)   │                                           │
│  └──────────────┬───────────────┘                                           │
│                 │                                                           │
│                 ▼                                                           │
│  ┌──────────────────────────────┐                                           │
│  │  argv[0] Detection           │                                           │
│  │  "node" → shim mode          │                                           │
│  └──────────────┬───────────────┘                                           │
│                 │                                                           │
│                 ▼                                                           │
│  ┌──────────────────────────────┐     ┌─────────────────────────────┐       │
│  │  Version Resolution          │────▶│  Priority Order:            │       │
│  │  (walk up directory tree)    │     │  1. .node-version           │       │
│  └──────────────┬───────────────┘     │  2. package.json#engines    │       │
│                 │                     │  3. package.json#devEngines │       │
│                 │                     │  4. User default (config)   │       │
│                 │                     │  5. Latest LTS              │       │
│                 ▼                     └─────────────────────────────┘       │
│  ┌──────────────────────────────┐                                           │
│  │  Ensure Node.js installed    │                                           │
│  │  (download if needed)        │                                           │
│  └──────────────┬───────────────┘                                           │
│                 │                                                           │
│                 ▼                                                           │
│  ┌──────────────────────────────┐                                           │
│  │  execve() real node binary   │                                           │
│  │  ~/.vite-plus/.../node       │                                           │
│  └──────────────────────────────┘                                           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                         DIRECTORY STRUCTURE                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ~/.vite-plus/                        (VITE_PLUS_HOME)                      │
│  ├── shims/                                                                 │
│  │   ├── node ──────────────────────┐                                       │
│  │   ├── npm  ──────────────────────┼──▶ Hardlinks to vp binary             │
│  │   └── npx  ──────────────────────┘                                       │
│  ├── config.json                      User settings (default version, etc.) │
│  └── current/bin/vp                   The vp CLI binary                     │
│                                                                             │
│  $VITE_PLUS_HOME/js_runtime/node/     (Node.js installations)               │
│      ├── 20.18.0/bin/node             Installed Node.js versions            │
│      ├── 22.13.0/bin/node                                                   │
│      └── ...                                                                │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                      VERSION RESOLUTION (walk_up=true)                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  /home/user/projects/app/src/         ◄── Current directory                 │
│           │                                                                 │
│           ▼                                                                 │
│  ┌─────────────────────────────────────────────────────────────────┐        │
│  │ Check /home/user/projects/app/src/                              │        │
│  │   ├── .node-version?     ✗ not found                            │        │
│  │   └── package.json?      ✗ not found                            │        │
│  └─────────────────────────────────────────────────────────────────┘        │
│           │ walk up                                                         │
│           ▼                                                                 │
│  ┌─────────────────────────────────────────────────────────────────┐        │
│  │ Check /home/user/projects/app/                                  │        │
│  │   ├── .node-version?     ✗ not found                            │        │
│  │   └── package.json?      ✓ found! engines.node = "^20.0.0"      │        │
│  └─────────────────────────────────────────────────────────────────┘        │
│           │                                                                 │
│           ▼                                                                 │
│  Return: version="^20.0.0", source="engines.node",                          │
│          project_root="/home/user/projects/app"                             │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### VITE_PLUS_HOME Directory Layout

```
VITE_PLUS_HOME/                              # Default: ~/.vite-plus
├── shims/
│   ├── node                          # Hardlink to vp binary (Unix)
│   ├── npm                           # Hardlink to vp binary (Unix)
│   ├── npx                           # Hardlink to vp binary (Unix)
│   ├── node.exe                      # Copy of vp.exe (Windows)
│   ├── npm.cmd                       # Wrapper script (Windows)
│   └── npx.cmd                       # Wrapper script (Windows)
├── cache/
│   └── resolve_cache.json            # LRU cache for version resolution
└── config.json                       # User configuration (default version, etc.)
```

### config.json Format

```json
// ~/.vite-plus/config.json

{
  // Default Node.js version when no project version file is found
  // Set via: vp env default <version>
  "defaultNodeVersion": "20.18.0",

  // Alternatively, use aliases:
  // "defaultNodeVersion": "lts"     // Always use latest LTS
  // "defaultNodeVersion": "latest"  // Always use latest (not recommended)

  // Shim mode: controls how shims resolve tools
  // Set via: vp env on (managed) or vp env off (system_first)
  // - "managed" (default): Shims always use vite-plus managed Node.js
  // - "system_first": Shims prefer system Node.js, fallback to managed if not found
  "shimMode": "managed",
}
```

**Note**: Node.js binaries are stored in VITE_PLUS_HOME:

- Linux/macOS: `~/.vite-plus/js_runtime/node/{version}/`
- Windows: `%USERPROFILE%\.vite-plus\js_runtime\node\{version}\`

## Implementation Architecture

### File Structure

```
crates/vite_global_cli/
├── src/
│   ├── main.rs                       # Entry point with shim detection
│   ├── cli.rs                        # Add Env command
│   ├── shim/
│   │   ├── mod.rs                    # Shim module root
│   │   ├── dispatch.rs               # Main shim dispatch logic
│   │   ├── exec.rs                   # Platform-specific execution
│   │   └── cache.rs                  # Resolution cache
│   └── commands/
│       └── env/
│           ├── mod.rs                # Env command module
│           ├── config.rs             # Configuration and version resolution
│           ├── setup.rs              # setup subcommand implementation
│           ├── doctor.rs             # doctor subcommand implementation
│           ├── which.rs              # which subcommand implementation
│           ├── current.rs            # --current implementation
│           ├── default.rs            # default subcommand implementation
│           ├── on.rs                 # on subcommand implementation
│           └── off.rs                # off subcommand implementation
```

### Shim Dispatch Flow

1. Check `VITE_PLUS_BYPASS` environment variable → bypass to system tool
2. Check shim mode from config:
   - If `system_first`: try system tool first, fallback to managed
   - If `managed`: use vite-plus managed Node.js
3. Resolve version (with mtime-based caching)
4. Ensure Node.js is installed (download if needed)
5. Locate tool binary in the installed Node.js
6. Prepend real node bin dir to PATH for child processes
7. Execute the tool (Unix: `execve`, Windows: spawn)

## Design Decisions

### 1. Single Binary with argv[0] Detection

**Decision**: Use a single `vp` binary that detects shim mode from `argv[0]`.

**Rationale**:

- Simplifies upgrades (update one binary, refresh shims)
- Reduces disk usage vs separate binaries
- Consistent behavior across all tools
- Already proven pattern (used by fnm, volta)

### 2. Hardlinks over Symlinks (Unix)

**Decision**: Use hardlinks for shims on Unix, with fallback to copy.

**Rationale**:

- Hardlinks work across more filesystem types than symlinks
- Symlinks can cause argv[0] to resolve to the target name
- Hardlinks preserve the intended argv[0] value
- Copy fallback for cross-filesystem scenarios

### 3. Wrapper Scripts for Windows npm/npx

**Decision**: Use `.cmd` wrapper scripts for npm/npx on Windows with `VITE_PLUS_SHIM_TOOL` environment variable.

**Rationale**:

- Windows PATH resolution prefers `.cmd` over `.exe` for extensionless commands
- npm is typically invoked as `npm` not `npm.exe`
- `.cmd` wrappers set `VITE_PLUS_SHIM_TOOL` env var and forward to `vp.exe`
- More maintainable than multiple .exe copies - only one binary to update

### 4. execve on Unix, spawn on Windows

**Decision**: Use `execve` (process replacement) on Unix, `spawn` on Windows.

**Rationale**:

- `execve` preserves PID, signals, and process hierarchy on Unix
- Windows doesn't support `execve`-style process replacement
- `spawn` on Windows with proper exit code propagation is standard practice

### 5. Separate VITE_PLUS_HOME from Cache

**Decision**: Keep VITE_PLUS_HOME (shims, config) separate from cache (Node binaries).

**Rationale**:

- Cache uses XDG/platform-standard locations (already implemented)
- VITE_PLUS_HOME needs to be user-accessible for PATH configuration
- Allows clearing cache without breaking shim setup

### 6. mtime-Based Cache Invalidation

**Decision**: Invalidate resolution cache when version file mtime changes.

**Rationale**:

- Fast O(1) validation (stat call)
- No need to re-parse files on every invocation
- Content changes trigger mtime updates
- Simple and reliable

## Error Handling

### No Version File Found (Default Fallback)

When no version file is found, vite-plus uses the configured default version:

```bash
$ node -v
v20.18.0  # Uses user-configured default (set via 'vp env default 20.18.0')

# If no default configured, uses latest LTS
$ node -v
v22.13.0  # Falls back to latest LTS
```

The resolution order is:

1. `.node-version` in current or parent directories
2. `package.json#engines.node` in current or parent directories
3. `package.json#devEngines.runtime` in current or parent directories
4. **User Default**: Configured via `vp env default <version>` (stored in `~/.vite-plus/config.json`)
5. **System Default**: Latest LTS version

### Installation Failure

```bash
$ node -v
vp: Failed to install Node 20.18.0: Network error: connection refused
vp: Check your network connection and try again
vp: Or set VITE_PLUS_BYPASS=1 to use system node
```

### Tool Not Found

```bash
$ npx vitest
vp: Tool 'npx' not found in Node 14.0.0 installation
vp: npx is available in Node 5.2.0+
```

### PATH Misconfiguration

```bash
$ vp env doctor

VP Environment Doctor
=====================

VITE_PLUS_HOME: /Users/user/.vite-plus
  ✓ Directory exists
  ✓ Shims directory exists

PATH Analysis:
  ✗ VP shims not in PATH

  Found 'node' at: /usr/local/bin/node (system)
  Expected: /Users/user/.vite-plus/shims/node

Recommended Fix:
  Add to ~/.zshrc:
    export PATH="/Users/user/.vite-plus/shims:$PATH"

  Then restart your terminal and IDE.
```

## User Experience

### First-Time Setup via Install Script

**Note on Directory Structure:**

- CLI binary: `~/.vite-plus/current/bin/vp` (existing)
- Shims directory: `~/.vite-plus/shims/` (new, for node/npm/npx intercept)

The global CLI installation script (`packages/global/install.sh`) will be updated to:

1. Install the `vp` binary (existing behavior)
2. Run `vp env --setup` to create shims (new)
3. Prompt user: "Would you like to add vite-plus node shims to your PATH? (y/n)" (new)
4. If yes and not already configured, prepend `~/.vite-plus/shims` to shell profile
5. If already configured, skip silently

```bash
$ curl -fsSL https://vite-plus.dev/install.sh | sh

Setting up VITE+(⚡)...

✔ VITE+(⚡) successfully installed!

  Version: 1.2.3
  Location: ~/.vite-plus/current/bin

  ✓ Created shims (node, npm, npx) in ~/.vite-plus/shims

Would you like to add vite-plus node shims to your PATH? (y/n): y
  ✓ Added to ~/.zshrc

Restart your terminal and IDE, then run 'vp env doctor' to verify.
```

**Important**: The shims PATH (`~/.vite-plus/shims`) must be **before** the CLI bin PATH (`~/.vite-plus/current/bin`) if both are configured, so that `node` resolves to the shim first.

### Manual Setup

If user declines or needs to reconfigure:

```bash
$ vp env setup

Setting up vite-plus environment...

Created shims:
  /Users/user/.vite-plus/shims/node
  /Users/user/.vite-plus/shims/npm
  /Users/user/.vite-plus/shims/npx

Add to your shell profile (~/.zshrc, ~/.bashrc, etc.):

  export PATH="/Users/user/.vite-plus/shims:$PATH"

For IDE support (VS Code, Cursor), ensure shims are in system PATH:
  - macOS: Add to ~/.profile or use launchd
  - Linux: Add to ~/.profile for display manager integration
  - Windows: System Properties → Environment Variables → Path

Restart your terminal and IDE, then run 'vp env doctor' to verify.
```

### Doctor Output (Healthy)

```bash
$ vp env doctor

VP Environment Doctor
=====================

VITE_PLUS_HOME: /Users/user/.vite-plus
  ✓ Directory exists
  ✓ Shims directory exists
  ✓ All shims present (node, npm, npx)

Shim Mode:
  Mode: managed
  ✓ Shims always use vite-plus managed Node.js

  Run 'vp env on' to always use managed Node.js
  Run 'vp env off' to prefer system Node.js

PATH Analysis:
  ✓ VP shims first in PATH

  node → /Users/user/.vite-plus/shims/node

Current Directory: /Users/user/projects/my-app
  Version Source: .node-version
  Resolved Version: 20.18.0
  Node Path: /Users/user/.cache/vite-plus/js_runtime/node/20.18.0/bin/node
  ✓ Node binary exists

No conflicts detected.
```

**Doctor Output with System-First Mode:**

```bash
$ vp env doctor

...

Shim Mode:
  Mode: system-first
  ✓ Shims prefer system Node.js, fallback to managed
  System Node.js: /usr/local/bin/node

  Run 'vp env on' to always use managed Node.js
  Run 'vp env off' to prefer system Node.js

...
```

### Default Version Command

```bash
# Show current default version
$ vp env default
Default Node.js version: 20.18.0
  Set via: ~/.vite-plus/config.json

# Set a specific version as default
$ vp env default 22.13.0
✓ Default Node.js version set to 22.13.0

# Set to latest LTS
$ vp env default lts
✓ Default Node.js version set to lts (currently 22.13.0)

# When no default is configured
$ vp env default
No default version configured. Using latest LTS (22.13.0).
  Run 'vp env default <version>' to set a default.
```

### Shim Mode Commands

The shim mode controls how shims resolve tools:

| Mode                | Description                                                   |
| ------------------- | ------------------------------------------------------------- |
| `managed` (default) | Shims always use vite-plus managed Node.js                    |
| `system_first`      | Shims prefer system Node.js, fallback to managed if not found |

```bash
# Enable managed mode (always use vite-plus Node.js)
$ vp env on
✓ Shim mode set to managed.

Shims will now always use vite-plus managed Node.js.
Run 'vp env off' to prefer system Node.js instead.

# Enable system-first mode (prefer system Node.js)
$ vp env off
✓ Shim mode set to system-first.

Shims will now prefer system Node.js, falling back to managed if not found.
Run 'vp env on' to always use vite-plus managed Node.js.

# If already in the requested mode
$ vp env on
Shim mode is already set to managed.
Shims will always use vite-plus managed Node.js.
```

**Use cases for system-first mode (`vp env off`)**:

- When you have a system Node.js that you want to use by default
- When working on projects that don't need vite-plus version management
- When debugging version-related issues by comparing system vs managed Node.js

### Which Command

```bash
$ vp env which node
/Users/user/.cache/vite-plus/js_runtime/node/20.18.0/bin/node

$ vp env which npm
/Users/user/.cache/vite-plus/js_runtime/node/20.18.0/bin/npm
```

### Current Command (JSON)

```bash
$ vp env --current --json
{
  "version": "20.18.0",
  "source": ".node-version",
  "project_root": "/Users/user/projects/my-app",
  "node_path": "/Users/user/.cache/vite-plus/js_runtime/node/20.18.0/bin/node",
  "tool_paths": {
    "node": "/Users/user/.cache/vite-plus/js_runtime/node/20.18.0/bin/node",
    "npm": "/Users/user/.cache/vite-plus/js_runtime/node/20.18.0/bin/npm",
    "npx": "/Users/user/.cache/vite-plus/js_runtime/node/20.18.0/bin/npx"
  }
}
```

## Environment Variables

| Variable               | Description                         | Default        |
| ---------------------- | ----------------------------------- | -------------- |
| `VITE_PLUS_HOME`       | Base directory for shims and config | `~/.vite-plus` |
| `VITE_PLUS_LOG`        | Log level: debug, info, warn, error | `warn`         |
| `VITE_PLUS_DEBUG_SHIM` | Enable extra shim diagnostics       | unset          |
| `VITE_PLUS_BYPASS`     | Bypass shim and use system node     | unset          |

## Windows-Specific Considerations

### Shim Structure

```
VITE_PLUS_HOME\shims\
├── node.exe        # Copy of vp.exe
├── npm.cmd         # Wrapper script
└── npx.cmd         # Wrapper script
```

### Wrapper Script Template (npm.cmd)

```batch
@echo off
setlocal
set "VITE_PLUS_SHIM_TOOL=npm"
"%~dp0node.exe" %*
exit /b %ERRORLEVEL%
```

The `.cmd` wrapper sets `VITE_PLUS_SHIM_TOOL` environment variable before calling `node.exe` (which is a copy of `vp.exe`). The Rust binary checks this env var first before falling back to argv[0] detection.

**Benefits of this approach**:

- Single `vp.exe` binary to update (copied as `node.exe`)
- `.cmd` wrappers are trivial text files
- Clear separation of concerns: `.cmd` sets context, binary does the work

### Windows Installation (install.ps1)

The Windows installer (`install.ps1`) follows the same flow:

1. Download and install `vp.exe`
2. Run `vp env --setup` to create shims
3. Prompt user to add shims to User PATH
4. Update PATH via `[Environment]::SetEnvironmentVariable`

## Testing Strategy

### Unit Tests

- Tool name extraction from argv[0]
- Cache invalidation based on mtime
- PATH manipulation
- Shim mode loading

### Integration Tests

- Shim dispatch with version resolution
- Concurrent installation handling
- Doctor diagnostic output

### Snap Tests

Add snap tests in `packages/global/snap-tests/`:

```
env-setup/
├── package.json
├── steps.json      # [{"command": "vp env setup"}]
└── snap.txt

env-doctor/
├── package.json
├── .node-version   # "20.18.0"
├── steps.json      # [{"command": "vp env doctor"}]
└── snap.txt
```

### CI Matrix

- ubuntu-latest: Full integration tests
- macos-latest: Full integration tests
- windows-latest: Full integration tests with .cmd wrapper validation

## Security Considerations

1. **Path Validation**: Verify executed binaries are under VITE_PLUS_HOME/cache paths
2. **No Path Traversal**: Sanitize version strings before path construction
3. **Atomic Installs**: Use temp directory + rename pattern (already implemented)
4. **Log Sanitization**: Don't log sensitive environment variables

## Implementation Plan

### Phase 1: Core Infrastructure (P0)

1. Add `vp env` command structure to CLI
2. Implement argv[0] detection in main.rs (also check `VITE_PLUS_SHIM_TOOL` env var for Windows)
3. Implement shim dispatch logic for `node`
4. Implement `vp env setup` (Unix hardlinks, Windows .exe copy + .cmd wrappers)
5. Implement `vp env doctor` basic diagnostics
6. Add resolution cache (persists across upgrades with version field)
7. Implement `vp env default [version]` to set/show global default Node.js version
8. Implement `vp env on` and `vp env off` for shim mode control

### Phase 2: Full Tool Support (P1)

1. Add shims for `npm`, `npx`
2. Implement `vp env which`
3. Implement `vp env --current --json`
4. Enhanced doctor with conflict detection

### Phase 3: Polish (P2)

1. Implement `vp env --print` for session-only env
2. Add VITE_PLUS_BYPASS escape hatch
3. Improve error messages
4. Add IDE-specific setup guidance
5. Documentation

## Backward Compatibility

This is a new feature with no impact on existing functionality. The `vp` binary continues to work normally when invoked directly.

## Future Enhancements

1. **Multiple Runtime Support**: Extend shim architecture for other runtimes (Bun, Deno)
2. **SQLite Cache**: Replace JSON cache with SQLite for better performance at scale
3. **Version Pinning**: Allow per-directory version overrides via `vp env pin 20.18.0`
4. **Shell Integration**: Provide shell hooks for prompt version display

## Design Decisions Summary

The following decisions have been made:

1. **VITE_PLUS_HOME Default Location**: `~/.vite-plus` - Simple, memorable path that's easy for users to find and configure.

2. **Windows Wrapper Strategy**: `.cmd` wrappers with `VITE_PLUS_SHIM_TOOL` environment variable - More maintainable, only one binary to update.

3. **Corepack Handling**: Not included - vite-plus has integrated package manager functionality, making corepack shims unnecessary.

4. **Cache Persistence**: Persist across upgrades - Better performance, with cache format versioning for compatibility.

## Conclusion

The `vp env` command provides:

- ✅ System-wide Node version management via shims
- ✅ IDE-safe operation (works with GUI-launched apps)
- ✅ Zero daily friction (automatic version switching)
- ✅ Cross-platform support (Windows, macOS, Linux)
- ✅ Comprehensive diagnostics (`--doctor`)
- ✅ Flexible shim mode control (`on`/`off` for managed vs system-first)
- ✅ Leverages existing version resolution and installation infrastructure
