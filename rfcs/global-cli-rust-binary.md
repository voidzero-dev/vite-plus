# RFC: Global CLI Rust Binary

## Status

Implemented

## Background

Currently, the vite+ global CLI (`vite-plus-cli` in `packages/global`) uses Node.js as its entry point:

```
bin/vite (shell script) → src/index.ts (Node.js) → Rust bindings (NAPI)
```

This architecture requires users to have Node.js pre-installed before they can use the global CLI. While the core functionality is already implemented in Rust via NAPI bindings, the Node.js requirement creates friction for new users who want to try vite+.

### Current Pain Points

1. **Installation Prerequisite**: Users must install Node.js before using vite+
2. **Version Compatibility**: Different Node.js versions may cause compatibility issues
3. **Onboarding Friction**: New users cannot simply download and run the CLI
4. **Distribution Complexity**: Need to manage both npm package and native bindings

### Opportunity

The `vite_js_runtime` crate already provides robust Node.js download and management capabilities:

- Automatic Node.js version resolution and download
- Multi-platform support (Linux, macOS, Windows; x64, arm64)
- Intelligent caching with ETag support
- Hash verification for security
- Per-project version control via `devEngines.runtime` in package.json

By making the global CLI a Rust binary entry point:

1. **Users can download and run it immediately** without pre-installing Node.js
2. **Projects control their JS runtime version** via `devEngines.runtime` configuration
3. **Consistent development environments** across teams - everyone uses the same runtime version
4. **No system-wide Node.js conflicts** - each project can specify its required version

The core innovation is enhancing JS runtime management, not eliminating Node.js usage. The CLI will automatically download and manage Node.js to execute package managers and JS scripts.

## Goals

1. **Remove Node.js installation prerequisite**: Create a standalone Rust binary that users can download and run immediately, without needing to pre-install Node.js on their system
2. **Enhanced JS Runtime Management**: Use `vite_js_runtime` to automatically download, cache, and manage Node.js versions, enabling:
   - Automatic Node.js provisioning for package manager and CLI operations
   - Per-project runtime version control via `devEngines.runtime` in package.json
   - Consistent runtime versions across development environments
3. **Maintain current functionality**: All commands from `packages/global` continue to work via bundled JS scripts
4. **Maintain backward compatibility**: Existing command-line interface and behaviors remain unchanged
5. **Cross-platform distribution**: Support Linux, macOS, and Windows via platform-specific binaries

## Non-Goals

1. Replacing the local CLI (`packages/cli`) - that remains a Node.js package
2. Removing the NAPI bindings - they will coexist for the local CLI use case
3. Changing the command syntax or behavior
4. Supporting JavaScript-only execution mode (always uses managed runtime)

## User Stories

### Story 1: First-time User Installation

```bash
# Before (requires Node.js)
npm install -g vite-plus-cli
vite new my-app

# After (no Node.js required)
curl -fsSL https://viteplus.dev/install.sh | bash
# or
brew install vite-plus
# or download binary directly

vite new my-app  # Works immediately
```

### Story 2: Running Package Manager Commands

```bash
# User runs install command (no Node.js pre-installed on system)
vite install lodash

# CLI automatically:
# 1. Checks if managed Node.js is cached
# 2. Downloads Node.js 22.22.0 if not present
# 3. Detects workspace package manager (pnpm/npm/yarn)
# 4. Downloads package manager if needed
# 5. Executes: node /path/to/pnpm install lodash
```

**Note:** Package managers (pnpm, npm, yarn) are Node.js programs, so the CLI uses managed Node.js to run them. The key benefit is that users don't need to pre-install Node.js - the CLI handles it automatically.

### Story 3: Commands Requiring JavaScript Execution

```bash
# User runs a command that needs JS
vite new --template create-vite my-app

# CLI automatically:
# 1. Checks if managed Node.js is cached
# 2. Downloads Node.js 22.22.0 if not present
# 3. Executes create-vite using managed Node.js
```

## Technical Design

### New Crate: `vite_global_cli`

Create a new crate at `crates/vite_global_cli` that compiles to a standalone binary.

```
crates/
├── vite_global_cli/         # New crate
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs          # Entry point
│       ├── cli.rs           # CLI parsing (clap)
│       ├── commands/        # Command implementations
│       │   ├── mod.rs
│       │   ├── pm.rs        # Package manager commands
│       │   ├── new.rs       # Project scaffolding
│       │   ├── migrate.rs   # Migration command
│       │   └── ...
│       ├── js_executor.rs   # JS execution via vite_js_runtime
│       └── workspace.rs     # Workspace detection (reuse from vite_task)
├── vite_js_runtime/         # Existing - Node.js management
├── vite_task/               # Existing - Task execution
└── ...
```

### Command Categories

Based on the current global CLI analysis, commands fall into four categories:

#### Category A: Package Manager Commands (Rust CLI + Managed Node.js)

These commands wrap existing package managers (pnpm/npm/yarn), which are Node.js programs. The Rust CLI handles argument parsing and workspace detection, then uses managed Node.js to execute the actual package manager:

| Command               | Description          | Implementation                             |
| --------------------- | -------------------- | ------------------------------------------ |
| `install [packages]`  | Install dependencies | Rust CLI → Managed Node.js → pnpm/npm/yarn |
| `add <packages>`      | Add packages         | Rust CLI → Managed Node.js → pnpm/npm/yarn |
| `remove <packages>`   | Remove packages      | Rust CLI → Managed Node.js → pnpm/npm/yarn |
| `update [packages]`   | Update packages      | Rust CLI → Managed Node.js → pnpm/npm/yarn |
| `outdated [packages]` | Check outdated       | Rust CLI → Managed Node.js → pnpm/npm/yarn |
| `dedupe`              | Deduplicate deps     | Rust CLI → Managed Node.js → pnpm/npm/yarn |
| `why <package>`       | Explain dependency   | Rust CLI → Managed Node.js → pnpm/npm/yarn |
| `info <package>`      | View package info    | Rust CLI → Managed Node.js → pnpm/npm/yarn |
| `link [package]`      | Link packages        | Rust CLI → Managed Node.js → pnpm/npm/yarn |
| `unlink [package]`    | Unlink packages      | Rust CLI → Managed Node.js → pnpm/npm/yarn |
| `dlx <package>`       | Execute package      | Rust CLI → Managed Node.js → pnpm/npm dlx  |
| `pm <subcommand>`     | Forward to PM        | Rust CLI → Managed Node.js → pnpm/npm/yarn |

