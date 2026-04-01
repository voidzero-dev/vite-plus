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
- Shims (`node`, `npm`, `npx`) are symlinks to the `vp` binary (Unix) or trampoline `.exe` files (Windows)
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

# List locally installed Node.js versions
vp env list
vp env ls             # Alias

# List available Node.js versions from the registry
vp env list-remote
vp env list-remote --lts     # Show only LTS versions
vp env list-remote 20        # Show versions matching pattern
```

### Session Version Override

```bash
# Use a specific Node.js version for this shell session
vp env use 24          # Switch to Node 24.x
vp env use lts         # Switch to latest LTS
vp env use             # Install & activate project's configured version
vp env use --unset     # Remove session override

# Options
vp env use --no-install           # Skip auto-install if version not present
vp env use --silent-if-unchanged  # Suppress output if version already active
```

**How it works:**

1. `~/.vite-plus/env` includes a `vp()` shell function that intercepts `vp env use` calls
2. The wrapper sets `VITE_PLUS_ENV_USE_EVAL_ENABLE=1` before calling `command vp env use ...`
3. When the env var is present (wrapper active), `vp env use` outputs shell commands to stdout for eval
4. When the env var is absent (CI, direct invocation), `vp env use` writes a session file (`~/.vite-plus/.session-node-version`) instead
5. The shim dispatch checks `VITE_PLUS_NODE_VERSION` env var first, then the session file, in the resolution chain

**Automatic session file (for CI / wrapper-less environments):**

When `vp env use` detects that the shell eval wrapper is not active (i.e., `VITE_PLUS_ENV_USE_EVAL_ENABLE` is not set), it automatically writes the resolved version to `~/.vite-plus/.session-node-version`. Shims read this file directly from disk, so `vp env use` works without the shell wrapper — no extra flags needed. The env var still takes priority when set, so the shell wrapper experience is unchanged.

```bash
# GitHub Actions example (no shell wrapper, session file written automatically)
- run: vp env use 20
- run: node --version   # v20.x via shim reading session file
- run: vp env use --unset  # Clean up
```

**Shell-specific output:**

| Shell            | Set                                       | Unset                                        |
| ---------------- | ----------------------------------------- | -------------------------------------------- |
| POSIX (bash/zsh) | `export VITE_PLUS_NODE_VERSION=20.18.1`   | `unset VITE_PLUS_NODE_VERSION`               |
| Fish             | `set -gx VITE_PLUS_NODE_VERSION 20.18.1`  | `set -e VITE_PLUS_NODE_VERSION`              |
| PowerShell       | `$env:VITE_PLUS_NODE_VERSION = "20.18.1"` | `Remove-Item Env:VITE_PLUS_NODE_VERSION ...` |
| cmd.exe          | `set VITE_PLUS_NODE_VERSION=20.18.1`      | `set VITE_PLUS_NODE_VERSION=`                |

**Shell function wrappers** are included in env files created by `vp env setup`:

- `~/.vite-plus/env` (POSIX - bash/zsh): `vp()` function
- `~/.vite-plus/env.fish` (fish): `function vp`
- `~/.vite-plus/env.ps1` (PowerShell): `function vp`
- `~/.vite-plus/bin/vp-use.cmd` (cmd.exe): dedicated wrapper since cmd.exe lacks shell functions

### Node.js Version Management

```bash
# Install a Node.js version
vp env install 20.18.0
vp env install lts
vp env install latest

# Uninstall a Node.js version
vp env uninstall 20.18.0
```

### Global Package Commands

```bash
# Install a global package
vp install -g typescript
vp install -g typescript@5.0.0

# Install with specific Node.js version
vp install -g --node 22 typescript
vp install -g --node lts typescript

# Force install (auto-uninstalls conflicting packages)
vp install -g --force eslint-v9    # Removes 'eslint' if it provides same binary

# List installed global packages
vp list -g
vp list -g --json

# Example output (table format with colored package names):
# Package            Node version   Binaries
# ---                ---            ---
# pnpm@10.28.2      22.22.0        pnpm, pnpx
# serve@14.2.5      22.22.0        serve
# typescript@5.9.3  22.22.0        tsc, tsserver

# Uninstall a global package
vp remove -g typescript

