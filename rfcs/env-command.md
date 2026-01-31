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
vp env --setup

# Force refresh shims (after vp binary upgrade)
vp env --setup --refresh

# Set the global default Node.js version (used when no project version file exists)
vp env default 20.18.0
vp env default lts        # Use latest LTS version
vp env default latest     # Use latest version (not recommended for stability)

# Show current default version
vp env default
```

### Diagnostic Commands

```bash
# Comprehensive system diagnostics
vp env --doctor

# Show which node binary would be executed in current directory
vp env --which node
vp env --which npm

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

### config.json Format (JSONC - JSON with Comments)

```jsonc
// ~/.vite-plus/config.json

{
  // Default Node.js version when no project version file is found
  // Set via: vp env default <version>
  "defaultNodeVersion": "20.18.0",

  // Alternatively, use aliases:
  // "defaultNodeVersion": "lts"     // Always use latest LTS
  // "defaultNodeVersion": "latest"  // Always use latest (not recommended)
}
```

**Note**: Node.js binaries continue to use existing cache location:

- Linux: `~/.cache/vite-plus/js_runtime/node/{version}/`
- macOS: `~/Library/Caches/vite-plus/js_runtime/node/{version}/`
- Windows: `%LOCALAPPDATA%\vite-plus\js_runtime\node\{version}\`

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
│           ├── setup.rs              # --setup implementation
│           ├── doctor.rs             # --doctor implementation
│           ├── which.rs              # --which implementation
│           └── current.rs            # --current implementation
```

### Command Definition

```rust
// crates/vite_global_cli/src/cli.rs

#[derive(Subcommand, Debug)]
pub enum Commands {
    // ... existing commands ...

    /// Manage Node.js environment and shims
    Env(EnvArgs),
}

#[derive(Args, Debug)]
pub struct EnvArgs {
    /// Create or update shims in VITE_PLUS_HOME/shims
    #[arg(long)]
    pub setup: bool,

    /// Force refresh shims even if they exist
    #[arg(long, requires = "setup")]
    pub refresh: bool,

    /// Run diagnostics and show environment status
    #[arg(long)]
    pub doctor: bool,

    /// Show path to the tool that would be executed
    #[arg(long, value_name = "TOOL")]
    pub which: Option<String>,

    /// Show current environment information
    #[arg(long)]
    pub current: bool,

    /// Output in JSON format
    #[arg(long, requires = "current")]
    pub json: bool,

    /// Print shell snippet to set environment for current session
    #[arg(long)]
    pub print: bool,

    /// Set or show the global default Node.js version
    /// Usage: vp env default [VERSION]
    /// Examples:
    ///   vp env default           # Show current default
    ///   vp env default 20.18.0   # Set specific version
    ///   vp env default lts       # Set to latest LTS
    ///   vp env default latest    # Set to latest version
    #[arg(long, value_name = "VERSION")]
    pub default: Option<Option<String>>,
}
```

### Shim Detection in main.rs

```rust
// crates/vite_global_cli/src/main.rs

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let argv0 = args.first().map(|s| s.as_str()).unwrap_or("vp");

    // Check VITE_PLUS_SHIM_TOOL first (set by Windows .cmd wrappers)
    // Then fall back to argv[0] detection
    let tool = std::env::var("VITE_PLUS_SHIM_TOOL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| extract_tool_name(argv0));

    match tool.as_str() {
        "node" | "npm" | "npx" => {
            // Shim mode
            let exit_code = shim::dispatch(&tool, &args[1..]);
            std::process::exit(exit_code);
        }
        _ => {
            // Normal CLI mode
            run_cli();
        }
    }
}

fn extract_tool_name(argv0: &str) -> String {
    let path = std::path::Path::new(argv0);
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();

    // Handle Windows: strip .exe, .cmd extensions
    stem.trim_end_matches(".exe")
        .trim_end_matches(".cmd")
        .to_lowercase()
}
```

### Shim Dispatch Logic