**Note:** Since pnpm, npm, and yarn are all Node.js programs, these commands require Node.js to execute. The global CLI will use `vite_js_runtime` to download and manage Node.js automatically when running any PM command.

#### Category B: JS Script Commands (Rust CLI + Managed Node.js + JS Scripts)

These commands execute JavaScript scripts bundled with the CLI:

| Command          | JS Dependency                        | Implementation                          |
| ---------------- | ------------------------------------ | --------------------------------------- |
| `new [template]` | Remote templates (create-vite, etc.) | Rust CLI → Managed Node.js → JS scripts |
| `migrate [path]` | Migration rules and transformations  | Rust CLI → Managed Node.js → JS scripts |
| `--version`      | Version display logic                | Rust CLI → Managed Node.js → JS scripts |

#### Category C: Local CLI Delegation (Rust CLI + Managed Node.js + JS Entry Point)

These commands delegate to the local `vite-plus` package through the JS entry point (`dist/index.js`), which handles detecting/installing local vite-plus:

| Command                                                          | Implementation                                           |
| ---------------------------------------------------------------- | -------------------------------------------------------- |
| `dev`, `build`, `test`, `lint`, `fmt`, `run`, `preview`, `cache` | Rust CLI → Managed Node.js → `dist/index.js` → local CLI |

**Note:** The global CLI uses `vite_js_runtime` to ensure Node.js is available, resolving the version from the project's `devEngines.runtime` configuration. The JS entry point handles detecting if vite-plus is installed locally, auto-installing if needed, and delegating to the local CLI's `dist/bin.js`.

#### Category D: Pure Rust Commands (No Node.js Required)

Only these commands can run without any Node.js:

| Command | Description | Implementation   |
| ------- | ----------- | ---------------- |
| `help`  | Show help   | Pure Rust (clap) |

**Note:** Even `help` might trigger Node.js download if the user runs `vite help new` and needs to display JS-specific help.

### Architecture

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                        vite_global_cli (Rust Binary)                         │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────────────┐   │
│  │   CLI Parser     │  │ Workspace Detect │  │   VITE_GLOBAL_CLI_JS_SCRIPTS_DIR│   │
│  │   (clap)         │  │ (from vite_task) │  │   (bundled scripts path) │   │
│  └────────┬─────────┘  └────────┬─────────┘  └────────────┬─────────────┘   │
│           │                     │                         │                 │
│  ┌────────▼─────────────────────▼─────────────────────────▼───────────────┐ │
│  │                          Command Router                                 │ │
│  └───┬──────────────────┬──────────────────┬──────────────────┬───────────┘ │
│      │                  │                  │                  │             │
│  ┌───▼────────────┐ ┌───▼────────────┐ ┌───▼────────────┐ ┌───▼──────────┐ │
│  │ Category A     │ │ Category B     │ │ Category C     │ │ Category D   │ │
│  │ PM Commands    │ │ JS Scripts     │ │ Delegation     │ │ Pure Rust    │ │
│  │ - install      │ │ - new          │ │ - dev          │ │ - help       │ │
│  │ - add          │ │ - migrate      │ │ - build        │ │              │ │
│  │ - remove       │ │ - --version    │ │ - test         │ │              │ │
│  │ - update       │ │                │ │ - lint         │ │              │ │
│  │ - ...          │ │                │ │ - ...          │ │              │ │
│  └───────┬────────┘ └───────┬────────┘ └───────┬────────┘ └──────────────┘ │
│          │                  │                  │                           │
└──────────┼──────────────────┼──────────────────┼───────────────────────────┘
           │                  │                  │
           ▼                  ▼                  ▼
┌─────────────────────────────────────┐    ┌────────────────────────────────┐
│    Flow 1: CLI Runtime              │    │    Flow 2: Project Runtime     │
│    (Categories A & B)               │    │    (Category C)                │
│                                     │    │                                │
│  download_runtime_for_project(      │    │  download_runtime_for_project( │
│    cli_package_json_dir             │    │    project_dir                 │
│  )                                  │    │  )                             │
│                                     │    │                                │
│  vite_js_runtime reads:             │    │  vite_js_runtime reads:        │
│  packages/global/package.json       │    │  <project>/package.json        │
│  └─> devEngines.runtime: "22.22.0"  │    │  └─> devEngines.runtime        │
│                                     │    │                                │
└─────────────┬───────────────────────┘    └─────────────┬──────────────────┘
              │                                          │
              ▼                                          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          vite_js_runtime crate                              │
│                                                                             │
│  Built-in logic (same for both flows):                                      │
│  1. Read package.json from provided path                                    │
│  2. Extract devEngines.runtime.version                                      │
│  3. Resolve semver range if needed                                          │
│  4. Check cache (~/.cache/vite/js_runtime/node/{version}/)                  │
│  5. Download Node.js if not cached                                          │
│  6. Return JsRuntime with binary path                                       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
              │                                          │
              ▼                                          ▼