# Update global packages
vp update -g              # Update all global packages
vp update -g typescript   # Update specific package
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
│  │  (walk up directory tree)    │     │  0. VITE_PLUS_NODE_VERSION  │       │
│  └──────────────┬───────────────┘     │  1. .session-node-version   │       │
│                 │                     │  2. .node-version           │       │
│                 │                     │  3. package.json#engines    │       │
│                 │                     │  4. package.json#devEngines │       │
│                 │                     │  5. User default (config)   │       │
│                 │                     │  6. Latest LTS              │       │
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
│  ├── .session-node-version              Session override (written by vp env use)│
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
│   ├── vp.exe                        # Trampoline forwarding to current\bin\vp.exe (Windows)
│   ├── node.exe                      # Trampoline shim for node (Windows)
│   ├── npm.exe                       # Trampoline shim for npm (Windows)
│   ├── npx.exe                       # Trampoline shim for npx (Windows)
│   └── tsc.exe                       # Trampoline shim for global package (Windows)
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
├── bins/                             # Per-binary config files (tracks ownership)
│   ├── tsc.json                      # { "package": "typescript", ... }
│   ├── tsserver.json
│   └── eslint.json
├── shared/                           # NODE_PATH symlinks
│   ├── typescript -> ../packages/typescript/lib/node_modules/typescript
│   └── eslint -> ../packages/eslint/lib/node_modules/eslint
├── cache/
│   └── resolve_cache.json            # LRU cache for version resolution
├── tmp/                              # Staging directory for installs
│   └── packages/
├── .session-node-version             # Session override (written by `vp env use`)
└── config.json                       # User configuration (default version, etc.)
```

**Key Directories:**

| Directory          | Purpose                                                            |
| ------------------ | ------------------------------------------------------------------ |
| `bin/`             | vp symlink and all shims (node, npm, npx, global package binaries) |
| `current/bin/`     | The actual vp CLI binary (bin/ shims point here)                   |
| `js_runtime/node/` | Installed Node.js versions                                         |
| `packages/`        | Installed global packages with metadata                            |
| `bins/`            | Per-binary config files (tracks which package owns each binary)    |
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
| **Wildcard**        | `*`                               | Highest matching (prefers LTS) | time-based (1 hour) |
| **Latest**          | `latest`                          | Absolute latest version        | time-based (1 hour) |

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

0. **`VITE_PLUS_NODE_VERSION` env var** (session override, highest priority)
   - Set by `vp env use` via shell wrapper eval
   - Overrides all file-based resolution

1. **`.session-node-version`** file (session override)
   - Written by `vp env use` to `~/.vite-plus/.session-node-version`
   - Works without shell eval wrapper (CI environments)
   - Deleted by `vp env use --unset`

2. **`.node-version`** file
   - Checked in current directory, then parent directories
   - Simple format: one version per file

3. **`package.json#engines.node`**
   - Checked in current directory, then parent directories
   - Standard npm constraint field

4. **`package.json#devEngines.runtime`**
   - Checked in current directory, then parent directories
   - npm RFC-compliant development engines spec

5. **User default** (`~/.vite-plus/config.json`)
   - Set via `vp env default <version>`

6. **System default** (latest LTS)
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
│           ├── list.rs               # list subcommand implementation
│           └── use.rs                # use subcommand implementation
```

### Shim Dispatch Flow

1. Check `VITE_PLUS_BYPASS` environment variable → bypass to system tool (filters all listed directories from PATH)
2. Check `VITE_PLUS_TOOL_RECURSION` → if set, use passthrough mode
3. Check shim mode from config:
   - If `system_first`: try system tool first, fallback to managed; appends own bin dir to `VITE_PLUS_BYPASS` before exec to prevent loops with multiple installations
   - If `managed`: use vite-plus managed Node.js
4. Resolve version (with mtime-based caching)
5. Ensure Node.js is installed (download if needed)
6. Locate tool binary in the installed Node.js
7. Prepend real node bin dir to PATH for child processes
8. Set `VITE_PLUS_TOOL_RECURSION=1` to prevent recursion
9. Execute the tool (Unix: `execve`, Windows: spawn)

### Shim Recursion Prevention

To prevent infinite loops when shims invoke other shims, vite-plus uses environment variable markers:

**Environment Variable**: `VITE_PLUS_TOOL_RECURSION`

**Mechanism:**

1. When a shim executes the real binary, it sets `VITE_PLUS_TOOL_RECURSION=1`
2. Subsequent shim invocations check this variable
3. If set, shims use **passthrough mode** (skip version resolution, use current PATH)
4. `vp env exec` explicitly **removes** this variable to force re-evaluation

**Environment Variable**: `VITE_PLUS_BYPASS` (PATH-style list)

**SystemFirst Loop Prevention:**

When multiple vite-plus installations exist in PATH and `system_first` mode is active, each installation could find the other's shim as the "system tool", causing an infinite exec loop. To prevent this:

1. In `system_first` mode, before exec'ing the found system tool, the current installation appends its own bin directory to `VITE_PLUS_BYPASS`
2. The next installation sees `VITE_PLUS_BYPASS` is set and enters bypass mode via `find_system_tool()`
3. `find_system_tool()` filters all directories listed in `VITE_PLUS_BYPASS` (plus its own bin dir) from PATH
4. This ensures the search skips all known vite-plus bin directories and finds the real system binary (or errors cleanly)
5. `VITE_PLUS_BYPASS` is preserved through `vp env exec` so loop protection remains active

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
- Allows `vp env exec` to override versions mid-execution
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

### 3. Trampoline Executables for Windows

**Decision**: Use lightweight trampoline `.exe` files on Windows instead of `.cmd` wrappers. Each trampoline detects its tool name from its own filename, sets `VITE_PLUS_SHIM_TOOL`, and spawns `vp.exe`. See [RFC: Trampoline EXE for Shims](./trampoline-exe-for-shims.md).

**Rationale**:

- `.cmd` wrappers cause "Terminate batch job (Y/N)?" prompt on Ctrl+C
- `.exe` files work in all shells (cmd.exe, PowerShell, Git Bash) without needing separate wrappers
- Single trampoline binary (~100-150KB) copied per tool — no `.cmd` + shell script pair needed
- Ctrl+C handled cleanly via `SetConsoleCtrlHandler`

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

1. `VITE_PLUS_NODE_VERSION` env var (session override)
2. `.session-node-version` file (session override)
3. `.node-version` in current or parent directories
4. `package.json#engines.node` in current or parent directories
5. `package.json#devEngines.runtime` in current or parent directories
6. **User Default**: Configured via `vp env default <version>` (stored in `~/.vite-plus/config.json`)
7. **System Default**: Latest LTS version

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
Installation
  ✓ VITE_PLUS_HOME    ~/.vite-plus
  ✓ Bin directory     exists
  ✓ Shims             node, npm, npx

