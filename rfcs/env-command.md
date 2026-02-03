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

- `VITE_PLUS_HOME/bin/` directory is added to PATH (system-level for IDE reliability)
- Shims (`node`, `npm`, `npx`) are symlinks to the `vp` binary (Unix) or `.cmd` wrappers (Windows)
- The `vp` CLI itself is also in `VITE_PLUS_HOME/bin/`, so users only need one PATH entry
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

### Global Package Commands

```bash
# Install a global package
vp env install typescript
vp env install typescript@5.0.0

# Install with specific Node.js version
vp env install --node 22 typescript
vp env install --node lts typescript

# List installed global packages
vp env packages
vp env packages --json

# Uninstall a global package
vp env uninstall typescript
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
│                           PATH CONFIGURATION                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  User's PATH (after setup):                                                 │
│                                                                             │
│    PATH="~/.vite-plus/bin:/usr/local/bin:/usr/bin:..."                      │
│           ▲                                                                 │
│           │                                                                 │
│           └── First in PATH = shims intercept node/npm/npx commands         │
│                                                                             │
│  When user runs `node`:                                                     │
│                                                                             │
│    $ node app.js                                                            │
│        │                                                                    │
│        ▼                                                                    │
│    Shell searches PATH left-to-right:                                       │
│        1. ~/.vite-plus/bin/node  ✓ Found! (shim)                            │
│        2. /usr/local/bin/node    (skipped)                                  │
│        3. /usr/bin/node          (skipped)                                  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                           SHIM DISPATCH FLOW                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  User runs:  $ node app.js                                                  │
│                  │                                                          │
│                  ▼                                                          │
│  ┌──────────────────────────────┐                                           │
│  │  ~/.vite-plus/bin/node       │  ◄── Symlink to vp binary (via PATH)      │
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
│  ├── bin/                                                                   │
│  │   ├── vp   ──────────────────────  Symlink to ../current/bin/vp          │
│  │   ├── node ──────────────────────┐                                       │
│  │   ├── npm  ──────────────────────┼──▶ Symlinks to ../current/bin/vp      │
│  │   └── npx  ──────────────────────┘                                       │
│  ├── current/bin/vp                   The actual vp CLI binary              │
│  ├── js_runtime/node/                 Node.js installations                 │
│  │   ├── 20.18.0/bin/node             Installed Node.js versions            │
│  │   ├── 22.13.0/bin/node                                                   │
│  │   └── ...                                                                │
│  └── config.json                      User settings (default version, etc.) │
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
├── bin/
│   ├── vp -> ../current/bin/vp       # Symlink to current vp binary (Unix)
│   ├── node -> ../current/bin/vp     # Symlink to vp binary (Unix)
│   ├── npm -> ../current/bin/vp      # Symlink to vp binary (Unix)
│   ├── npx -> ../current/bin/vp      # Symlink to vp binary (Unix)
│   ├── tsc -> ../current/bin/vp      # Symlink for global package (Unix)
│   ├── vp.cmd                        # Wrapper calling ..\current\bin\vp.exe (Windows)
│   ├── node.cmd                      # Wrapper calling vp env run node (Windows)
│   ├── npm.cmd                       # Wrapper calling vp env run npm (Windows)
│   └── npx.cmd                       # Wrapper calling vp env run npx (Windows)
├── current/
│   └── bin/
│       ├── vp                        # The actual vp CLI binary (Unix)
│       └── vp.exe                    # The actual vp CLI binary (Windows)
├── js_runtime/
│   └── node/
│       ├── 20.18.0/                  # Installed Node versions
│       │   └── bin/
│       │       ├── node
│       │       ├── npm
│       │       └── npx
│       └── 22.13.0/
├── packages/                         # Global packages
│   ├── typescript/
│   │   └── lib/
│   │       └── node_modules/
│   │           └── typescript/
│   │               └── bin/
│   ├── typescript.json               # Package metadata
│   ├── eslint/
│   └── eslint.json
├── shared/                           # NODE_PATH symlinks
│   ├── typescript -> ../packages/typescript/lib/node_modules/typescript
│   └── eslint -> ../packages/eslint/lib/node_modules/eslint
├── cache/
│   └── resolve_cache.json            # LRU cache for version resolution
├── tmp/                              # Staging directory for installs
│   └── packages/
└── config.json                       # User configuration (default version, etc.)
```

**Key Directories:**