┌─────────────────────────────────────┐    ┌────────────────────────────────┐
│    Managed Node.js                  │    │    Managed Node.js             │
│    (CLI's version: 22.22.0)         │    │    (Project's version)         │
│                                     │    │                                │
│  ┌─────────────┐  ┌──────────────┐  │    │  ┌──────────────────────────┐  │
│  │ pnpm/npm/   │  │ Bundled      │  │    │  │ dist/index.js            │  │
│  │ yarn        │  │ JS Scripts   │  │    │  │ → detects/installs local │  │
│  │ (Cat. A)    │  │ (Cat. B)     │  │    │  │ → delegates to local CLI │  │
│  └─────────────┘  └──────────────┘  │    │  └──────────────────────────┘  │
└─────────────────────────────────────┘    └────────────────────────────────┘

Legend:
- Both flows use download_runtime_for_project(), just with different directory paths
- vite_js_runtime handles all devEngines.runtime logic internally
- Category C delegates through dist/index.js which handles local CLI detection
- Category D: No Node.js required (pure Rust)
```

### JS Executor Module

When JavaScript execution is needed, the executor uses `download_runtime_for_project()` with different directory paths:

```rust
// crates/vite_global_cli/src/js_executor.rs

use vite_js_runtime::download_runtime_for_project;
use std::process::Command;

pub struct JsExecutor {
    cli_runtime: Option<JsRuntime>,      // Cached runtime for CLI commands
    project_runtime: Option<JsRuntime>,  // Cached runtime for project delegation
    scripts_dir: PathBuf,                // From VITE_GLOBAL_CLI_JS_SCRIPTS_DIR
}

impl JsExecutor {
    pub fn new(scripts_dir: PathBuf) -> Self {
        Self {
            cli_runtime: None,
            project_runtime: None,
            scripts_dir,
        }
    }

    /// Get the CLI's package.json directory (parent of scripts_dir)
    fn get_cli_package_dir(&self) -> PathBuf {
        self.scripts_dir.parent().unwrap().to_path_buf()
    }

    /// Get runtime for CLI's own commands (Categories A & B)
    /// Uses CLI's package.json devEngines.runtime (e.g., "22.22.0")
    pub async fn ensure_cli_runtime(&mut self) -> Result<&JsRuntime, Error> {
        if self.cli_runtime.is_none() {
            // download_runtime_for_project reads devEngines.runtime from
            // the package.json in the given directory
            let cli_dir = self.get_cli_package_dir();
            let runtime = download_runtime_for_project(&cli_dir).await?;
            self.cli_runtime = Some(runtime);
        }
        Ok(self.cli_runtime.as_ref().unwrap())
    }

    /// Get runtime for project delegation (Category C)
    /// Uses project's package.json devEngines.runtime
    pub async fn ensure_project_runtime(&mut self, project_path: &Path) -> Result<&JsRuntime, Error> {
        if self.project_runtime.is_none() {
            // download_runtime_for_project reads devEngines.runtime from
            // the project's package.json
            let runtime = download_runtime_for_project(project_path).await?;
            self.project_runtime = Some(runtime);
        }
        Ok(self.project_runtime.as_ref().unwrap())
    }

    /// Execute CLI's bundled JS script (Categories A & B)
    pub async fn execute_cli_script(&mut self, script_name: &str, args: &[&str]) -> Result<ExitStatus, Error> {
        let runtime = self.ensure_cli_runtime().await?;
        let script_path = self.scripts_dir.join(script_name);
        let status = Command::new(runtime.get_binary_path())
            .arg(&script_path)
            .args(args)
            .status()?;
        Ok(status)
    }

    /// Execute package manager command (Category A)
    pub async fn execute_pm_command(&mut self, pm: &str, args: &[&str]) -> Result<ExitStatus, Error> {
        let runtime = self.ensure_cli_runtime().await?;
        // PM binaries are in the same bin directory as node
        let pm_path = runtime.get_bin_prefix().join(pm);
        let status = Command::new(runtime.get_binary_path())
            .arg(&pm_path)
            .args(args)
            .status()?;
        Ok(status)
    }