Configuration
  ✓ Shim mode         managed

PATH
  ✗ vp                not in PATH
                      Expected: ~/.vite-plus/bin

    Add to your shell profile (~/.zshrc, ~/.bashrc, etc.):

      . "$HOME/.vite-plus/env"

    Then restart your terminal.

...

✗ Some issues found. Run the suggested commands to fix them.
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
$ curl -fsSL https://vite.plus | sh

Setting up VITE+...

Would you want Vite+ to manage Node.js versions?
Press Enter to accept (Y/n):

✔ VITE+ successfully installed!

  The Unified Toolchain for the Web.

  Get started:
    vp create       Create a new project
    vp env          Manage Node.js versions
    vp install      Install dependencies
    vp dev          Start dev server

  Node.js is now managed by Vite+ (via vp env).
  Run vp env doctor to verify your setup.

  Run vp help for more information.

  Note: Run `source ~/.zshrc` or restart your terminal.
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
Installation
  ✓ VITE_PLUS_HOME    ~/.vite-plus
  ✓ Bin directory     exists
  ✓ Shims             node, npm, npx

Configuration
  ✓ Shim mode         managed
  ✓ IDE integration   env sourced in ~/.zshenv

PATH
  ✓ vp                first in PATH
  ✓ node              ~/.vite-plus/bin/node (vp shim)
  ✓ npm               ~/.vite-plus/bin/npm (vp shim)
  ✓ npx               ~/.vite-plus/bin/npx (vp shim)

Version Resolution
    Directory         /Users/user/projects/my-app
    Source            .node-version
    Version           20.18.0
  ✓ Node binary       installed

✓ All checks passed
```

**Doctor Output with Session Override:**

```bash
$ vp env doctor
...

Configuration
  ✓ Shim mode         managed
  ✓ IDE integration   env sourced in ~/.zshenv
  ⚠ Session override  VITE_PLUS_NODE_VERSION=20.18.0
                      Overrides all file-based resolution.
                      Run 'vp env use --unset' to remove.
  ⚠ Session override (file)  .session-node-version=20.18.0
                      Written by 'vp env use'. Run 'vp env use --unset' to remove.

...
```

**Doctor Output with System-First Mode:**

```bash
$ vp env doctor
...

Configuration
  ✓ Shim mode         system-first
    System Node.js    /usr/local/bin/node
  ✓ IDE integration   env sourced in ~/.zshenv

...
```

**Doctor Output with System-First Mode (No System Node):**

```bash
$ vp env doctor
...

Configuration
  ✓ Shim mode         system-first
  ⚠ System Node.js    not found (will use managed)

...
```

**Doctor Output (Unhealthy):**

```bash
$ vp env doctor
Installation
  ✓ VITE_PLUS_HOME    ~/.vite-plus
  ✗ Bin directory     does not exist
  ✗ Missing shims     node, npm, npx
                      Run 'vp env setup' to create bin directory and shims.

Configuration
  ✓ Shim mode         managed

PATH
  ✗ vp                not in PATH
                      Expected: ~/.vite-plus/bin

    Add to your shell profile (~/.zshrc, ~/.bashrc, etc.):

      . "$HOME/.vite-plus/env"

    For fish shell, add to ~/.config/fish/config.fish:

      source "$HOME/.vite-plus/env.fish"

    Then restart your terminal.

  node                not found
  npm                 not found
  npx                 not found

Version Resolution
    Directory         /Users/user/projects/my-app
    Source            .node-version
    Version           20.18.0
  ⚠ Node binary       not installed
                      Version will be downloaded on first use.

Conflicts
  ⚠ nvm               detected (NVM_DIR is set)
                      Consider removing other version managers from your PATH
                      to avoid version conflicts.

IDE Setup
  ⚠ GUI applications may not see shell PATH changes.

    macOS:
      Add to ~/.zshenv or ~/.profile:
        . "$HOME/.vite-plus/env"
      Then restart your IDE to apply changes.

✗ Some issues found. Run the suggested commands to fix them.
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

Shims will now always use the Vite+ managed Node.js.
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

Shows the path to the tool binary that would be executed. The first line is always the bare path (pipe-friendly, copy-pastable).

**Core tools** - shows the resolved Node.js binary path with version and resolution source:

```bash
$ vp env which node
/Users/user/.vite-plus/js_runtime/node/20.18.0/bin/node
  Version:    20.18.0
  Source:     /Users/user/projects/my-app/.node-version

$ vp env which npm
/Users/user/.vite-plus/js_runtime/node/20.18.0/bin/npm
  Version:    20.18.0
  Source:     /Users/user/projects/my-app/.node-version
```

When using session override:

```bash
$ vp env which node
/Users/user/.vite-plus/js_runtime/node/18.20.0/bin/node
  Version:    18.20.0
  Source:     VITE_PLUS_NODE_VERSION (session)
```

**Global packages** - shows binary path plus package metadata:

```bash
$ vp env which tsc
/Users/user/.vite-plus/packages/typescript/lib/node_modules/typescript/bin/tsc
  Package:    typescript@5.7.0
  Binaries:   tsc, tsserver
  Node:       20.18.0
  Installed:  2024-01-15

$ vp env which eslint
/Users/user/.vite-plus/packages/eslint/lib/node_modules/eslint/bin/eslint.js
  Package:    eslint@9.0.0
  Binaries:   eslint
  Node:       22.13.0
  Installed:  2024-02-20
```

| Tool Type       | Resolution                          | Output                                                         |
| --------------- | ----------------------------------- | -------------------------------------------------------------- |
| Core tools      | Node.js version from project config | Binary path + Version + Source                                 |
| Global packages | Package metadata lookup             | Binary path + Package version + Node.js version + Install date |

**Error cases:**

```bash
# Unknown tool (not core tool, not in any global package)
$ vp env which unknown-tool
error: tool 'unknown-tool' not found
Not a core tool (node, npm, npx) or installed global package.
Run 'vp list -g' to see installed packages.

# Node.js version not installed
$ vp env which node
error: node not found
Node.js 20.18.0 is not installed.
Run 'vp env install 20.18.0' to install it.

# Global package binary missing
$ vp env which tsc
error: binary 'tsc' not found
Package typescript may need to be reinstalled.
Run 'vp install -g typescript' to reinstall.
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
  Run 'vp env list-remote' to see available versions

# Network error during alias resolution
$ vp env pin lts
Error: Failed to resolve 'lts': Network error
  Check your network connection and try again
```

## Global Package Management

vite-plus provides cross-Node-version global package management via `vp install -g`, `vp remove -g`, and `vp update -g`. Unlike `npm install -g` which installs into a Node-version-specific directory, vite-plus manages global packages independently so they persist across Node.js version changes.

Note: `npm install -g` passes through to the real npm (Node-version-specific). Use `vp install -g` for vite-plus managed global packages.

### How It Works

When you run `vp install -g typescript`, vite-plus:

1. Resolves the Node.js version (from `--node` flag or current directory)
2. Installs the package to `~/.vite-plus/packages/typescript/`
3. Records metadata (package version, Node version used, binaries)
4. Creates shims for each binary the package provides (`tsc`, `tsserver`)

### Installation Flow

```
vp install -g typescript
    │
    ▼
Parse global flag → route to managed global install
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

### Installation with Specific Node.js Version

```bash
# Install a global package (uses Node.js version from current directory)
vp install -g typescript

# Install with a specific Node.js version
vp install -g --node 22 typescript
vp install -g --node 20.18.0 typescript
vp install -g --node lts typescript

# Install multiple packages
vp install -g typescript eslint prettier
```

The `--node` flag allows you to specify which Node.js version to use for installation. If not provided, it resolves the version from the current directory (same as shim behavior).

### Upgrade and Uninstall

```bash
# Upgrade replaces the existing package
vp update -g typescript
vp install -g typescript@latest

# Update all global packages
vp update -g

# Uninstall removes package and shims
vp remove -g typescript
```

### Binary Conflict Handling

When two packages provide the same binary name (e.g., both `eslint` and `eslint-v9` provide an `eslint` binary), vite-plus uses a **Volta-style hard fail** approach:

#### Conflict Detection

Each binary has a per-binary config file that tracks which package owns it:

```
~/.vite-plus/
  packages/
    typescript.json      # Package metadata
    eslint.json
  bins/                  # Per-binary config files
    tsc.json             # { "package": "typescript", ... }
    tsserver.json
    eslint.json          # { "package": "eslint", ... }
```

**Binary config format** (`~/.vite-plus/bins/tsc.json`):

```json
{
  "name": "tsc",
  "package": "typescript",
  "version": "5.7.0",
  "nodeVersion": "20.18.0"
}
```

#### Default Behavior: Hard Fail

When installing a package that provides a binary already owned by another package, the installation **fails with a clear error**:

```bash
$ vp install -g eslint-v9
Installing eslint-v9 globally...

error: Executable 'eslint' is already installed by eslint