```rust
// crates/vite_global_cli/src/shim/dispatch.rs

pub fn dispatch(tool: &str, args: &[String]) -> i32 {
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let cwd = AbsolutePathBuf::new(cwd).expect("Invalid current directory");

    // 1. Check bypass mode
    if std::env::var("VITE_PLUS_BYPASS").is_ok() {
        return bypass_to_system(tool, args);
    }

    // 2. Resolve version (with caching)
    let resolution = match resolve_with_cache(&cwd) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("vp: Failed to resolve Node version: {}", e);
            eprintln!("vp: Run 'vp env --doctor' for diagnostics");
            return 1;
        }
    };

    // 3. Ensure Node.js is installed
    if let Err(e) = ensure_installed(&resolution.version) {
        eprintln!("vp: Failed to install Node {}: {}", resolution.version, e);
        return 1;
    }

    // 4. Locate tool binary
    let tool_path = match locate_tool(&resolution.version, tool) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("vp: Tool '{}' not found: {}", tool, e);
            return 1;
        }
    };

    // 5. Prepare environment for recursive invocations
    // Prepend real node bin dir to PATH so child processes (e.g., npm running node)
    // use the correct version without going through shims again
    let node_bin_dir = tool_path.parent().expect("Tool has no parent directory");
    prepend_path_env(node_bin_dir);

    // Optional: set diagnostic env vars
    if std::env::var("VITE_PLUS_DEBUG_SHIM").is_ok() {
        std::env::set_var("VITE_PLUS_ACTIVE_NODE", &resolution.version);
        std::env::set_var("VITE_PLUS_RESOLVE_SOURCE", &resolution.source);
    }

    // 6. Execute - child processes will see real node in PATH
    exec::exec_tool(&tool_path, args)
}
```

### Platform-Specific Execution

```rust
// crates/vite_global_cli/src/shim/exec.rs

#[cfg(unix)]
pub fn exec_tool(path: &AbsolutePath, args: &[String]) -> i32 {
    use std::os::unix::process::CommandExt;

    let mut cmd = std::process::Command::new(path.as_path());
    cmd.args(args);

    // Use exec to replace process (preserves PID, signals)
    let err = cmd.exec();
    eprintln!("vp: Failed to exec {}: {}", path, err);
    1
}

#[cfg(windows)]
pub fn exec_tool(path: &AbsolutePath, args: &[String]) -> i32 {
    use std::process::Command;

    let status = Command::new(path.as_path())
        .args(args)
        .status()
        .expect("Failed to execute tool");

    status.code().unwrap_or(1)
}
```

### Resolution Cache

```rust
// crates/vite_global_cli/src/shim/cache.rs

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ResolveCacheEntry {
    pub version: String,
    pub source: String,
    pub project_root: String,
    pub resolved_at: u64,
    pub version_file_mtime: u64,
}

#[derive(Serialize, Deserialize)]
pub struct ResolveCache {
    /// Cache format version for upgrade compatibility
    version: u32,
    entries: HashMap<String, ResolveCacheEntry>, // key = cwd
    #[serde(skip)]
    max_entries: usize,
}

impl Default for ResolveCache {
    fn default() -> Self {
        Self {
            version: 1,
            entries: HashMap::new(),
            max_entries: Self::DEFAULT_MAX_ENTRIES,
        }
    }
}

impl ResolveCache {
    const DEFAULT_MAX_ENTRIES: usize = 4096;

    pub fn get(&self, cwd: &AbsolutePath) -> Option<&ResolveCacheEntry> {
        let key = cwd.to_string();
        let entry = self.entries.get(&key)?;

        // Validate mtime of version source file
        if !self.is_entry_valid(entry) {
            return None;
        }

        Some(entry)
    }

    pub fn insert(&mut self, cwd: &AbsolutePath, entry: ResolveCacheEntry) {
        // LRU eviction if needed
        if self.entries.len() >= self.max_entries {
            self.evict_oldest();
        }
        self.entries.insert(cwd.to_string(), entry);
    }

    fn is_entry_valid(&self, entry: &ResolveCacheEntry) -> bool {
        // Check if source file mtime has changed
        let source_path = std::path::Path::new(&entry.source);
        if let Ok(metadata) = std::fs::metadata(source_path) {
            if let Ok(mtime) = metadata.modified() {
                let mtime_secs = mtime.duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                return mtime_secs == entry.version_file_mtime;
            }
        }
        false
    }
}
```

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
$ vp env --doctor

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