    /// Delegate to local vite-plus CLI (Category C)
    ///
    /// Passes the command through `dist/index.js` which handles:
    /// - Detecting if vite-plus is installed locally
    /// - Auto-installing if it's a dependency but not installed
    /// - Prompting user to add it if not found
    /// - Delegating to the local CLI's `dist/bin.js`
    pub async fn delegate_to_local_cli(
        &mut self,
        project_path: &Path,
        args: &[&str]
    ) -> Result<ExitStatus, Error> {
        // Use project's runtime version via download_runtime_for_project
        let runtime = self.ensure_project_runtime(project_path).await?;

        // Get the JS entry point (dist/index.js)
        let entry_point = self.scripts_dir.join("index.js");

        // Execute dist/index.js with the command and args
        // The JS layer handles detecting/installing local vite-plus
        let status = Command::new(runtime.get_binary_path())
            .arg(&entry_point)
            .args(args)
            .current_dir(project_path)
            .status()?;
        Ok(status)
    }
}
```

**Key points:**

- Both flows use `download_runtime_for_project()` - the only difference is the directory path
- `vite_js_runtime` handles all `devEngines.runtime` logic internally (reading package.json, resolving versions, caching)
- CLI commands use CLI's package.json directory (e.g., `packages/global/`)
- Project delegation uses project's directory and passes commands through `dist/index.js`
- The JS entry point handles local CLI detection, auto-installation, and delegation

### Implementation Phases

#### Phase 1: Foundation & All Package Manager Commands

**Scope:**

- Set up `vite_global_cli` crate structure
- Implement CLI parsing with clap
- Implement workspace detection (reuse from `vite_task`)
- Implement package manager detection and wrapping
- Implement ALL package manager commands:
  - `install [packages]` / `i` - Install dependencies or add packages
  - `add <packages>` - Add packages to dependencies
  - `remove <packages>` / `rm`, `un`, `uninstall` - Remove packages
  - `update [packages]` / `up` - Update packages
  - `outdated [packages]` - Check for outdated packages
  - `dedupe` / `ddp` - Deduplicate dependencies
  - `why <package>` / `explain` - Explain why a package is installed
  - `info <package>` / `view`, `show` - View package info from registry
  - `link [package|dir]` / `ln` - Link packages
  - `unlink [package|dir]` - Unlink packages
  - `dlx <package>` - Execute package without installing
  - `pm <subcommand>` - Forward to package manager (list, prune, pack)

**Files to create:**

- `crates/vite_global_cli/Cargo.toml`
- `crates/vite_global_cli/src/main.rs`
- `crates/vite_global_cli/src/cli.rs`
- `crates/vite_global_cli/src/commands/mod.rs`
- `crates/vite_global_cli/src/commands/add.rs` # Add packages (struct-based: AddCommand)
- `crates/vite_global_cli/src/commands/install.rs` # Install dependencies (struct-based: InstallCommand)
- `crates/vite_global_cli/src/commands/remove.rs` # Remove packages (struct-based: RemoveCommand)
- `crates/vite_global_cli/src/commands/update.rs` # Update packages (struct-based: UpdateCommand)
- `crates/vite_global_cli/src/commands/dedupe.rs` # Deduplicate deps (struct-based: DedupeCommand)
- `crates/vite_global_cli/src/commands/outdated.rs` # Check outdated (struct-based: OutdatedCommand)
- `crates/vite_global_cli/src/commands/why.rs` # Explain dependency (struct-based: WhyCommand)
- `crates/vite_global_cli/src/commands/link.rs` # Link packages (struct-based: LinkCommand)
- `crates/vite_global_cli/src/commands/unlink.rs` # Unlink packages (struct-based: UnlinkCommand)
- `crates/vite_global_cli/src/commands/dlx.rs` # Execute package (struct-based: DlxCommand)
- `crates/vite_global_cli/src/commands/pm.rs` # PM subcommands (prune, pack, list, etc.)
- `crates/vite_global_cli/src/commands/new.rs` # Project scaffolding
- `crates/vite_global_cli/src/commands/migrate.rs` # Migration command
- `crates/vite_global_cli/src/commands/delegate.rs` # Local CLI delegation
- `crates/vite_global_cli/src/commands/version.rs` # Version display
- `crates/vite_global_cli/src/js_executor.rs`
- `crates/vite_global_cli/src/error.rs`

**Success Criteria:**

- [x] All PM commands work without pre-installed Node.js (uses managed Node.js)
- [x] Managed Node.js is downloaded automatically when first PM command runs
- [x] Auto-detects pnpm/npm/yarn in the project
- [x] Package manager is downloaded via managed Node.js if not available
- [x] All PM commands work identically to current Node.js CLI
- [x] `--help` documentation matches current CLI
- [x] Command aliases work correctly (i, rm, up, etc.)

#### Phase 2: Project Scaffolding

**Scope:**

- Implement `new` command for built-in templates (vite:monorepo, etc.)
- Implement JS executor for remote templates
- Integrate with `vite_js_runtime` for Node.js download

**Success Criteria:**

- [x] `vite new vite:monorepo` works without Node.js
- [x] `vite new create-vite` downloads Node.js and executes correctly

#### Phase 3: Migration & Remaining Commands

**Scope:**

- Implement `migrate` command
- Implement local CLI delegation
- Implement `--version` and help system

**Success Criteria:**

- [x] `vite migrate` works correctly
- [x] Local commands delegate properly
- [x] Full feature parity with Node.js CLI

#### Phase 4: Distribution & Testing

**Scope:**

- Set up cross-platform builds (Linux, macOS, Windows)
- Create installation scripts
- Add to Homebrew, cargo install, etc.
- Comprehensive testing

**Success Criteria:**

- [x] Binary available via multiple channels
- [x] Installation scripts work on all platforms
- [x] All snap tests pass

### Dependency Changes

**New dependencies for `vite_global_cli`:**

```toml
[dependencies]
vite_js_runtime = { path = "../vite_js_runtime" }
vite_shared = { path = "../vite_shared" }  # For cache dir, etc.
vite_path = { path = "../vite_path" }

clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
thiserror = "1"
```

### Configuration

The global CLI will use the same configuration locations as the current CLI:

- **Cache directory**: `~/.cache/vite/` (via `vite_shared::cache_dir`)
- **Node.js runtime**: `~/.cache/vite/js_runtime/node/{version}/`
- **Package manager**: Auto-detected from lockfile or package.json

### JS Runtime Version Management

There are **two distinct runtime resolution strategies** based on the command category:

#### Strategy 1: Global CLI Commands (Categories A & B)

For package manager commands, `new`, `migrate`, and `--version`, the runtime version comes from the **global CLI's own package.json** (`packages/global/package.json`):

```json
{
  "name": "vite-plus-cli",
  "devEngines": {
    "runtime": {
      "name": "node",
      "version": "22.22.0"
    }
  }
}
```

**Rationale:**

- These commands are part of the global CLI's functionality
- They should use a consistent, tested Node.js version
- The version can be updated with CLI releases
- Users don't need a project to run `vite new` or `vite install`

#### Strategy 2: Local CLI Delegation (Category C)

For commands delegated to local `vite-plus` (`dev`, `build`, `test`, `lint`, etc.), the runtime version comes from the **current project's package.json**:

```json
{
  "name": "my-project",
  "devEngines": {
    "runtime": {
      "name": "node",
      "version": "^20.18.0"
    }
  }
}
```

**Resolution order for Category C:**

1. Project's `devEngines.runtime` (if present)
2. Fallback to CLI's default version (from `packages/global/package.json`)

**Rationale:**

- Projects may require specific Node.js versions for their builds
- Team members need consistent runtime versions for reproducibility
- Different projects can use different Node.js versions

#### Summary Table

| Command Category | Runtime Source                        | Example Commands             |
| ---------------- | ------------------------------------- | ---------------------------- |
| A: PM Commands   | CLI's package.json                    | install, add, remove, update |
| B: JS Scripts    | CLI's package.json                    | new, migrate, --version      |
| C: Delegation    | Project's package.json → CLI fallback | dev, build, test, lint       |
| D: Pure Rust     | None                                  | help                         |

**Benefits:**

- **Separation of concerns**: CLI commands use CLI's runtime, project commands use project's runtime
- **Per-project control**: Each project specifies its required runtime version for builds
- **Team consistency**: All developers use the same runtime version for a project
- **No system conflicts**: Different projects can use different Node.js versions
- **Automatic provisioning**: Runtime is downloaded automatically if not cached

This integrates with the existing `vite_js_runtime` crate's capabilities (see [js-runtime RFC](./js-runtime.md)).

### Packaging & Distribution Strategy

Since `new` and `migrate` commands are still implemented via JS scripts, we need a hybrid distribution strategy that provides both the Rust binary and the JS scripts.

#### Platform-Specific npm Packages

Create platform-specific npm packages containing only the native binary:

| Package Name                               | Platform | Architecture          |
| ------------------------------------------ | -------- | --------------------- |
| `@voidzero-dev/vite-plus-cli-darwin-arm64` | macOS    | ARM64 (Apple Silicon) |
| `@voidzero-dev/vite-plus-cli-darwin-x64`   | macOS    | Intel x64             |
| `@voidzero-dev/vite-plus-cli-linux-arm64`  | Linux    | ARM64                 |
| `@voidzero-dev/vite-plus-cli-linux-x64`    | Linux    | Intel x64             |
| `@voidzero-dev/vite-plus-cli-win32-arm64`  | Windows  | ARM64                 |
| `@voidzero-dev/vite-plus-cli-win32-x64`    | Windows  | Intel x64             |

**Package structure:**

```
@voidzero-dev/vite-plus-cli-darwin-arm64/
├── package.json
└── vite                    # Native binary (no extension on Unix)

