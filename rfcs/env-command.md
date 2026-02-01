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

### Version Management Commands

```bash
# Pin a specific version in current directory (creates .node-version)
vp env pin 20.18.0

# Pin using version aliases (resolved to exact version)
vp env pin lts        # Resolves and pins current LTS (e.g., 22.13.0)
vp env pin latest     # Resolves and pins latest version

# Pin using semver ranges
vp env pin "^20.0.0"

# Show current pinned version
vp env pin

# Remove pin (delete .node-version file)
vp env pin --unpin
vp env unpin          # Alternative syntax

# Skip pre-downloading the pinned version
vp env pin 20.18.0 --no-install

# List available Node.js versions
vp env list
vp env list --lts     # Show only LTS versions
vp env list 20        # Show versions matching pattern
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
│           ├── off.rs                # off subcommand implementation
│           ├── pin.rs                # pin subcommand implementation
│           ├── unpin.rs              # unpin subcommand implementation
│           └── list.rs               # list subcommand implementation
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

## Pin Command

The `vp env pin` command provides per-directory Node.js version pinning by managing `.node-version` files.

### Behavior

**Pinning a Version:**

```bash
$ vp env pin 20.18.0
✓ Pinned Node.js version to 20.18.0
  Created .node-version in /Users/user/projects/my-app
✓ Node.js 20.18.0 installed
```

**Pinning with Aliases:**

Aliases (`lts`, `latest`) are resolved to exact versions at pin time for reproducibility:

```bash
$ vp env pin lts
✓ Pinned Node.js version to 22.13.0 (resolved from lts)
  Created .node-version in /Users/user/projects/my-app
✓ Node.js 22.13.0 installed
```

**Showing Current Pin:**

```bash
$ vp env pin
Pinned version: 20.18.0
  Source: /Users/user/projects/my-app/.node-version

# If no .node-version in current directory but found in parent
$ vp env pin
No version pinned in current directory.
  Inherited: 22.13.0 from /Users/user/projects/.node-version

# If no .node-version anywhere
$ vp env pin
No version pinned.
  Using default: 20.18.0 (from ~/.vite-plus/config.json)
```

**Removing a Pin:**

```bash
$ vp env pin --unpin
✓ Removed .node-version from /Users/user/projects/my-app

# Alternative syntax
$ vp env unpin
✓ Removed .node-version from /Users/user/projects/my-app
```

### Version Format Support

| Input | Written to File | Behavior |
|-------|-----------------|----------|
| `20.18.0` | `20.18.0` | Exact version |
| `20.18` | `20.18` | Latest 20.18.x at runtime |
| `20` | `20` | Latest 20.x.x at runtime |
| `lts` | `22.13.0` | Resolved at pin time |
| `latest` | `24.0.0` | Resolved at pin time |
| `^20.0.0` | `^20.0.0` | Semver range resolved at runtime |

### Flags

| Flag | Description |
|------|-------------|
| `--unpin` | Remove the `.node-version` file |
| `--no-install` | Skip pre-downloading the pinned version |
| `--force` | Overwrite existing `.node-version` without confirmation |

### Pre-download Behavior

By default, `vp env pin` downloads the Node.js version immediately after pinning. Use `--no-install` to skip:

```bash
$ vp env pin 20.18.0 --no-install
✓ Pinned Node.js version to 20.18.0
  Created .node-version in /Users/user/projects/my-app
  Note: Version will be downloaded on first use.
```

### Overwrite Confirmation

When a `.node-version` file already exists:

```bash
$ vp env pin 22.13.0
.node-version already exists with version 20.18.0
Overwrite with 22.13.0? (y/n): y
✓ Pinned Node.js version to 22.13.0
```

Use `--force` to skip confirmation:

```bash
$ vp env pin 22.13.0 --force
✓ Pinned Node.js version to 22.13.0
```

### Error Handling

```bash
# Invalid version format
$ vp env pin invalid
Error: Invalid Node.js version: invalid
  Use exact version (20.18.0), partial version (20), or semver range (^20.0.0)

# Version doesn't exist
$ vp env pin 99.0.0
Error: Node.js version 99.0.0 does not exist
  Run 'vp env list' to see available versions

# Network error during alias resolution
$ vp env pin lts
Error: Failed to resolve 'lts': Network error
  Check your network connection and try again
```

## List Command

The `vp env list` command displays available Node.js versions.

### Usage

```bash
# List recent versions (default: last 10 major versions)
$ vp env list
Available Node.js versions:

  LTS Versions:
    22.13.0 (Jod)      ← Latest LTS
    20.18.0 (Iron)
    18.20.0 (Hydrogen)

  Current:
    24.0.0             ← Latest

  Use 'vp env pin <version>' to pin a version.
  Use 'vp env list --all' to see all versions.

# List only LTS versions
$ vp env list --lts
LTS Node.js versions:
  22.13.0 (Jod)        ← Latest LTS
  22.12.0 (Jod)
  22.11.0 (Jod)
  ...
  20.18.0 (Iron)
  ...

# Filter by major version
$ vp env list 20
Node.js 20.x versions:
  20.18.0 (Iron LTS)
  20.17.0
  20.16.0
  ...

# Show all versions
$ vp env list --all
```

### Flags

| Flag | Description |
|------|-------------|
| `--lts` | Show only LTS versions |
| `--all` | Show all versions (not just recent) |
| `--json` | Output as JSON |

### JSON Output

```bash
$ vp env list --json
{
  "versions": [
    {"version": "24.0.0", "lts": false, "latest": true},
    {"version": "22.13.0", "lts": "Jod", "latest_lts": true},
    {"version": "22.12.0", "lts": "Jod", "latest_lts": false},
    ...
  ]
}
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
9. Implement `vp env pin [version]` for per-directory version pinning
10. Implement `vp env unpin` as alias for `pin --unpin`
11. Implement `vp env list` to show available versions

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
3. **Shell Integration**: Provide shell hooks for prompt version display

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
- ✅ Comprehensive diagnostics (`doctor`)
- ✅ Flexible shim mode control (`on`/`off` for managed vs system-first)
- ✅ Easy version pinning per project (`pin`/`unpin`)
- ✅ Version discovery with `list` command
- ✅ Leverages existing version resolution and installation infrastructure