Please remove eslint before installing eslint-v9, or use --force to auto-replace
```

This approach:

- Prevents silent binary masking
- Makes conflicts explicit and visible
- Requires intentional user action to resolve

#### Force Mode: Auto-Uninstall

The `--force` flag automatically uninstalls the conflicting package before installing the new one:

```bash
$ vp install -g --force eslint-v9
Installing eslint-v9 globally...
Uninstalling eslint (conflicts with eslint-v9)...
Uninstalled eslint
Installed eslint-v9 v9.0.0
Binaries: eslint
```

**Important**: `--force` completely removes the conflicting package (not just the binary). This ensures a clean state without orphaned files.

#### Two-Phase Uninstall

Uninstall uses a resilient two-phase approach (inspired by Volta):

1. **Phase 1**: Try to use `PackageMetadata` to get binary names
2. **Phase 2**: If metadata is missing, scan `bins/` directory for orphaned binary configs

This allows recovery even if package metadata is corrupted or manually deleted.

```bash
# Normal uninstall
$ vp remove -g typescript
Uninstalling typescript...
Uninstalled typescript

# Recovery mode (if typescript.json is missing)
$ vp remove -g typescript
Uninstalling typescript...
note: Package metadata not found, scanning for orphaned binaries...
Uninstalled typescript
```

#### Deterministic Binary Resolution

Binary execution uses per-binary config for deterministic lookup:

1. Check `~/.vite-plus/bins/{binary}.json` for owner package
2. Load package metadata to get Node.js version and binary path
3. If not found, the binary is not installed (no fallback scanning)

This eliminates the non-deterministic behavior of filesystem iteration order.

### npm Global Install Guidance

When the npm shim detects `npm install -g <packages>`, it runs real npm normally but uses `spawn+wait` (instead of `exec`) so it can run post-install checks. After npm completes successfully, it checks whether the installed binaries are reachable from `$PATH` and prints a hint if they aren't.

#### Why This Is Needed

```
~/.vite-plus/
├── bin/                          ← ON $PATH (only this dir)
│   ├── node → ../current/bin/vp  (shim)
│   ├── npm → ../current/bin/vp   (shim)
│   └── npx → ../current/bin/vp   (shim)
└── js_runtime/node/20.18.0/bin/  ← NOT on $PATH
    ├── node
    ├── npm
    ├── npx
    └── codex                     ← installed by `npm i -g`, but unreachable
```

Users instinctively run `npm install -g codex`, which installs into the managed Node's bin dir — not on `$PATH`. The binary is silently unreachable.

#### Call Flow: `npm install -g codex` (with post-install hint)

```
User runs: npm install -g codex
         │
         ▼
┌─────────────────────────┐
│  ~/.vite-plus/bin/npm   │  (symlink to vp binary)
│  argv[0] = "npm"        │
└────────────┬────────────┘
             │
             ▼