@voidzero-dev/vite-plus-cli-win32-x64/
├── package.json
└── vite.exe                # Native binary (Windows)
```

**Platform package.json:**

```json
{
  "name": "@voidzero-dev/vite-plus-cli-darwin-arm64",
  "version": "1.0.0",
  "os": ["darwin"],
  "cpu": ["arm64"],
  "main": "vite",
  "files": ["vite"]
}
```

#### Main npm Package (vite-plus-cli)

The main `vite-plus-cli` package uses `optionalDependencies` to install the correct platform binary:

```json
{
  "name": "vite-plus-cli",
  "version": "1.0.0",
  "bin": {
    "vite": "./bin/vite"
  },
  "optionalDependencies": {
    "@voidzero-dev/vite-plus-cli-darwin-arm64": "1.0.0",
    "@voidzero-dev/vite-plus-cli-darwin-x64": "1.0.0",
    "@voidzero-dev/vite-plus-cli-linux-arm64": "1.0.0",
    "@voidzero-dev/vite-plus-cli-linux-x64": "1.0.0",
    "@voidzero-dev/vite-plus-cli-win32-arm64": "1.0.0",
    "@voidzero-dev/vite-plus-cli-win32-x64": "1.0.0"
  }
}
```

**Binary resolution (`bin/vite`):**

The `bin/vite` script needs to be refactored to find and execute the Rust binary from `optionalDependencies`:

```javascript
#!/usr/bin/env node

import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);

// Platform to package mapping
const PLATFORMS = {
  'darwin-arm64': '@voidzero-dev/vite-plus-cli-darwin-arm64',
  'darwin-x64': '@voidzero-dev/vite-plus-cli-darwin-x64',
  'linux-arm64': '@voidzero-dev/vite-plus-cli-linux-arm64',
  'linux-x64': '@voidzero-dev/vite-plus-cli-linux-x64',
  'win32-arm64': '@voidzero-dev/vite-plus-cli-win32-arm64',
  'win32-x64': '@voidzero-dev/vite-plus-cli-win32-x64',
};

function getBinaryPath() {
  const binaryName = process.platform === 'win32' ? 'vp.exe' : 'vp';

  // 1. First check for local binary in same directory (local development)
  const localBinaryPath = join(__dirname, binaryName);
  if (existsSync(localBinaryPath)) {
    return localBinaryPath;
  }

  // 2. Find binary from platform-specific optionalDependency
  const platform = `${process.platform}-${process.arch}`;
  const packageName = PLATFORMS[platform];

  if (!packageName) {
    throw new Error(`Unsupported platform: ${platform}`);
  }

  // Try to find the binary in node_modules
  const binaryPath = join(__dirname, '..', 'node_modules', packageName, binaryName);

  if (existsSync(binaryPath)) {
    return binaryPath;
  }

  // Fallback: try require.resolve
  const packagePath = require.resolve(`${packageName}/package.json`);
  return join(dirname(packagePath), binaryName);
}

const binaryPath = getBinaryPath();
// Set VITE_GLOBAL_CLI_JS_SCRIPTS_DIR to point to dist/index.js location
const jsScriptsDir = join(__dirname, '..');

execFileSync(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
  env: {
    ...process.env,
    VITE_GLOBAL_CLI_JS_SCRIPTS_DIR: jsScriptsDir,
  },
});
```

**How it works:**

1. `bin/vite` finds the Rust binary (`vp`) from the platform-specific optional dependency
2. Sets `VITE_GLOBAL_CLI_JS_SCRIPTS_DIR` pointing to the package root (where `dist/index.js` is)
3. Executes the Rust binary with all arguments
4. The Rust binary uses the JS entry point at `$VITE_GLOBAL_CLI_JS_SCRIPTS_DIR/dist/index.js`

This ensures npm installation works the same way as standalone installation.

#### Standalone Installation (install.sh)

For users who prefer standalone installation without npm:

```bash
#!/bin/bash
# https://viteplus.dev/install.sh

set -e