Restart your terminal and IDE, then run 'vp env --doctor' to verify.
```

**Important**: The shims PATH (`~/.vite-plus/shims`) must be **before** the CLI bin PATH (`~/.vite-plus/current/bin`) if both are configured, so that `node` resolves to the shim first.

### Manual Setup

If user declines or needs to reconfigure:

```bash
$ vp env --setup

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

Restart your terminal and IDE, then run 'vp env --doctor' to verify.
```

### Doctor Output (Healthy)

```bash
$ vp env --doctor

VP Environment Doctor
=====================

VITE_PLUS_HOME: /Users/user/.vite-plus
  ✓ Directory exists
  ✓ Shims directory exists
  ✓ All shims present (node, npm, npx)

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

### Which Command

```bash
$ vp env --which node
/Users/user/.cache/vite-plus/js_runtime/node/20.18.0/bin/node

$ vp env --which npm
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

```powershell
# packages/global/install.ps1

Write-Host "Installing vite-plus..."

# Download and install vp.exe
# ... download logic ...

# Create shims
& "$env:USERPROFILE\.vite-plus\bin\vp.exe" env --setup

# Prompt for PATH configuration
$addPath = Read-Host "Would you like to add vite-plus shims to your PATH? (y/n)"
if ($addPath -eq 'y') {
    $shimPath = "$env:USERPROFILE\.vite-plus\shims"
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")

    if ($currentPath -notlike "*$shimPath*") {
        [Environment]::SetEnvironmentVariable("Path", "$shimPath;$currentPath", "User")
        Write-Host "Added to User PATH. Restart your terminal and IDE."
    } else {
        Write-Host "Already in PATH, skipping."
    }
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tool_name() {
        assert_eq!(extract_tool_name("node"), "node");
        assert_eq!(extract_tool_name("/usr/bin/node"), "node");
        assert_eq!(extract_tool_name("C:\\shims\\node.exe"), "node");
        assert_eq!(extract_tool_name("npm.cmd"), "npm");
        assert_eq!(extract_tool_name("/path/to/vp"), "vp");
    }

    #[test]
    fn test_cache_invalidation() {
        // Test mtime-based cache invalidation
    }

    #[test]
    fn test_path_prepend() {
        // Test PATH environment variable manipulation
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_shim_dispatch_node() {
    // Setup: Create test project with .node-version
    // Run: Invoke shim as 'node -v'
    // Verify: Output matches resolved version
}

#[tokio::test]
async fn test_shim_concurrent_install() {
    // Setup: Create scenario requiring Node download
    // Run: Invoke 10 concurrent 'node -v' commands
    // Verify: All succeed, only one download occurs
}

#[tokio::test]
async fn test_doctor_detects_path_issues() {
    // Setup: Environment without shims in PATH
    // Run: vp env --doctor
    // Verify: Correct diagnostic output
}
```

### Snap Tests

Add snap tests in `packages/global/snap-tests/`:

```
env-setup/
├── package.json
├── steps.json      # [{"command": "vp env --setup"}]
└── snap.txt

env-doctor/
├── package.json
├── .node-version   # "20.18.0"
├── steps.json      # [{"command": "vp env --doctor"}]
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
4. Implement `vp env --setup` (Unix hardlinks, Windows .exe copy + .cmd wrappers)
5. Implement `vp env --doctor` basic diagnostics
6. Add resolution cache (persists across upgrades with version field)
7. Implement `vp env default [version]` to set/show global default Node.js version

### Phase 2: Full Tool Support (P1)

1. Add shims for `npm`, `npx`
2. Implement `vp env --which`
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
- ✅ Leverages existing version resolution and installation infrastructure