┌───────────────────────────────────────────────────────────┐
│  dispatch("npm", ["install", "-g", "codex"])               │
│  (crates/vite_global_cli/src/shim/dispatch.rs)             │
│                                                             │
│  1–5. vpx / recursion / bypass / shim / core checks        │
│  6. resolve version    → 20.18.0                           │
│  7. ensure installed   → ok                                │
│  8. locate npm binary  → ~/.vite-plus/js_runtime/          │
│                           node/20.18.0/bin/npm              │
│  9. save original_path = $PATH                             │
│  10. prepend node bin dir to PATH                          │
│  11. set recursion marker                                  │
│                                                             │
│  ┌─── npm global install detection ─────────────────────┐  │
│  │                                                       │  │
│  │  parse_npm_global_install(args)                       │  │
│  │    → detects "install" + "-g"                         │  │
│  │    → extracts packages: ["codex"]                     │  │
│  │    → returns Some(NpmGlobalInstall)                   │  │
│  │                                                       │  │
│  │  spawn_tool(npm_path, args)    ← NOT exec!            │  │
│  │    → runs real npm install -g codex                   │  │
│  │    → waits for completion, exit_code = 0              │  │
│  │                                                       │  │
│  │  check_npm_global_install_result(                     │  │
│  │      pkgs, ver, orig_path, npm_path)                  │  │
│  │                                                       │  │
│  │    ┌─ Determine actual npm global prefix ───────────┐ │  │
│  │    │  run `npm config get prefix` → e.g. /usr/local │ │  │
│  │    │  npm_bin_dir = <prefix>/bin/                    │ │  │
│  │    │  (fallback: node_dir if npm fails)             │ │  │
│  │    └────────────────────────────────────────────────┘ │  │
│  │                                                       │  │
│  │    ┌─ Is npm_bin_dir in original_path? ─────────────┐ │  │
│  │    │  YES → return (binaries on PATH)               │ │  │
│  │    │  NO  → continue to per-binary check            │ │  │
│  │    └────────────────────────────────────────────────┘ │  │
│  │                                                       │  │
│  │    → for each binary in package:                      │  │
│  │        skip core shims (node/npm/npx/vp)              │  │
│  │        if already exists in ~/.vite-plus/bin/:         │  │
│  │          if BinConfig exists → managed_conflicts       │  │
│  │          skip (don't overwrite)                        │  │
│  │        check source exists in npm_bin_dir             │  │
│  │        add to missing_bins list                       │  │
│  │    → warn about managed conflicts                     │  │
│  │    → interactive? prompt to create links              │  │
│  │      non-interactive? create links directly           │  │
│  │    → prints tip: use `vp install -g` instead          │  │
│  │                                                       │  │
│  │  return exit_code (0)                                 │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

**Conflict with `vp install -g` shims**: If a binary already exists in `~/.vite-plus/bin/` AND has a BinConfig file (`~/.vite-plus/bins/{name}.json`), it is managed by `vp install -g`. The shim warns the user instead of silently skipping:

```
'codex' is already managed by `vp install -g`. Run `vp uninstall -g` first to replace it.
```

**Interactive mode** (stdin is a TTY):

```
'codex' is not available on your PATH.
Create a link in ~/.vite-plus/bin/ to make it available? [Y/n]
```

If the user confirms (Y or Enter):

- Creates a symlink: `~/.vite-plus/bin/codex` → `~/.vite-plus/js_runtime/node/20.18.0/bin/codex`
- Prints: `Linked 'codex' to ~/.vite-plus/bin/codex`

Then always prints the tip:

```
tip: Use `vp install -g codex` for managed shims that persist across Node.js version changes.
```

**Non-interactive mode** (piped/CI):

- Creates the symlink directly (no prompt)
- Prints: `Linked 'codex' to ~/.vite-plus/bin/codex`
- Prints the same tip

#### Call Flow: Normal `npm install react` — unaffected

```
User runs: npm install react
         │
         ▼
┌───────────────────────────────────────────────────┐
│  dispatch("npm", ["install", "react"])              │
│                                                     │
│  ... version resolution, PATH setup ...             │
│                                                     │
│  parse_npm_global_install(args)                      │
│    → no "-g" or "--global" flag                      │
│    → returns None                                    │
│                                                     │
│  (falls through to normal exec_tool)                 │
│    → exec_tool(npm_path, args)                       │
│       └─ replaces process with real npm (Unix exec)  │
└───────────────────────────────────────────────────┘
```

#### `npm uninstall -g` Link Cleanup

When `npm uninstall -g` is detected, the shim uses `spawn_tool()` (like install) to retain control after npm finishes. Before running npm, it collects bin names from the package's `package.json` (which will be removed by npm). After a successful uninstall, it removes the corresponding symlinks from `~/.vite-plus/bin/`.

**Link tracking via BinConfig**: When `npm install -g` creates links in `~/.vite-plus/bin/`, a `BinConfig` with `source: "npm"` is written to `~/.vite-plus/bins/{name}.json`. This distinguishes npm-created links from `vp install -g` managed shims (`source: "vp"`) and user-owned binaries (no BinConfig).

**Safe uninstall cleanup**: `npm uninstall -g` only removes links that have a BinConfig with `source: "npm"` AND whose `package` field matches the package being uninstalled. This prevents removing links that were overwritten by a later install of a different package exposing the same bin name. User-owned binaries and `vp install -g` managed shims are never touched.

**`--prefix` support**: When `--prefix <dir>` is passed to `npm install -g` or `npm uninstall -g`, the shim uses that prefix for package.json lookups and bin dir resolution instead of running `npm config get prefix`. Both absolute and relative paths are supported — relative paths (e.g., `./custom`, `../foo`) are resolved against the current working directory.

**Windows local path support**: `resolve_package_name()` treats drive-letter paths (`C:\...`) as local paths.

#### Design Decision: spawn vs exec

On Unix, `exec_tool()` uses `exec()` which replaces the current process — no code runs after. For `npm install -g` and `npm uninstall -g` specifically, we use `spawn_tool()` (spawn + wait) to retain control after npm finishes, enabling the post-install hint and post-uninstall link cleanup. All other npm commands continue to use `exec_tool()` for zero overhead.

## Exec Command

The `vp env exec` command executes a command with a specific Node.js version. It operates in two modes:

1. **Explicit version mode**: When `--node` is provided, runs with the specified version
2. **Shim mode**: When `--node` is not provided and the command is a shim tool (node/npm/npx or global package), uses the same version resolution as Unix symlinks

This is useful for:

- Testing code against different Node versions
- Running one-off commands without changing project configuration
- CI/CD scripts that need explicit version control
- Legacy Windows `.cmd` wrappers (deprecated in favor of trampoline `.exe` shims)

### Usage

```bash
# Shim mode: version resolved automatically (same as Unix symlinks)
vp env exec node --version        # Core tool - resolves from .node-version/package.json
vp env exec npm install           # Core tool
vp env exec npx vitest            # Core tool
vp env exec tsc --version         # Global package - uses Node.js from install time

# Explicit version mode: run with specific Node version
vp env exec --node 20.18.0 node app.js

# Run with specific Node and npm versions
vp env exec --node 22.13.0 --npm 10.8.0 npm install

# Version can be semver range (resolved at runtime)
vp env exec --node "^20.0.0" node -v

# Run npm scripts
vp env exec --node 18.20.0 npm test

# Pass arguments to the command
vp env exec --node 20 -- node --inspect app.js

# Error: non-shim command without --node
vp env exec python --version      # Fails: --node required for non-shim tools
```

### Flags

| Flag               | Description                                                                   |
| ------------------ | ----------------------------------------------------------------------------- |
| `--node <version>` | Node.js version to use (optional for shim tools, required for other commands) |
| `--npm <version>`  | npm version to use (not yet implemented, uses bundled npm)                    |

### Shim Mode Behavior

When `--node` is **not provided** and the first command is a shim tool:

- **Core tools (node, npm, npx)**: Version resolved from `.node-version`, `package.json#engines.node`, or default
- **Global packages (tsc, eslint, etc.)**: Uses the Node.js version that was used during `vp install -g`

Both use the **exact same code path** as Unix symlinks (`shim::dispatch()`), ensuring identical behavior across platforms. On Windows, trampoline `.exe` shims set `VITE_PLUS_SHIM_TOOL` to enter shim dispatch mode.

**Important**: The `VITE_PLUS_TOOL_RECURSION` environment variable is cleared before dispatch to ensure fresh version resolution, even when invoked from within a context where the variable is already set (e.g., when pnpm runs through the vite-plus shim).

### Explicit Version Mode Behavior

When `--node` **is provided**:

1. **Version Resolution**: Specified versions are resolved to exact versions
2. **Auto-Install**: If the version isn't installed, it's downloaded automatically
3. **PATH Construction**: Constructs PATH with specified version's bin directory
4. **Recursion Reset**: Clears `VITE_PLUS_TOOL_RECURSION` to force context re-evaluation

### Examples

```bash
# Shim mode: same behavior as Unix symlinks
vp env exec node -v               # Uses version from project config
vp env exec npm install           # Uses same version
vp env exec tsc --version         # Global package

# Test against multiple Node versions in CI
for version in 18 20 22; do
  vp env exec --node $version npm test
done

# Run with exact version
vp env exec --node 20.18.0 node -e "console.log(process.version)"
# Output: v20.18.0

# Debug with specific Node version
vp env exec --node 22 -- node --inspect-brk app.js
```

### Use in Scripts

```bash
#!/bin/bash
# test-matrix.sh

VERSIONS="18.20.0 20.18.0 22.13.0"

for v in $VERSIONS; do
  echo "Testing with Node $v..."
  vp env exec --node "$v" npm test || exit 1
done

echo "All tests passed!"
```

## List Command (Local)

The `vp env list` (alias `ls`) command displays locally installed Node.js versions.

### Usage

```bash
$ vp env list
* v18.20.0
* v20.18.0 default
* v22.13.0 current
```

- Current version line is highlighted in blue
- `current` and `default` markers are shown in dimmed text

### Flags

| Flag     | Description    |
| -------- | -------------- |
| `--json` | Output as JSON |

### JSON Output

```bash
$ vp env list --json
[
  {"version": "18.20.0", "current": false, "default": false},
  {"version": "20.18.0", "current": false, "default": true},
  {"version": "22.13.0", "current": true, "default": false}
]
```

### Empty State

```bash
$ vp env list
No Node.js versions installed.

Install a version with: vp env install <version>
```

## List-Remote Command

The `vp env list-remote` (alias `ls-remote`) command displays available Node.js versions from the registry.

### Usage

```bash
# List recent versions (default: last 10 major versions, ascending order)
$ vp env list-remote
v20.0.0
v20.1.0
...
v20.18.0 (Iron)
v22.0.0
...
v22.13.0 (Jod)
v24.0.0

# List only LTS versions
$ vp env list-remote --lts

# Filter by major version
$ vp env list-remote 20

# Show all versions
$ vp env list-remote --all

# Sort newest first
$ vp env list-remote --sort desc
```

### Flags

| Flag                 | Description                         |
| -------------------- | ----------------------------------- |
| `--lts`              | Show only LTS versions              |
| `--all`              | Show all versions (not just recent) |
| `--json`             | Output as JSON                      |
| `--sort <asc\|desc>` | Sorting order (default: asc)        |

### JSON Output

```bash
$ vp env list-remote --json
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

| Variable                        | Description                                                                                     | Default        |
| ------------------------------- | ----------------------------------------------------------------------------------------------- | -------------- |
| `VITE_PLUS_HOME`                | Base directory for bin and config                                                               | `~/.vite-plus` |
| `VITE_PLUS_NODE_VERSION`        | Session override for Node.js version (set by `vp env use`)                                      | unset          |
| `VITE_PLUS_LOG`                 | Log level: debug, info, warn, error                                                             | `warn`         |
| `VITE_PLUS_DEBUG_SHIM`          | Enable extra shim diagnostics                                                                   | unset          |
| `VITE_PLUS_BYPASS`              | PATH-style list of bin dirs to skip when finding system tools; set `=1` to bypass shim entirely | unset          |
| `VITE_PLUS_TOOL_RECURSION`      | **Internal**: Prevents shim recursion                                                           | unset          |
| `VITE_PLUS_ENV_USE_EVAL_ENABLE` | **Internal**: Set by shell wrappers to signal that `vp env use` output will be eval'd           | unset          |

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
│   ├── vp.exe       # Trampoline forwarding to current\bin\vp.exe
│   ├── node.exe     # Trampoline shim (sets VITE_PLUS_SHIM_TOOL=node)
│   ├── npm.exe      # Trampoline shim (sets VITE_PLUS_SHIM_TOOL=npm)
│   ├── npx.exe      # Trampoline shim (sets VITE_PLUS_SHIM_TOOL=npx)
│   └── tsc.exe      # Trampoline shim for global package
└── current\
    └── bin\
        ├── vp.exe       # The actual vp CLI binary
        └── vp-shim.exe  # Trampoline template (copied as shims)
```

### Trampoline Executables

Windows shims use lightweight trampoline `.exe` files (see [RFC: Trampoline EXE for Shims](./trampoline-exe-for-shims.md)). Each trampoline detects its tool name from its own filename, sets `VITE_PLUS_SHIM_TOOL`, and spawns `vp.exe`. This avoids the "Terminate batch job (Y/N)?" prompt from `.cmd` wrappers and works in all shells (cmd.exe, PowerShell, Git Bash) without needing separate wrapper formats.

#### Why Not Symlinks?

On Unix, shims are symlinks to the vp binary, which preserves argv[0] for tool detection. On Windows, we use explicit `vp env exec <tool>` calls instead of symlinks because:

1. **Admin privileges required**: Windows symlinks need admin rights or Developer Mode
2. **Unreliable Git Bash support**: Symlink emulation varies by Git for Windows version

Instead, trampoline `.exe` files are used. See [RFC: Trampoline EXE for Shims](./trampoline-exe-for-shims.md) for the full design.

**How it works**:

1. User runs `npm install`
2. Windows finds `~/.vite-plus/bin/npm.exe` in PATH
3. Trampoline sets `VITE_PLUS_SHIM_TOOL=npm` and spawns `vp.exe`
4. `vp env exec` command handles version resolution and execution

**Benefits of this approach**:

- Single `vp.exe` binary to update in `current\bin\`
- All shims are trivial `.cmd` text files and shell scripts (no binary copies)
- Consistent with Volta's Windows approach
- Clear, readable wrapper scripts
- Works in both cmd.exe/PowerShell and Git Bash

### Windows Installation (install.ps1)

The Windows installer (`install.ps1`) follows this flow:

1. Download and install `vp.exe` and `vp-shim.exe` to `~/.vite-plus/current/bin/`
2. Create `~/.vite-plus/bin/vp.exe` trampoline (copy of `vp-shim.exe`)
3. Create shim trampolines: `node.exe`, `npm.exe`, `npx.exe` (via `vp env setup`)
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
- windows-latest: Full integration tests with trampoline `.exe` shim validation

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
4. Implement `vp env setup` (Unix symlinks, Windows trampoline `.exe` shims)
5. Implement `vp env doctor` basic diagnostics
6. Add resolution cache (persists across upgrades with version field)
7. Implement `vp env default [version]` to set/show global default Node.js version
8. Implement `vp env on` and `vp env off` for shim mode control
9. Implement `vp env pin [version]` for per-directory version pinning
10. Implement `vp env unpin` as alias for `pin --unpin`
11. Implement `vp env list` (local) and `vp env list-remote` (remote) to show versions
12. Implement recursion prevention (`VITE_PLUS_TOOL_RECURSION`)
13. Implement `vp env exec --node <version>` command

### Phase 2: Full Tool Support (P1)

1. Add shims for `npm`, `npx`
2. Implement `vp env which`
3. Implement `vp env --current --json`
4. Enhanced doctor with conflict detection
5. Implement `vp install -g` / `vp remove -g` / `vp update -g` for managed global packages
6. Implement package metadata storage
7. Implement per-package binary shims
8. Implement `vp list -g` / `vp pm list -g` to list installed global packages
9. Implement `vp env install <VERSION>` to install Node.js versions
10. Implement `vp env uninstall <VERSION>` to uninstall Node.js versions
11. Implement per-binary config files (`bins/`) for conflict detection
12. Implement binary conflict detection (hard fail by default)
13. Implement `--force` flag for auto-uninstall on conflict
14. Implement two-phase uninstall with orphan recovery

### Phase 3: Polish (P2)

1. Implement `vp env --print` for session-only env
2. Add VITE_PLUS_BYPASS escape hatch
3. Improve error messages
4. Add IDE-specific setup guidance
5. Documentation

### Phase 4: Future Enhancements (P3)

1. NODE_PATH setup for shared package resolution

## Backward Compatibility

This is a new feature with no impact on existing functionality. The `vp` binary continues to work normally when invoked directly.

## Future Enhancements

1. **Multiple Runtime Support**: Extend shim architecture for other runtimes (Bun, Deno)
2. **SQLite Cache**: Replace JSON cache with SQLite for better performance at scale
3. **Shell Integration**: Provide shell hooks for prompt version display

## Design Decisions Summary

The following decisions have been made:

1. **VITE_PLUS_HOME Default Location**: `~/.vite-plus` - Simple, memorable path that's easy for users to find and configure.

2. **Windows Shim Strategy**: Trampoline `.exe` files that set `VITE_PLUS_SHIM_TOOL` and spawn `vp.exe` - Avoids "Terminate batch job?" prompt, works in all shells. See [RFC: Trampoline EXE for Shims](./trampoline-exe-for-shims.md).

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