VITE_VERSION="${VITE_VERSION:-latest}"
INSTALL_DIR="${VITE_INSTALL_DIR:-$HOME/.vite}"
BIN_DIR="$INSTALL_DIR/bin"
DIST_DIR="$INSTALL_DIR/dist"

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin) PLATFORM="darwin" ;;
  Linux)  PLATFORM="linux" ;;
  MINGW*|MSYS*|CYGWIN*) PLATFORM="win32" ;;
  *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64) ARCH="x64" ;;
  arm64|aarch64) ARCH="arm64" ;;
  *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

PACKAGE_NAME="@voidzero-dev/vite-plus-cli-${PLATFORM}-${ARCH}"

# Get version if "latest"
if [ "$VITE_VERSION" = "latest" ]; then
  VITE_VERSION=$(curl -s "https://registry.npmjs.org/vite-plus-cli/latest" | jq -r '.version')
fi

echo "Installing vite-plus-cli v${VITE_VERSION} for ${PLATFORM}-${ARCH}..."

# Create directories
mkdir -p "$BIN_DIR" "$DIST_DIR"

# Download and extract native binary from platform package
BINARY_URL="https://registry.npmjs.org/${PACKAGE_NAME}/-/vite-plus-cli-${PLATFORM}-${ARCH}-${VITE_VERSION}.tgz"
curl -sL "$BINARY_URL" | tar xz -C "$BIN_DIR" --strip-components=1 package/vite

# Download and extract JS bundle from main package
MAIN_URL="https://registry.npmjs.org/vite-plus-cli/-/vite-plus-cli-${VITE_VERSION}.tgz"
curl -sL "$MAIN_URL" | tar xz -C "$DIST_DIR" --strip-components=2 package/dist

# Make binary executable
chmod +x "$BIN_DIR/vite"

# Automatically add to PATH
add_to_path() {
  local shell_config="$1"
  local path_line="export PATH=\"$BIN_DIR:\$PATH\""

  if [ -f "$shell_config" ]; then
    if ! grep -q "$BIN_DIR" "$shell_config" 2>/dev/null; then
      echo "" >> "$shell_config"
      echo "# Added by vite-plus installer" >> "$shell_config"
      echo "$path_line" >> "$shell_config"
      return 0
    fi
  fi
  return 1
}

PATH_ADDED=false

# Detect shell and update appropriate config
case "$SHELL" in
  */zsh)
    if add_to_path "$HOME/.zshrc"; then
      PATH_ADDED=true
      SHELL_CONFIG=".zshrc"
    fi
    ;;
  */bash)
    # Try .bashrc first, then .bash_profile
    if add_to_path "$HOME/.bashrc"; then
      PATH_ADDED=true
      SHELL_CONFIG=".bashrc"
    elif add_to_path "$HOME/.bash_profile"; then
      PATH_ADDED=true
      SHELL_CONFIG=".bash_profile"
    fi
    ;;
  */fish)
    FISH_CONFIG="$HOME/.config/fish/config.fish"
    if [ -f "$FISH_CONFIG" ] && ! grep -q "$BIN_DIR" "$FISH_CONFIG" 2>/dev/null; then
      echo "" >> "$FISH_CONFIG"
      echo "# Added by vite-plus installer" >> "$FISH_CONFIG"
      echo "set -gx PATH $BIN_DIR \$PATH" >> "$FISH_CONFIG"
      PATH_ADDED=true
      SHELL_CONFIG="config.fish"
    fi
    ;;
esac

echo ""
echo "Vite+ installed successfully!"
echo ""

if [ "$PATH_ADDED" = true ]; then
  echo "PATH has been updated in ~/$SHELL_CONFIG"
  echo "Run 'source ~/$SHELL_CONFIG' or restart your terminal to use vite."
else
  echo "Could not automatically update PATH. Please add the following to your shell profile:"
  echo "  export PATH=\"$BIN_DIR:\$PATH\""
fi
```

#### Windows Installation (install.ps1)

For Windows users, provide a PowerShell script:

```powershell
# https://viteplus.dev/install.ps1

$ErrorActionPreference = "Stop"

$ViteVersion = if ($env:VITE_VERSION) { $env:VITE_VERSION } else { "latest" }
$InstallDir = if ($env:VITE_INSTALL_DIR) { $env:VITE_INSTALL_DIR } else { "$env:USERPROFILE\.vite" }
$BinDir = "$InstallDir\bin"
$DistDir = "$InstallDir\dist"

# Detect architecture
$Arch = if ([Environment]::Is64BitOperatingSystem) {
    if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" }
} else {
    throw "32-bit Windows is not supported"
}

$PackageName = "@voidzero-dev/vite-plus-cli-win32-$Arch"

# Get version if "latest"
if ($ViteVersion -eq "latest") {
    $ViteVersion = (Invoke-RestMethod "https://registry.npmjs.org/vite-plus-cli/latest").version
}

Write-Host "Installing vite-plus-cli v$ViteVersion for win32-$Arch..."

# Create directories
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
New-Item -ItemType Directory -Force -Path $DistDir | Out-Null

# Download and extract native binary
$BinaryUrl = "https://registry.npmjs.org/$PackageName/-/vite-plus-cli-win32-$Arch-$ViteVersion.tgz"
$TempFile = New-TemporaryFile
Invoke-WebRequest -Uri $BinaryUrl -OutFile $TempFile
tar -xzf $TempFile -C $BinDir --strip-components=1 "package/vp.exe"
Remove-Item $TempFile

# Download and extract JS bundle
$MainUrl = "https://registry.npmjs.org/vite-plus-cli/-/vite-plus-cli-$ViteVersion.tgz"
$TempFile = New-TemporaryFile
Invoke-WebRequest -Uri $MainUrl -OutFile $TempFile
tar -xzf $TempFile -C $DistDir --strip-components=2 "package/dist"
Remove-Item $TempFile

# Add to user PATH
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$BinDir*") {
    $NewPath = "$BinDir;$UserPath"
    [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
    $env:Path = "$BinDir;$env:Path"
    $PathAdded = $true
} else {
    $PathAdded = $false
}