| Directory          | Purpose                                                            |
| ------------------ | ------------------------------------------------------------------ |
| `bin/`             | vp symlink and all shims (node, npm, npx, global package binaries) |
| `current/bin/`     | The actual vp CLI binary (bin/ shims point here)                   |
| `js_runtime/node/` | Installed Node.js versions                                         |
| `packages/`        | Installed global packages with metadata                            |
| `shared/`          | NODE_PATH symlinks for package require() resolution                |
| `tmp/`             | Staging area for atomic installations                              |
| `cache/`           | Resolution cache                                                   |

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
  "shimMode": "managed"
}
```

## Version Specification

This section documents the supported version formats for `.node-version` files, `package.json` engines, and CLI commands.

### Supported Version Formats

vite-plus supports the following version specification formats, compatible with nvm, fnm, and actions/setup-node:

| Format              | Example                           | Resolution                     | Cache Expiry        |
| ------------------- | --------------------------------- | ------------------------------ | ------------------- |
| **Exact version**   | `20.18.0`, `v20.18.0`             | Used directly                  | mtime-based         |
| **Partial version** | `20`, `20.18`                     | Highest matching (prefers LTS) | time-based (1 hour) |
| **Semver range**    | `^20.0.0`, `~20.18.0`, `>=20 <22` | Highest matching (prefers LTS) | time-based (1 hour) |
| **LTS latest**      | `lts/*`                           | Highest LTS version            | time-based (1 hour) |
| **LTS codename**    | `lts/iron`, `lts/jod`             | Highest version in LTS line    | time-based (1 hour) |
| **LTS offset**      | `lts/-1`, `lts/-2`                | nth-highest LTS line           | time-based (1 hour) |
| **Wildcard**        | `*`                               | Latest version                 | time-based (1 hour) |

### Exact Versions

Exact three-part versions are used directly without network resolution:

```
20.18.0      → 20.18.0
v20.18.0     → 20.18.0 (v prefix stripped)
22.13.1      → 22.13.1
```

### Partial Versions

Partial versions (major or major.minor) are resolved to the highest matching version at runtime. LTS versions are preferred over non-LTS versions:

```
20           → 20.19.0 (highest 20.x LTS)
20.18        → 20.18.3 (highest 20.18.x)
22           → 22.13.0 (highest 22.x LTS)
```

### Semver Ranges

Standard npm/node-semver range syntax is supported. LTS versions are preferred within the matching range:

```
^20.0.0      → 20.19.0 (highest 20.x.x LTS)
~20.18.0     → 20.18.3 (highest 20.18.x)
>=20 <22     → 20.19.0 (highest in range, LTS preferred)
18 || 20     → 20.19.0 (highest LTS in either range)
18.x         → 18.20.5 (highest 18.x)
```

### LTS Aliases

LTS (Long Term Support) versions can be specified using special aliases, following the pattern established by nvm and actions/setup-node:

**`lts/*`** - Resolves to the latest (highest version number) LTS version:

```
lts/*        → 22.13.0 (latest LTS as of 2025)
```

**`lts/<codename>`** - Resolves to the highest version in a specific LTS line:

```
lts/iron     → 20.19.0 (highest v20.x)
lts/jod      → 22.13.0 (highest v22.x)
lts/hydrogen → 18.20.5 (highest v18.x)
lts/krypton  → 24.x.x (when available)
```

Codenames are case-insensitive (`lts/Iron` and `lts/iron` both work).

**`lts/-n`** - Resolves to the nth-highest LTS line (useful for testing against older supported versions):

```
lts/-1       → 20.19.0 (second-highest LTS, when latest is 22.x)
lts/-2       → 18.20.5 (third-highest LTS)
```

### LTS Codename Reference

| Codename | Major Version | LTS Status                   |
| -------- | ------------- | ---------------------------- |
| Hydrogen | 18.x          | Maintenance until 2025-04-30 |
| Iron     | 20.x          | Active LTS until 2026-04-30  |
| Jod      | 22.x          | Active LTS until 2027-04-30  |
| Krypton  | 24.x          | Will be LTS starting 2025-10 |

New LTS codenames are added dynamically based on the Node.js release schedule. vite-plus fetches the version index from nodejs.org to resolve codenames, ensuring new LTS versions are supported automatically.

### Version Resolution Priority

When resolving which Node.js version to use, vite-plus checks the following sources in order:

1. **`.node-version`** file (highest priority)
   - Checked in current directory, then parent directories
   - Simple format: one version per file

2. **`package.json#engines.node`**
   - Checked in current directory, then parent directories
   - Standard npm constraint field

3. **`package.json#devEngines.runtime`**
   - Checked in current directory, then parent directories
   - npm RFC-compliant development engines spec

4. **User default** (`~/.vite-plus/config.json`)
   - Set via `vp env default <version>`

5. **System default** (latest LTS)
   - Fallback when no version source is found

### Cache Behavior

Version resolution results are cached for performance:

- **Exact versions**: Cached until the source file mtime changes
- **Range versions** (partial, semver, LTS aliases): Cached with 1-hour TTL, then re-resolved to pick up new releases

This ensures that:

- Exact version pins are fast and deterministic
- Range specifications can pick up new releases (e.g., `20` will use a newly released `20.20.0`)
- LTS aliases automatically use newer patch versions

### File Format Compatibility

The `.node-version` file format is intentionally simple and compatible with other tools:

```
# Supported content (one per file):
20.18.0
v20.18.0
20
lts/*
lts/iron
^20.0.0

# Comments are NOT supported
# Leading/trailing whitespace is trimmed
# Only the first line is used
```

**Compatibility matrix:**

| Tool               | `.node-version` | `.nvmrc` | LTS aliases | Semver ranges |
| ------------------ | --------------- | -------- | ----------- | ------------- |
| vite-plus          | ✅              | ✅       | ✅          | ✅            |
| nvm                | ❌              | ✅       | ✅          | ✅            |
| fnm                | ✅              | ✅       | ✅          | ✅            |
| volta              | ✅              | ❌       | ❌          | ❌            |
| actions/setup-node | ✅              | ✅       | ✅          | ✅            |
| asdf               | ✅              | ❌       | ❌          | ❌            |

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
2. Check `VITE_PLUS_TOOL_RECURSION` → if set, use passthrough mode
3. Check shim mode from config:
   - If `system_first`: try system tool first, fallback to managed
   - If `managed`: use vite-plus managed Node.js
4. Resolve version (with mtime-based caching)
5. Ensure Node.js is installed (download if needed)
6. Locate tool binary in the installed Node.js
7. Prepend real node bin dir to PATH for child processes
8. Set `VITE_PLUS_TOOL_RECURSION=1` to prevent recursion
9. Execute the tool (Unix: `execve`, Windows: spawn)

### Shim Recursion Prevention

To prevent infinite loops when shims invoke other shims, vite-plus uses an environment variable marker:

**Environment Variable**: `VITE_PLUS_TOOL_RECURSION`

**Mechanism:**

1. When a shim executes the real binary, it sets `VITE_PLUS_TOOL_RECURSION=1`
2. Subsequent shim invocations check this variable
3. If set, shims use **passthrough mode** (skip version resolution, use current PATH)
4. `vp env run` explicitly **removes** this variable to force re-evaluation

**Flow Diagram:**

```
User runs: node app.js
    │
    ▼
Shim checks VITE_PLUS_TOOL_RECURSION
    │
    ├── Not set → Resolve version, set RECURSION=1, exec real node
    │
    └── Set → Passthrough mode (use current PATH)
```

**Code Example:**

```rust
const RECURSION_ENV_VAR: &str = "VITE_PLUS_TOOL_RECURSION";

fn execute_shim() {
    if env::var(RECURSION_ENV_VAR).is_ok() {
        // Passthrough: context already evaluated
        execute_with_current_path();
    } else {
        // First invocation: resolve version and set marker
        let version = resolve_version();
        let path = build_path_for_version(version);

        env::set_var(RECURSION_ENV_VAR, "1");
        execute_with_path(path);
    }
}

fn execute_run_command() {
    // Clear marker to force re-evaluation
    env::remove_var(RECURSION_ENV_VAR);

    let version = parse_version_from_args();
    execute_with_version(version);
}
```

**Why This Matters:**

- Prevents infinite loops when Node scripts spawn other Node processes
- Allows `vp env run` to override versions mid-execution
- Ensures consistent behavior in complex process trees

## Design Decisions

### 1. Single Binary with argv[0] Detection

**Decision**: Use a single `vp` binary that detects shim mode from `argv[0]`.

**Rationale**:

- Simplifies upgrades (update one binary, refresh shims)
- Reduces disk usage vs separate binaries
- Consistent behavior across all tools
- Already proven pattern (used by fnm, volta)

### 2. Symlinks for Shims (Unix)

**Decision**: Use symlinks for all shims on Unix, pointing to the vp binary.

**Rationale**:

- Symlinks preserve argv[0] - executing a symlink sets argv[0] to the symlink path, not the target
- Proven pattern used by Volta successfully
- Single binary to maintain - update `current/bin/vp` and all shims work
- No binary accumulation issues (symlinks are just filesystem pointers)
- Relative symlinks (e.g., `../current/bin/vp`) work within the same directory tree

### 3. Wrapper Scripts for Windows

**Decision**: Use `.cmd` wrapper scripts on Windows that call `vp env run <tool>`.

**Rationale**:

- Windows PATH resolution prefers `.cmd` over `.exe` for extensionless commands
- Simple wrapper format: `vp env run npm %*` - no binary copies needed
- Same pattern as Volta (`volta run <tool>`)
- Single `vp.exe` binary to maintain in `current/bin/`
- No `VITE_PLUS_SHIM_TOOL` env var complexity - dispatch via `vp env run` command

### 4. execve on Unix, spawn on Windows

**Decision**: Use `execve` (process replacement) on Unix, `spawn` on Windows.

**Rationale**:

- `execve` preserves PID, signals, and process hierarchy on Unix
- Windows doesn't support `execve`-style process replacement
- `spawn` on Windows with proper exit code propagation is standard practice

### 5. Separate VITE_PLUS_HOME from Cache

**Decision**: Keep VITE_PLUS_HOME (bin, config) separate from cache (Node binaries).

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
  ✗ VP bin not in PATH

  Found 'node' at: /usr/local/bin/node (system)
  Expected: /Users/user/.vite-plus/bin/node

Recommended Fix:
  Add to ~/.zshrc:
    export PATH="/Users/user/.vite-plus/bin:$PATH"

  Then restart your terminal and IDE.
```

## User Experience

### First-Time Setup via Install Script

**Note on Directory Structure:**

- All binaries (vp CLI and shims): `~/.vite-plus/bin/`

The global CLI installation script (`packages/global/install.sh`) will be updated to:

1. Install the `vp` binary to `~/.vite-plus/current/bin/vp`
2. Create symlink `~/.vite-plus/bin/vp` → `../current/bin/vp`
3. Configure shell PATH to include `~/.vite-plus/bin`
4. Setup Node.js version manager based on environment:
   - **CI environment**: Auto-enable (no prompt)
   - **No system Node.js**: Auto-enable (no prompt)
   - **Interactive with system Node.js**: Prompt user "Would you want Vite+ to manage Node.js versions?"
5. If already configured, skip silently

```bash
$ curl -fsSL https://viteplus.dev/install.sh | sh

Setting up VITE+(⚡)...

✔ VITE+(⚡) successfully installed!

  Version: 1.2.3
  Location: ~/.vite-plus/bin

  ✓ Created shims (node, npm, npx) in ~/.vite-plus/bin

Would you want Vite+ to manage Node.js versions?
Press Enter to accept (Y/n):
  ✓ Added to ~/.zshrc

Restart your terminal and IDE, then run 'vp env doctor' to verify.
```

### Manual Setup

If user declines or needs to reconfigure:

```bash
$ vp env setup

Setting up vite-plus environment...

Created shims:
  /Users/user/.vite-plus/bin/node
  /Users/user/.vite-plus/bin/npm
  /Users/user/.vite-plus/bin/npx

Add to your shell profile (~/.zshrc, ~/.bashrc, etc.):

  export PATH="/Users/user/.vite-plus/bin:$PATH"

For IDE support (VS Code, Cursor), ensure bin directory is in system PATH:
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
  ✓ Bin directory exists
  ✓ All shims present (node, npm, npx)

Shim Mode:
  Mode: managed
  ✓ Shims always use vite-plus managed Node.js

  Run 'vp env on' to always use managed Node.js
  Run 'vp env off' to prefer system Node.js

PATH Analysis:
  ✓ VP bin first in PATH

  node → /Users/user/.vite-plus/bin/node

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

## Shell Configuration Reference

This section documents shell configuration file behavior for PATH setup and troubleshooting.

### Zsh Configuration Files

| File        | When Loaded                                                              | Use Case                           |
| ----------- | ------------------------------------------------------------------------ | ---------------------------------- |
| `.zshenv`   | **Always** - every zsh instance (login, interactive, scripts, subshells) | PATH and environment variables     |
| `.zprofile` | Login shells only                                                        | Login-time initialization          |
| `.zshrc`    | Interactive shells only                                                  | Aliases, functions, prompts        |
| `.zlogin`   | Login shells, after `.zshrc`                                             | Commands after full initialization |

**Loading Order (Login Interactive Shell):**

```
1. /etc/zshenv     → System environment
2. ~/.zshenv       → User environment (ALWAYS loaded)
3. /etc/zprofile   → System login setup
4. ~/.zprofile     → User login setup
5. /etc/zshrc      → System interactive setup
6. ~/.zshrc        → User interactive setup
7. /etc/zlogin     → System login finalization
8. ~/.zlogin       → User login finalization
```

**Key Point:** `.zshenv` is the **most reliable** location for PATH configuration because:

- Loaded for ALL zsh instances including IDE-spawned processes
- Loaded even for non-interactive scripts and subshells

### Bash Configuration Files

| File            | When Loaded                  | Use Case                                        |
| --------------- | ---------------------------- | ----------------------------------------------- |
| `.bash_profile` | Login shells only            | macOS Terminal, SSH sessions                    |
| `.bash_login`   | Login shells only (fallback) | Used if `.bash_profile` absent                  |
| `.profile`      | Login shells only (fallback) | Used if neither above exists; also read by `sh` |
| `.bashrc`       | Interactive non-login shells | Linux terminal emulators, subshells             |

**Loading Order (Login Shell):**

```
1. /etc/profile           → System profile
2. FIRST found of:        → User profile (ONLY ONE is loaded)
   - ~/.bash_profile
   - ~/.bash_login
   - ~/.profile
3. ~/.bashrc              → ONLY if explicitly sourced by above
```

**Critical Behavior:**

- Bash reads **only the first** profile file found (`.bash_profile` > `.bash_login` > `.profile`)
- `.bashrc` is **NOT automatically loaded** in login shells - the profile file must source it
- Standard pattern: `.bash_profile` should contain `source ~/.bashrc`

### Fish Configuration Files

Fish shell uses a simpler configuration model than bash/zsh.

| File                              | When Loaded                                                    | Use Case                         |
| --------------------------------- | -------------------------------------------------------------- | -------------------------------- |
| `~/.config/fish/config.fish`      | **Always** - every fish instance (login, interactive, scripts) | All configuration including PATH |
| `~/.config/fish/conf.d/*.fish`    | **Always** - before config.fish                                | Modular configuration snippets   |
| `~/.config/fish/functions/*.fish` | On-demand when function called                                 | Autoloaded function definitions  |

**Key Points:**

- Fish has **no distinction** between login and non-login shells for configuration
- `config.fish` is always loaded, similar to zsh's `.zshenv`
- This makes Fish more reliable for IDE integration than bash
- Universal variables (`set -U`) persist across sessions without config files

**PATH Syntax:**

```fish
# Fish uses different syntax than bash/zsh
set -gx PATH $HOME/.vite-plus/bin $PATH
```

### When Configuration Files May NOT Load

| Scenario                 | Zsh Behavior    | Bash Behavior                       | Fish Behavior        |
| ------------------------ | --------------- | ----------------------------------- | -------------------- |
| Non-interactive scripts  | Only `.zshenv`  | **NOTHING** (unless `BASH_ENV` set) | `config.fish` loaded |
| IDE-launched processes   | Only `.zshenv`  | **NOTHING** (critical gap)          | `config.fish` loaded |
| SSH sessions             | All login files | `.bash_profile` only                | `config.fish` loaded |
| Subshells                | Only `.zshenv`  | `.bashrc` (interactive) or nothing  | `config.fish` loaded |
| macOS Terminal.app       | All login files | `.bash_profile` → `.bashrc`         | `config.fish` loaded |
| Linux terminal emulators | `.zshrc`        | `.bashrc` only                      | `config.fish` loaded |

### IDE Integration Challenges

GUI-launched IDEs (VS Code, Cursor, JetBrains) have special PATH inheritance issues:

**macOS:**

- GUI apps inherit environment from `launchd`, not shell rc files
- IDE terminals may spawn login or non-login shells (varies by IDE settings)
- Solution: `.zshenv` for zsh; for bash, both `.bash_profile` and `.bashrc` needed

**Linux:**

- GUI apps inherit from display manager session
- `~/.profile` is often sourced by display managers (GDM, SDDM, etc.)
- Non-login terminals only read `.bashrc`

**Windows:**

- PATH is system/user environment variable
- No shell rc file complications

### Install Script Shell Configuration

The `install.sh` script configures PATH in multiple shell files for maximum compatibility:

**For Zsh (`$SHELL` ends with `/zsh`):**

- Adds to `~/.zshenv` - ensures all zsh instances see the PATH
- Adds to `~/.zshrc` - ensures PATH is at front for interactive shells

**For Bash (`$SHELL` ends with `/bash`):**

- Adds to `~/.bash_profile` - for login shells (macOS default)
- Adds to `~/.bashrc` - for interactive non-login shells (Linux default)
- Adds to `~/.profile` - fallback for systems without `.bash_profile`

**For Fish (`$SHELL` ends with `/fish`):**

- Adds to `~/.config/fish/config.fish`

**Important Notes:**

1. Only modifies files that **already exist** - does not create new rc files
2. Checks for existing PATH entry to avoid duplicates
3. Appends with comment marker: `# Vite+ bin (https://viteplus.dev)`

### Troubleshooting PATH Issues

**Symptom: `vp` not found after installation**

1. Check which shell you're using:

   ```bash
   echo $SHELL
   ```

2. Verify the PATH entry was added:

   ```bash
   # For zsh
   grep "vite-plus" ~/.zshenv ~/.zshrc

   # For bash
   grep "vite-plus" ~/.bash_profile ~/.bashrc ~/.profile

   # For fish
   grep "vite-plus" ~/.config/fish/config.fish
   ```

3. If no entry found, manually add to appropriate file:

   ```bash
   # For zsh/bash - add this line:
   export PATH="$HOME/.vite-plus/bin:$PATH"

   # For fish - add this line:
   set -gx PATH $HOME/.vite-plus/bin $PATH
   ```

4. Source the file or restart terminal:
   ```bash
   source ~/.zshrc  # or ~/.bashrc
   # For fish: source ~/.config/fish/config.fish
   ```

**Symptom: IDE terminal doesn't see `vp` or `node`**

1. For VS Code, check terminal profile settings (login shell recommended)
2. Ensure `~/.zshenv` contains the PATH entry (most reliable for zsh)
3. For bash users: may need to configure IDE to use login shell (`bash -l`)
4. Fish users: `config.fish` is always loaded, so PATH should work in IDEs
5. Run `vp env doctor` to diagnose PATH configuration

**Symptom: Shell scripts can't find `node`**

For bash scripts, non-interactive execution doesn't load rc files. Options:

- Use `#!/usr/bin/env bash` with `BASH_ENV` set
- Source the rc file explicitly: `source ~/.bashrc`
- Use full path: `~/.vite-plus/bin/node`

Note: Fish scripts (`#!/usr/bin/env fish`) always load `config.fish`, so this issue doesn't apply.

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

| Input     | Written to File | Behavior                         |
| --------- | --------------- | -------------------------------- |
| `20.18.0` | `20.18.0`       | Exact version                    |
| `20.18`   | `20.18`         | Latest 20.18.x at runtime        |
| `20`      | `20`            | Latest 20.x.x at runtime         |
| `lts`     | `22.13.0`       | Resolved at pin time             |
| `latest`  | `24.0.0`        | Resolved at pin time             |
| `^20.0.0` | `^20.0.0`       | Semver range resolved at runtime |

### Flags

| Flag           | Description                                             |
| -------------- | ------------------------------------------------------- |
| `--unpin`      | Remove the `.node-version` file                         |
| `--no-install` | Skip pre-downloading the pinned version                 |
| `--force`      | Overwrite existing `.node-version` without confirmation |

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

## Global Package Management

vite-plus intercepts global package installations (`npm install -g`, `npm i -g`, etc.) to provide isolated, reproducible global packages with platform pinning.

### How It Works

When you run `npm install -g typescript`, vite-plus:

1. Detects the global install via argument parsing
2. Redirects installation to `~/.vite-plus/packages/typescript/`
3. Records metadata (package version, Node version used, binaries)
4. Creates shims for each binary the package provides (`tsc`, `tsserver`)

### Installation Flow

```
npm install -g typescript
    │
    ▼
Shim intercepts → detects global install
    │
    ▼
Create staging: ~/.vite-plus/tmp/packages/typescript/
    │
    ▼
Set npm_config_prefix → staging directory
    │
    ▼
Execute npm with modified environment
    │
    ▼
On success:
├── Move to: ~/.vite-plus/packages/typescript/
├── Write config: ~/.vite-plus/packages/typescript.json
├── Create shims: ~/.vite-plus/bin/tsc, tsserver
└── Update shared NODE_PATH link
```

### Package Configuration File

`~/.vite-plus/packages/typescript.json`:

```json
{
  "name": "typescript",
  "version": "5.7.0",
  "platform": {
    "node": "20.18.0",
    "npm": "10.8.0"
  },
  "bins": ["tsc", "tsserver"],
  "manager": "npm",
  "installedAt": "2024-01-15T10:30:00Z"
}
```

### Binary Execution

When running `tsc`:

1. Shim reads `~/.vite-plus/packages/typescript.json`
2. Loads the pinned platform (Node 20.18.0)
3. Constructs PATH with that Node version's bin directory
4. Sets NODE_PATH to include shared packages
5. Executes `~/.vite-plus/packages/typescript/lib/node_modules/.bin/tsc`

### Direct Installation via CLI

You can also install global packages directly using `vp env install`:

```bash
# Install a global package (uses Node.js version from current directory)
vp env install typescript

# Install with a specific Node.js version
vp env install --node 22 typescript
vp env install --node 20.18.0 typescript
vp env install --node lts typescript

# Install multiple packages
vp env install typescript eslint prettier
```

The `--node` flag allows you to specify which Node.js version to use for installation. If not provided, it resolves the version from the current directory (same as shim behavior).

### Upgrade and Uninstall

```bash
# Upgrade replaces the existing package
npm install -g typescript@latest
# Or via vite-plus:
vp env install typescript@latest

# Uninstall removes package and shims
npm uninstall -g typescript
# Or via vite-plus:
vp env uninstall typescript
```

### Environment Variable: VITE_PLUS_UNSAFE_GLOBAL

Set `VITE_PLUS_UNSAFE_GLOBAL=1` to bypass global package interception:

```bash
VITE_PLUS_UNSAFE_GLOBAL=1 npm install -g typescript
# Installs to system npm global location
```

## Run Command

The `vp env run` command executes a command with a specific Node.js version, useful for:

- Testing code against different Node versions
- Running one-off commands without changing project configuration
- CI/CD scripts that need explicit version control

### Usage

```bash
# Run with specific Node version
vp env run --node 20.18.0 node app.js

# Run with specific Node and npm versions
vp env run --node 22.13.0 --npm 10.8.0 npm install

# Version can be semver range (resolved at runtime)
vp env run --node "^20.0.0" node -v

# Run npm scripts
vp env run --node 18.20.0 npm test

# Pass arguments to the command
vp env run --node 20 -- node --inspect app.js
```

### Flags

| Flag               | Description                                                |
| ------------------ | ---------------------------------------------------------- |
| `--node <version>` | Node.js version to use (required or from project)          |
| `--npm <version>`  | npm version to use (not yet implemented, uses bundled npm) |

### Behavior

1. **Version Resolution**: Specified versions are resolved to exact versions
2. **Auto-Install**: If the version isn't installed, it's downloaded automatically
3. **PATH Construction**: Constructs PATH with specified version's bin directory
4. **Recursion Reset**: Clears `VITE_PLUS_TOOL_RECURSION` to force context re-evaluation

### Examples

```bash
# Test against multiple Node versions in CI
for version in 18 20 22; do
  vp env run --node $version npm test
done

# Run with exact version
vp env run --node 20.18.0 node -e "console.log(process.version)"
# Output: v20.18.0

# Debug with specific Node version
vp env run --node 22 -- node --inspect-brk app.js
```

### Use in Scripts

```bash
#!/bin/bash
# test-matrix.sh

VERSIONS="18.20.0 20.18.0 22.13.0"

for v in $VERSIONS; do
  echo "Testing with Node $v..."
  vp env run --node "$v" npm test || exit 1
done

echo "All tests passed!"
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

| Flag     | Description                         |
| -------- | ----------------------------------- |
| `--lts`  | Show only LTS versions              |
| `--all`  | Show all versions (not just recent) |
| `--json` | Output as JSON                      |

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

| Variable                   | Description                           | Default        |
| -------------------------- | ------------------------------------- | -------------- |
| `VITE_PLUS_HOME`           | Base directory for bin and config     | `~/.vite-plus` |
| `VITE_PLUS_LOG`            | Log level: debug, info, warn, error   | `warn`         |
| `VITE_PLUS_DEBUG_SHIM`     | Enable extra shim diagnostics         | unset          |
| `VITE_PLUS_BYPASS`         | Bypass shim and use system node       | unset          |
| `VITE_PLUS_TOOL_RECURSION` | **Internal**: Prevents shim recursion | unset          |
| `VITE_PLUS_UNSAFE_GLOBAL`  | Bypass global package interception    | unset          |

## Unix-Specific Considerations

### Shim Structure

```
VITE_PLUS_HOME/
├── bin/
│   ├── vp -> ../current/bin/vp      # Symlink to actual binary
│   ├── node -> ../current/bin/vp    # Symlink to same binary
│   ├── npm -> ../current/bin/vp     # Symlink to same binary
│   ├── npx -> ../current/bin/vp     # Symlink to same binary
│   └── tsc -> ../current/bin/vp     # Symlink for global package
└── current/
    └── bin/
        └── vp                        # The actual vp CLI binary
```

### How argv[0] Detection Works

When a user runs `node`:

1. Shell finds `~/.vite-plus/bin/node` in PATH
2. This is a symlink to `../current/bin/vp`
3. Kernel resolves symlink and executes `vp` binary
4. `argv[0]` is set to the invoking path: `node` (or full path)
5. `vp` binary extracts tool name from `argv[0]` (gets "node")
6. Dispatches to shim logic for node

**Key Insight**: Symlinks preserve argv[0]. This is the same pattern Volta uses successfully.

### Symlink Creation

All shims use relative symlinks:

```bash
# Core tools
ln -sf ../current/bin/vp ~/.vite-plus/bin/node
ln -sf ../current/bin/vp ~/.vite-plus/bin/npm
ln -sf ../current/bin/vp ~/.vite-plus/bin/npx

# Global package binaries
ln -sf ../current/bin/vp ~/.vite-plus/bin/tsc
```

## Windows-Specific Considerations

### Shim Structure

```
VITE_PLUS_HOME\
├── bin\
│   ├── vp.cmd        # Wrapper calling ..\current\bin\vp.exe
│   ├── node.cmd      # Wrapper calling vp env run node
│   ├── npm.cmd       # Wrapper calling vp env run npm
│   └── npx.cmd       # Wrapper calling vp env run npx
└── current\
    └── bin\
        └── vp.exe    # The actual vp CLI binary
```

### Wrapper Script Template (vp.cmd)

```batch
@echo off
"%~dp0..\current\bin\vp.exe" %*
exit /b %ERRORLEVEL%
```

The `vp.cmd` wrapper forwards all arguments to the actual `vp.exe` binary.

### Wrapper Script Template (node.cmd, npm.cmd, npx.cmd)

```batch
@echo off
"%~dp0..\current\bin\vp.exe" env run node %*
exit /b %ERRORLEVEL%
```

For npm:

```batch
@echo off
"%~dp0..\current\bin\vp.exe" env run npm %*
exit /b %ERRORLEVEL%
```

**How it works**:

1. User runs `npm install`
2. Windows finds `~/.vite-plus/bin/npm.cmd` in PATH
3. Wrapper calls `vp.exe env run npm install`
4. `vp env run` command handles version resolution and execution

**Benefits of this approach**:

- Single `vp.exe` binary to update in `current\bin\`
- All shims are trivial `.cmd` text files (no binary copies)
- Consistent with Volta's Windows approach
- Clear, readable wrapper scripts

### Windows Installation (install.ps1)

The Windows installer (`install.ps1`) follows this flow:

1. Download and install `vp.exe` to `~/.vite-plus/current/bin/`
2. Create `~/.vite-plus/bin/vp.cmd` wrapper script
3. Create shim wrappers: `node.cmd`, `npm.cmd`, `npx.cmd`
4. Configure User PATH to include `~/.vite-plus/bin`

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
2. Implement argv[0] detection in main.rs
3. Implement shim dispatch logic for `node`
4. Implement `vp env setup` (Unix symlinks, Windows .cmd wrappers)
5. Implement `vp env doctor` basic diagnostics
6. Add resolution cache (persists across upgrades with version field)
7. Implement `vp env default [version]` to set/show global default Node.js version
8. Implement `vp env on` and `vp env off` for shim mode control
9. Implement `vp env pin [version]` for per-directory version pinning
10. Implement `vp env unpin` as alias for `pin --unpin`
11. Implement `vp env list` to show available versions
12. Implement recursion prevention (`VITE_PLUS_TOOL_RECURSION`)
13. Implement `vp env run --node <version>` command

### Phase 2: Full Tool Support (P1)

1. Add shims for `npm`, `npx`
2. Implement `vp env which`
3. Implement `vp env --current --json`
4. Enhanced doctor with conflict detection
5. Implement global package interception for npm
6. Implement package metadata storage
7. Implement per-package binary shims
8. Implement `vp env packages` to list installed global packages
9. Implement `vp env uninstall <package>` command
10. Implement `vp env install <package>` command with `--node` flag

### Phase 3: Polish (P2)

1. Implement `vp env --print` for session-only env
2. Add VITE_PLUS_BYPASS escape hatch
3. Improve error messages
4. Add IDE-specific setup guidance
5. Documentation

### Phase 4: Future Enhancements (P3)

1. NODE_PATH setup for shared package resolution
2. Yarn global package interception (`yarn global add/remove`)
3. pnpm global package interception (`pnpm add -g`)

## Backward Compatibility

This is a new feature with no impact on existing functionality. The `vp` binary continues to work normally when invoked directly.

## Future Enhancements

1. **Multiple Runtime Support**: Extend shim architecture for other runtimes (Bun, Deno)
2. **SQLite Cache**: Replace JSON cache with SQLite for better performance at scale
3. **Shell Integration**: Provide shell hooks for prompt version display

## Design Decisions Summary

The following decisions have been made:

1. **VITE_PLUS_HOME Default Location**: `~/.vite-plus` - Simple, memorable path that's easy for users to find and configure.

2. **Windows Wrapper Strategy**: `.cmd` wrappers that call `vp env run <tool>` - Consistent with Volta, no binary copies needed.

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