Write-Host ""
Write-Host "Vite+ installed successfully!"
Write-Host ""

if ($PathAdded) {
    Write-Host "PATH has been updated. Restart your terminal to use vite."
} else {
    Write-Host "PATH already contains $BinDir"
}
```

**Windows installation options:**

1. **PowerShell one-liner:**

   ```powershell
   irm https://viteplus.dev/install.ps1 | iex
   ```

2. **npm (if Node.js is available):**

   ```cmd
   npm install -g vite-plus-cli
   ```

3. **Scoop (future):**
   ```cmd
   scoop install vite-plus
   ```

#### Directory Layout for Standalone Installation

```
~/.vite/
├── bin/
│   └── vite           # Native Rust binary
└── dist/
    └── index.js       # Bundled JS entry point (all commands)
```

#### How the Rust Binary Uses JS Scripts

When the Rust binary needs to execute JS (for `new`, `migrate`, `--version`, or PM commands):

1. Check `VITE_GLOBAL_CLI_JS_SCRIPTS_DIR` environment variable (optional)
2. If not set, auto-detect by looking for `dist/index.js` relative to the binary
3. Download Node.js via `vite_js_runtime` if not cached
4. Execute the JS entry point with managed Node.js, passing command and arguments

**Auto-detection logic:**

- For npm installation: binary is in `node_modules/vite-plus-cli/bin/`, JS entry point is `node_modules/vite-plus-cli/dist/index.js`
- For standalone installation: binary is in `~/.vite/bin/`, JS entry point is `~/.vite/dist/index.js`
- For local development: binary is in `packages/global/bin/`, JS entry point is `packages/global/dist/index.js`

```rust
// In the Rust binary
fn get_js_entry_point() -> Result<PathBuf, Error> {
    // 1. Check environment variable first
    if let Ok(dir) = std::env::var("VITE_GLOBAL_CLI_JS_SCRIPTS_DIR") {
        return Ok(PathBuf::from(dir).join("dist/index.js"));
    }

    // 2. Auto-detect based on binary location
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent().ok_or(Error::JsEntryPointNotFound)?;

    // JS entry point is always at ../dist/index.js relative to bin/
    let entry_point = exe_dir.join("../dist/index.js");

    if entry_point.exists() {
        return Ok(entry_point.canonicalize()?);
    }

    Err(Error::JsEntryPointNotFound)
}

async fn run_js_command(&self, command: &str, args: &[&str]) -> Result<(), Error> {
    let entry_point = get_js_entry_point()?;

    // Ensure Node.js is available
    let runtime = self.js_executor.ensure_cli_runtime().await?;

    // Execute JS entry point with command and arguments
    // The JS entry point handles routing to the appropriate handler
    let status = Command::new(runtime.get_binary_path())
        .arg(&entry_point)
        .arg(command)  // e.g., "new", "migrate", "--version"
        .args(args)
        .status()?;

    Ok(())
}
```

#### Build & Publish Workflow

The existing `packages/global/publish-native-addons.ts` script already publishes platform-specific packages via `@napi-rs/cli`. We only need to modify it to also include the Rust binary.

**Current artifact structure** (see [@voidzero-dev/vite-plus-cli-darwin-arm64 on unpkg](https://app.unpkg.com/@voidzero-dev/vite-plus-cli-darwin-arm64)):

```
@voidzero-dev/vite-plus-cli-darwin-arm64/
├── package.json
├── vite-plus-cli.darwin-arm64.node  # NAPI binding (existing)
└── vp                                # Rust binary (to be added)
```

**Changes to `publish-native-addons.ts`:**

1. Before publishing, copy the compiled Rust binary to each platform's directory
2. Add the binary to the package's `files` array
3. Publish as usual

```typescript
// packages/global/publish-native-addons.ts

// ... existing code ...

// NEW: Copy Rust binary to platform package before publishing
const rustBinaryName = platform === 'win32' ? 'vp.exe' : 'vp';
const rustBinarySource = `../../target/${rustTarget}/release/${rustBinaryName}`;
const rustBinaryDest = `npm/${platform}-${arch}/${rustBinaryName}`;

if (fs.existsSync(rustBinarySource)) {
  fs.copyFileSync(rustBinarySource, rustBinaryDest);
  console.log(`Copied Rust binary to ${rustBinaryDest}`);
}

// ... existing publish code ...
```

**Rust binary targets:**

| Platform Package | Rust Target                 |
| ---------------- | --------------------------- |
| darwin-arm64     | `aarch64-apple-darwin`      |
| darwin-x64       | `x86_64-apple-darwin`       |
| linux-arm64      | `aarch64-unknown-linux-gnu` |
| linux-x64        | `x86_64-unknown-linux-gnu`  |
| win32-arm64      | `aarch64-pc-windows-msvc`   |
| win32-x64        | `x86_64-pc-windows-msvc`    |

**CI/CD Integration:**

The existing CI workflow builds NAPI bindings for all platforms. We add a step to also build the Rust binary:

```yaml
# In existing CI workflow
- name: Build Rust CLI
  run: cargo build --release --target ${{ matrix.target }} -p vite_global_cli
```

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("No package manager detected. Please run in a project directory.")]
    NoPackageManager,

    #[error("Failed to download Node.js runtime: {0}")]
    RuntimeDownload(#[from] vite_js_runtime::Error),

    #[error("Command execution failed: {0}")]
    CommandExecution(std::io::Error),

    // ... more variants
}
```

**Note:** Local CLI detection errors are handled by the JS layer (`dist/index.js`), which provides better UX with auto-install prompts and user-friendly messages.

### Local Development

During local development, the Rust binary needs to be available alongside the JS scripts in `packages/global/`.

**Installation script:**

The script `packages/tools/src/install-global-cli.ts` handles copying the compiled Rust binary to the correct location:

```
packages/global/
├── bin/
│   └── vp              # Rust binary copied here by install-global-cli.ts
├── src/
│   ├── new/
│   ├── migration/
│   ├── version.ts
│   └── ...
└── package.json        # Contains devEngines.runtime: "22.22.0"
```

**Development workflow:**

1. Build the Rust binary: `cargo build -p vite_global_cli`
2. Build JS: `pnpm -F vite-plus-cli build`
3. Run install script: `pnpm bootstrap-cli` (which internally runs `install-global-cli.ts`)
4. The script copies the binary to `packages/global/bin/vp`
5. The script updates `package.json` bin entry: `{ "vp": "./bin/vp" }`
6. Local development and snap tests work unchanged

**Directory structure after setup:**

```
packages/global/
├── bin/
│   └── vp              # Rust binary copied here
├── dist/
│   └── index.js        # Bundled JS entry point
└── package.json        # Contains devEngines.runtime: "22.22.0"
```

**Implementation note for `install-global-cli.ts`:**

```typescript
// Update package.json bin entry to point to Rust binary
packageJson.bin = { vp: './bin/vp' };
```

**Benefits:**

- Consistent experience with production
- Snap tests run against the actual Rust binary
- Auto-detection finds `dist/index.js` relative to binary location
- No wrapper scripts or environment variables needed

### Testing Strategy

**Unit Tests:**

- CLI argument parsing
- Workspace detection
- Command routing

**Integration Tests:**

- Full command execution in test fixtures
- Cross-platform behavior
- JS executor with real Node.js download

**Snap Tests:**

- Reuse existing snap test infrastructure
- Add new tests for Rust binary behavior
- Tests run against the Rust binary in `packages/global/bin/vp`

```rust
#[test]
fn test_install_command_parsing() {
    let args = cli::parse(&["vite", "install", "lodash", "--save-dev"]);
    assert!(matches!(args.command, Command::Install { .. }));
}

#[tokio::test]
async fn test_js_executor_downloads_node() {
    let mut executor = JsExecutor::new();
    let runtime = executor.ensure_runtime().await.unwrap();
    assert!(runtime.get_binary_path().exists());
}
```

## Design Decisions

### 1. Why Node.js 22.22.0 as Default?

Node.js 22 is the current LTS line with long-term support. Version 22.22.0 is chosen as a stable point release.

**Configuration approach:**

- Default version is configured in `packages/global/package.json` via `devEngines.runtime`
- Can be updated in future releases without rebuilding the Rust binary
- Projects can override via their own `devEngines.runtime` configuration

**Version resolution priority:**

1. Project's `devEngines.runtime` (if present)
2. CLI's default from bundled `package.json`

### 2. Why Not Bundle Node.js?

Bundling Node.js would significantly increase binary size (~100MB+). Instead, downloading on-demand:

- Keeps initial download small (~20MB)
- Allows version flexibility
- Leverages existing `vite_js_runtime` caching

### 3. Why Wrap Package Managers Instead of Reimplementing?

Reimplementing pnpm/npm/yarn would be a massive undertaking with subtle compatibility issues. Wrapping existing package managers:

- Ensures compatibility
- Reduces maintenance burden
- Allows users to use their preferred PM

### 4. Why Keep NAPI Bindings?

The NAPI bindings serve the local CLI (`vite-plus` package) use case where Node.js is already available. This allows the same Rust code to be used in both:

- Standalone binary (for global CLI)
- Node.js addon (for local CLI performance)

### 5. Why Platform-Specific npm Packages?

This approach (used by esbuild, swc, rolldown, etc.) provides several benefits:

- **npm compatibility**: Users can still `npm install -g vite-plus-cli`
- **Automatic platform detection**: npm handles installing the correct binary
- **Dual-use distribution**: Same binaries work for both npm and standalone installation
- **No binary in main package**: Main package stays small, only platform-specific binaries are downloaded
- **CDN distribution**: Unpkg/jsdelivr can serve binaries directly

### 6. Why Keep JS Scripts for `new` and `migrate`?

These commands involve:

- Complex template rendering with user prompts (@clack/prompts)
- Remote template downloads and execution (create-vite, etc.)
- Code transformation rules that may change frequently
- Integration with the existing vite-plus ecosystem

Rewriting these in Rust would be significant effort with limited benefit. Instead:

- JS scripts continue to work as-is
- Rust binary invokes them via managed Node.js runtime
- Updates to templates/migrations don't require binary rebuilds

## Migration Path

### For Existing Users

1. Users with `vite-plus-cli` via npm continue to work
2. New installation methods become available (brew, curl, cargo)
3. Eventual deprecation of npm-based global CLI (with ample warning period)

### For CI/CD

```yaml
# Before
- run: npm install -g vite-plus-cli

# After (recommended)
- run: curl -fsSL https://viteplus.dev/install.sh | bash
# or
- uses: voidzero-dev/setup-vite-plus-action@v1
```

## Future Enhancements

- [ ] Support Bun/Deno as alternative JS runtimes
- [ ] Self-update command (`vite upgrade`)
- [ ] Plugin system for custom commands
- [ ] Shell completions generation
- [ ] Offline mode with cached templates

## Success Criteria

1. [x] Binary runs on Linux, macOS, and Windows without pre-installed Node.js
2. [x] Managed Node.js is downloaded automatically when needed (PM commands, new, migrate)
3. [x] All current commands work identically to the existing Node.js CLI
4. [x] Cold start time < 100ms (excluding Node.js/PM download)
5. [x] Binary size < 30MB
6. [x] Existing snap tests pass
7. [x] Platform-specific npm packages published and installable
8. [x] `npm install -g vite-plus-cli` works on all supported platforms
9. [x] Standalone installation via `curl | bash` works
10. [x] JS scripts for `new` and `migrate` correctly bundled and executed

## References

- [vite_js_runtime RFC](./js-runtime.md)
- [split-global-cli RFC](./split-global-cli.md)
- [install-command RFC](./install-command.md)
- [Node.js Releases](https://nodejs.org/en/about/releases/)
