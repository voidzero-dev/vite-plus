# RFC: Vite+ Why Package Command

## Summary

Add `vite why` (alias: `vite explain`) command that automatically adapts to the detected package manager (pnpm/npm/yarn) for showing all packages that depend on a specified package. This helps developers understand dependency relationships, audit package usage, and debug dependency tree issues.

## Motivation

Currently, developers must manually use package manager-specific commands to understand why a package is installed:

```bash
pnpm why <package>
npm explain <package>
yarn why <package>
```

This creates friction in dependency analysis workflows and requires remembering different syntaxes. A unified interface would:

1. **Simplify dependency analysis**: One command works across all package managers
2. **Auto-detection**: Automatically uses the correct package manager
3. **Consistency**: Same syntax regardless of underlying tool
4. **Integration**: Works seamlessly with existing Vite+ features

### Current Pain Points

```bash
# Developer needs to know which package manager is used
pnpm why react                    # pnpm project
npm explain react                 # npm project (different command name)
yarn why react                    # yarn project

# Different output formats
pnpm why react --json             # pnpm - JSON output
npm explain react --json          # npm - JSON output
yarn why react                    # yarn - custom format

# Different workspace targeting
pnpm why react --filter app       # pnpm - filter workspaces
npm explain react --workspace app # npm - specify workspace
yarn why react                    # yarn - no workspace filtering
```

### Proposed Solution

```bash
# Works for all package managers
vite why <package>                # Show why package is installed
vite explain <package>            # Alias (matches npm)

# Output formats
vite why react --json             # JSON output
vite why react --long             # Verbose output
vite why react --parseable        # Parseable format

# Workspace operations
vite why react --filter app       # Check in specific workspace (pnpm)
vite why react -r                 # Check recursively across workspaces

# Dependency type filtering
vite why react --prod             # Only production dependencies
vite why react --dev              # Only dev dependencies
vite why react --depth 2          # Limit tree depth
```

## Proposed Solution

### Command Syntax

#### Why Command

```bash
vite why <PACKAGE> [OPTIONS]
vite explain <PACKAGE> [OPTIONS]        # Alias
```

**Examples:**

```bash
# Basic usage
vite why react
vite explain lodash

# Multiple packages (pnpm style)
vite why react react-dom
vite why "babel-*" "eslint-*"

# Output formats
vite why react --json             # JSON output
vite why react --long             # Verbose output
vite why react --parseable        # Parseable output

# Workspace operations
vite why react -r                 # Recursive across all workspaces
vite why react --filter app       # Check in specific workspace (pnpm)

# Dependency type filtering
vite why react --prod             # Only production dependencies
vite why react --dev              # Only dev dependencies
vite why react --no-optional      # Exclude optional dependencies

# Depth control
vite why react --depth 3          # Limit tree depth to 3 levels

# Global packages
vite why typescript -g            # Check globally installed packages

# Custom finder (pnpm only)
vite why react --find-by myFinder # Use finder function from .pnpmfile.cjs
```

### Global Packages Checking

Only use `npm` to check globally installed packages, because `vite install -g` uses `npm` cli to install global packages.

```bash
vite why typescript -g            # Check globally installed packages

-> npm why typescript -g
```

### Command Mapping

#### Why Command Mapping

**pnpm references:**

- https://pnpm.io/cli/why
- Shows all packages that depend on the specified package

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-explain
- Explains why a package is installed (alias: `npm why`)

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/why (yarn@1)
- https://yarnpkg.com/cli/why (yarn@2+)
- Identifies why a package has been installed

| Vite+ Flag                | pnpm                      | npm                     | yarn@1              | yarn@2+                  | Description                                                     |
| ------------------------- | ------------------------- | ----------------------- | ------------------- | ------------------------ | --------------------------------------------------------------- |
| `vite why <pkg>`          | `pnpm why <pkg>`          | `npm explain <pkg>`     | `yarn why <pkg>`    | `yarn why <pkg> --peers` | Show why package is installed                                   |
| `--json`                  | `--json`                  | `--json`                | `--json`            | `--json`                 | JSON output format                                              |
| `--long`                  | `--long`                  | N/A                     | N/A                 | N/A                      | Verbose output (pnpm only)                                      |
| `--parseable`             | `--parseable`             | N/A                     | N/A                 | N/A                      | Parseable format (pnpm only)                                    |
| `-r, --recursive`         | `-r, --recursive`         | N/A                     | N/A                 | `--recursive`            | Check across all workspaces                                     |
| `--filter <pattern>`      | `--filter <pattern>`      | `--workspace <pattern>` | N/A                 | N/A                      | Target specific workspace (pnpm/npm)                            |
| `-w, --workspace-root`    | `-w`                      | N/A                     | N/A                 | N/A                      | Check in workspace root (pnpm-specific)                         |
| `-P, --prod`              | `-P, --prod`              | N/A                     | N/A                 | N/A                      | Only production dependencies (pnpm only)                        |
| `-D, --dev`               | `-D, --dev`               | N/A                     | N/A                 | N/A                      | Only dev dependencies (pnpm only)                               |
| `--depth <number>`        | `--depth <number>`        | N/A                     | N/A                 | N/A                      | Limit tree depth (pnpm only)                                    |
| `--no-optional`           | `--no-optional`           | N/A                     | `--ignore-optional` | N/A                      | Exclude optional dependencies (pnpm only)                       |
| `-g, --global`            | `-g, --global`            | N/A                     | N/A                 | N/A                      | Check globally installed packages                               |
| `--exclude-peers`         | `--exclude-peers`         | N/A                     | N/A                 | Removes `--peers` flag   | Exclude peer dependencies (yarn@2+ defaults to including peers) |
| `--find-by <finder_name>` | `--find-by <finder_name>` | N/A                     | N/A                 | N/A                      | Use finder function from .pnpmfile.cjs                          |

**Note:**

- npm uses `explain` as the primary command, `why` as alias, supports multiple packages
- pnpm uses `why` as the primary command, supports multiple packages and glob patterns
- yarn has `why` command in both v1 and v2+, but different output formats, only supports single package
- pnpm has the most comprehensive filtering and output options
- npm has simpler output focused on the dependency path

**Aliases:**

- `vite explain` = `vite why` (matches npm's primary command name)

### Why Behavior Differences Across Package Managers

#### pnpm

**Why behavior:**

- Shows all packages that depend on the specified package
- Supports multiple packages and glob patterns: `pnpm why babel-* eslint-*`
- Displays dependency tree with complete paths
- Truncates output after 10 end leaves to prevent memory issues
- Supports workspace filtering with `--filter`
- Can filter by dependency type (prod, dev, optional)
- Supports depth limiting
- Can check global packages with `-g`

**Output format:**

```
Legend: production dependency, optional only, dev only

package-a@1.0.0 /path/to/package-a
└── react@18.3.1
    └── react-dom@18.3.1

package-b@2.0.0 /path/to/package-b
└─┬ @testing-library/react@14.0.0
  └── react@18.3.1
```

**Options:**

- `--json`: JSON format
- `--long`: Extended information
- `--parseable`: Parseable format (no tree structure)
- `-r`: Recursive across workspaces
- `--filter`: Workspace filtering
- `--prod`/`--dev`: Dependency type filtering
- `--depth`: Limit tree depth
- `--exclude-peers`: Exclude peer dependencies

#### npm

**Explain behavior:**

- Shows the dependency path for why a package is installed
- Primary command is `explain`, `why` is an alias
- Simple, focused output showing dependency chain
- Supports workspace targeting with `--workspace`
- JSON output available

**Output format:**

```
react@18.3.1
node_modules/react
  react@"^18.3.1" from react-dom@18.3.1
  node_modules/react-dom
    react-dom@"^18.3.1" from the root project
  react@"^18.3.1" from @testing-library/react@14.0.0
  node_modules/@testing-library/react
    @testing-library/react@"^14.0.0" from the root project
```

**Options:**

- `--json`: JSON format
- `--workspace`: Target specific workspace

#### yarn@1 (Classic)

**Why behavior:**

- Identifies why a package has been installed
- Shows which packages depend on it
- Displays disk size information (with and without dependencies)
- Shows whether package is hoisted
- Can accept package name, folder path, or file path

**Output format:**

```
[1/4] 🤔  Why do we have the package "jest"?
[2/4] 🚚  Required dependencies
info Reasons this module exists
   - "@my/package#devDependencies" depends on it
   - Hoisted from "@my/package#jest"
[3/4] 💾  Disk size without dependencies: "0B"
[4/4] 📦  Dependencies using this package
```

**Options:**

- No command-line options
- Single package only

#### yarn@2+ (Berry)

**Why behavior:**

- Shows why a package is present in the dependency tree
- More streamlined output than yarn@1
- Supports recursive workspace checking
- Includes peer dependencies by default (uses `--peers` flag)
- Use `--exclude-peers` to remove the `--peers` flag

**Output format:**

```
➤ YN0000: react@npm:18.3.1
➤ YN0000: └ Required by: react-dom@npm:18.3.1
➤ YN0000: └ Required by: @testing-library/react@npm:14.0.0
```

**Options:**

- `--recursive`: Check across workspaces
- `--peers`: Include peer dependencies (added by default via Vite+)
- Different plugin system may affect output

### Implementation Architecture

#### 1. Command Structure

**File**: `crates/vite_task/src/lib.rs`

Add new command variant:

```rust
#[derive(Subcommand, Debug)]
pub enum Commands {
    // ... existing commands

    /// Show why a package is installed
    #[command(disable_help_flag = true, alias = "explain")]
    Why {
        /// Package(s) to check
        packages: Vec<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Show extended information (pnpm only)
        #[arg(long)]
        long: bool,

        /// Show parseable output (pnpm only)
        #[arg(long)]
        parseable: bool,

        /// Check recursively across all workspaces
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Filter packages in monorepo (pnpm only)
        #[arg(long, value_name = "PATTERN")]
        filter: Vec<String>,

        /// Check in workspace root (pnpm only)
        #[arg(short = 'w', long)]
        workspace_root: bool,

        /// Only production dependencies (pnpm only)
        #[arg(short = 'P', long)]
        prod: bool,

        /// Only dev dependencies (pnpm only)
        #[arg(short = 'D', long)]
        dev: bool,

        /// Limit tree depth (pnpm only)
        #[arg(long)]
        depth: Option<u32>,

        /// Exclude optional dependencies (pnpm only)
        #[arg(long)]
        no_optional: bool,

        /// Check globally installed packages (pnpm only)
        #[arg(short = 'g', long)]
        global: bool,

        /// Exclude peer dependencies (pnpm only)
        #[arg(long)]
        exclude_peers: bool,

        /// Use a finder function defined in .pnpmfile.cjs (pnpm only)
        #[arg(long)]
        find_by: Option<String>,

        /// Arguments to pass to package manager
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
}
```

#### 2. Package Manager Adapter

**File**: `crates/vite_package_manager/src/commands/why.rs` (new file)

```rust
use std::{collections::HashMap, process::ExitStatus};

use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env, run_command,
};

#[derive(Debug, Default)]
pub struct WhyCommandOptions<'a> {
    pub packages: &'a [String],
    pub json: bool,
    pub long: bool,
    pub parseable: bool,
    pub recursive: bool,
    pub filters: Option<&'a [String]>,
    pub workspace_root: bool,
    pub prod: bool,
    pub dev: bool,
    pub depth: Option<u32>,
    pub no_optional: bool,
    pub global: bool,
    pub exclude_peers: bool,
    pub find_by: Option<&'a str>,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the why command with the package manager.
    #[must_use]
    pub async fn run_why_command(
        &self,
        options: &WhyCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_why_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the why command.
    #[must_use]
    pub fn resolve_why_command(&self, options: &WhyCommandOptions) -> ResolveCommandResult {
        let bin_name: String;
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                bin_name = "pnpm".into();

                // pnpm: --filter must come before command
                if let Some(filters) = options.filters {
                    for filter in filters {
                        args.push("--filter".into());
                        args.push(filter.clone());
                    }
                }

                args.push("why".into());

                if options.json {
                    args.push("--json".into());
                }

                if options.long {
                    args.push("--long".into());
                }

                if options.parseable {
                    args.push("--parseable".into());
                }

                if options.recursive {
                    args.push("--recursive".into());
                }

                if options.workspace_root {
                    args.push("--workspace-root".into());
                }

                if options.prod {
                    args.push("--prod".into());
                }

                if options.dev {
                    args.push("--dev".into());
                }

                if let Some(depth) = options.depth {
                    args.push("--depth".into());
                    args.push(depth.to_string());
                }

                if options.no_optional {
                    args.push("--no-optional".into());
                }

                if options.global {
                    args.push("--global".into());
                }

                if options.exclude_peers {
                    args.push("--exclude-peers".into());
                }

                if let Some(find_by) = options.find_by {
                    args.push("--find-by".into());
                    args.push(find_by.to_string());
                }

                // Add packages (pnpm supports multiple packages)
                args.extend_from_slice(options.packages);
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();

                args.push("why".into());

                // yarn only supports single package
                if options.packages.len() > 1 {
                    eprintln!("Warning: yarn only supports checking one package at a time, using first package");
                }
                args.push(options.packages[0].clone());

                // yarn@2+ supports --recursive
                if options.recursive && !self.version.starts_with("1.") {
                    args.push("--recursive".into());
                }

                // yarn@2+: Add --peers by default unless --exclude-peers is set
                if !self.version.starts_with("1.") && !options.exclude_peers {
                    args.push("--peers".into());
                }

                // Warn about unsupported flags
                if options.json {
                    eprintln!("Warning: --json not supported by yarn");
                }
                if options.long {
                    eprintln!("Warning: --long not supported by yarn");
                }
                if options.parseable {
                    eprintln!("Warning: --parseable not supported by yarn");
                }
                if let Some(filters) = options.filters {
                    if !filters.is_empty() {
                        eprintln!("Warning: --filter not supported by yarn");
                    }
                }
                if options.prod || options.dev {
                    eprintln!("Warning: --prod/--dev not supported by yarn");
                }
                if options.find_by.is_some() {
                    eprintln!("Warning: --find-by not supported by yarn");
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();

                // npm uses 'explain' as primary command
                args.push("explain".into());

                // npm: --workspace comes after command
                if let Some(filters) = options.filters {
                    for filter in filters {
                        args.push("--workspace".into());
                        args.push(filter.clone());
                    }
                }

                if options.json {
                    args.push("--json".into());
                }

                // Add packages (npm supports multiple packages)
                args.extend_from_slice(options.packages);

                // Warn about pnpm-specific flags
                if options.long {
                    eprintln!("Warning: --long not supported by npm");
                }
                if options.parseable {
                    eprintln!("Warning: --parseable not supported by npm");
                }
                if options.prod || options.dev {
                    eprintln!("Warning: --prod/--dev not supported by npm");
                }
                if options.depth.is_some() {
                    eprintln!("Warning: --depth not supported by npm");
                }
                if options.find_by.is_some() {
                    eprintln!("Warning: --find-by not supported by npm");
                }
            }
        }

        // Add pass-through args
        if let Some(pass_through_args) = options.pass_through_args {
            args.extend_from_slice(pass_through_args);
        }

        ResolveCommandResult { bin_path: bin_name, args, envs }
    }
}
```

**File**: `crates/vite_package_manager/src/commands/mod.rs`

Update to include why module:

```rust
pub mod add;
mod install;
pub mod remove;
pub mod update;
pub mod link;
pub mod unlink;
pub mod dedupe;
pub mod why;  // Add this line
```

#### 3. Why Command Implementation

**File**: `crates/vite_task/src/why.rs` (new file)

```rust
use vite_error::Error;
use vite_path::AbsolutePathBuf;
use vite_package_manager::{
    PackageManager,
    commands::why::WhyCommandOptions,
};
use vite_workspace::Workspace;

pub struct WhyCommand {
    workspace_root: AbsolutePathBuf,
}

impl WhyCommand {
    pub fn new(workspace_root: AbsolutePathBuf) -> Self {
        Self { workspace_root }
    }

    pub async fn execute(
        self,
        packages: Vec<String>,
        json: bool,
        long: bool,
        parseable: bool,
        recursive: bool,
        filters: Vec<String>,
        workspace_root: bool,
        prod: bool,
        dev: bool,
        depth: Option<u32>,
        no_optional: bool,
        global: bool,
        exclude_peers: bool,
        extra_args: Vec<String>,
    ) -> Result<ExecutionSummary, Error> {
        if packages.is_empty() {
            return Err(Error::NoPackagesSpecified);
        }

        let package_manager = PackageManager::builder(&self.workspace_root).build().await?;
        let workspace = Workspace::partial_load(self.workspace_root)?;

        // Build why command options
        let why_options = WhyCommandOptions {
            packages: &packages,
            json,
            long,
            parseable,
            recursive,
            filters: if filters.is_empty() { None } else { Some(&filters) },
            workspace_root,
            prod,
            dev,
            depth,
            no_optional,
            global,
            exclude_peers,
            pass_through_args: if extra_args.is_empty() { None } else { Some(&extra_args) },
        };

        let exit_status = package_manager
            .run_why_command(&why_options, &workspace.root)
            .await?;

        if !exit_status.success() {
            return Err(Error::CommandFailed {
                command: "why".to_string(),
                exit_code: exit_status.code(),
            });
        }

        workspace.unload().await?;

        Ok(ExecutionSummary::default())
    }
}
```

## Design Decisions

### 1. No Caching

**Decision**: Do not cache why operations.

**Rationale**:

- `why` queries current dependency state
- Results depend on installed packages
- Caching would provide stale information
- Fast operation, caching not needed

### 2. Multiple Package Support

**Decision**: Accept multiple packages and pass them through to package managers that support it.

**Rationale**:

- pnpm supports multiple packages: `pnpm why react react-dom`
- npm supports multiple packages: `npm explain react react-dom`
- yarn only supports single package
- Warn and use first package for yarn only
- Better UX than erroring

### 3. Alias Choice

**Decision**: Use `explain` as alias (matches npm).

**Rationale**:

- npm uses `explain` as primary command, `why` as alias
- More descriptive verb
- Helps npm users feel at home
- Both commands achieve same goal

### 4. Output Format Support

**Decision**: Support pnpm's output format flags with warnings on other package managers.

**Rationale**:

- pnpm has `--json`, `--long`, `--parseable`
- npm only has `--json`
- yarn has fixed output format
- Warn users about unsupported formats

### 5. Workspace Filtering

**Decision**: Support `--filter` flag which translates to appropriate package manager syntax.

**Rationale**:

- pnpm uses `--filter` before command: `pnpm --filter app why react`
- npm uses `--workspace` after command: `npm explain --workspace app react`
- Vite+ uses unified `--filter` flag that translates appropriately
- yarn doesn't support workspace filtering
- Consistent with other Vite+ commands

### 6. Dependency Type Filtering

**Decision**: Support pnpm's `--prod`, `--dev`, `--no-optional` flags with warnings.

**Rationale**:

- pnpm allows filtering by dependency type
- Not available in npm or yarn
- Useful for focused analysis
- Warn when not supported

## Error Handling

### No Package Manager Detected

```bash
$ vite why react
Error: No package manager detected
Please run one of:
  - vite install (to set up package manager)
  - Add packageManager field to package.json
```

### No Packages Specified

```bash
$ vite why
error: the following required arguments were not provided:
  <PACKAGES>...

Usage: vite why [OPTIONS] <PACKAGES>... [-- <PASS_THROUGH_ARGS>...]

For more information, try '--help'.
```

### Package Not Found

```bash
$ vite why nonexistent-package
Package 'nonexistent-package' is not in the project.
Exit code: 1
```

### Unsupported Flag Warning

```bash
$ vite why react --long
Warning: --long not supported by npm
Running: npm explain react
```

## User Experience

### Success Output (pnpm)

```bash
$ vite why react
Detected package manager: pnpm@10.15.0
Running: pnpm why react

Legend: production dependency, optional only, dev only

my-app@1.0.0 /Users/user/my-app

dependencies:
react 18.3.1
├── react-dom 18.3.1
└─┬ @testing-library/react 14.0.0
  └─┬ @testing-library/dom 9.3.4
    └─┬ @testing-library/user-event 14.5.2
      └── react-dom 18.3.1

devDependencies:
react 18.3.1
└── @types/react 18.3.3

Done in 0.5s
```

### Success Output (npm)

```bash
$ vite explain react
Detected package manager: npm@11.0.0
Running: npm explain react

react@18.3.1
node_modules/react
  react@"^18.3.1" from react-dom@18.3.1
  node_modules/react-dom
    react-dom@"^18.3.1" from the root project
  react@"^18.3.1" from @testing-library/react@14.0.0
  node_modules/@testing-library/react
    @testing-library/react@"^14.0.0" from the root project

Done in 0.3s
```

### Success Output (yarn)

```bash
$ vite why react
Detected package manager: yarn@1.22.19
Running: yarn why react

[1/4] 🤔  Why do we have the package "react"?
[2/4] 🚚  Required dependencies
info Reasons this module exists
   - "my-app#dependencies" depends on it
   - Hoisted from "my-app#react"
[3/4] 💾  Disk size without dependencies: "285KB"
[4/4] 📦  Dependencies using this package: react-dom, @testing-library/react

Done in 0.8s
```

### JSON Output (pnpm)

```bash
$ vite why react --json
Detected package manager: pnpm@10.15.0
Running: pnpm why react --json

[
  {
    "name": "my-app",
    "version": "1.0.0",
    "path": "/Users/user/my-app",
    "dependencies": {
      "react": {
        "version": "18.3.1",
        "dependents": [
          {
            "name": "react-dom",
            "version": "18.3.1"
          },
          {
            "name": "@testing-library/react",
            "version": "14.0.0"
          }
        ]
      }
    }
  }
]

Done in 0.4s
```

### Multiple Packages (pnpm)

```bash
$ vite why react react-dom lodash
Detected package manager: pnpm@10.15.0
Running: pnpm why react react-dom lodash

Legend: production dependency, optional only, dev only

my-app@1.0.0 /Users/user/my-app

react 18.3.1
└── react-dom 18.3.1

react-dom 18.3.1
dependency of my-app

lodash 4.17.21
└─┬ webpack 5.95.0
  └── babel-loader 9.2.1

Done in 0.6s
```

## Alternative Designs Considered

### Alternative 1: Separate Command Names

```bash
vite why <package>      # For pnpm/yarn
vite explain <package>  # For npm only
```

**Rejected because**:

- Creates confusion about which to use
- Package manager should be abstracted
- Aliases are better than separate commands

### Alternative 2: Always Use Multiple Package Format

```bash
vite why react react-dom  # Always accept multiple
# Error on npm/yarn
```

**Rejected because**:

- Too strict, prevents usage
- Better to warn and use first package
- Provides better UX

### Alternative 3: Auto-Translate Output Format

```bash
vite why react --json  # On yarn
# Attempt to convert yarn's output to JSON
```

**Rejected because**:

- Output format parsing is fragile
- Different package managers have different data
- Better to warn about unsupported features
- Let native output through

## Implementation Plan

### Phase 1: Core Functionality

1. Add `Why` command variant to `Commands` enum
2. Create `why.rs` module in both crates
3. Implement package manager command resolution
4. Add basic error handling

### Phase 2: Advanced Features

1. Implement output format options (json, long, parseable)
2. Add workspace filtering support
3. Implement dependency type filtering (prod, dev)
4. Handle depth limiting

### Phase 3: Testing

1. Unit tests for command resolution
2. Integration tests with mock package managers
3. Test multiple package support
4. Test workspace operations
5. Test output format options

### Phase 4: Documentation

1. Update CLI documentation
2. Add examples to README
3. Document package manager compatibility
4. Add troubleshooting guide

## Testing Strategy

### Test Package Manager Versions

- pnpm@9.x
- pnpm@10.x
- yarn@1.x
- yarn@4.x
- npm@10.x
- npm@11.x

### Unit Tests

```rust
#[test]
fn test_pnpm_why_basic() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.resolve_why_command(&WhyCommandOptions {
        packages: &["react".to_string()],
        ..Default::default()
    });
    assert_eq!(args, vec!["why", "react"]);
}

#[test]
fn test_pnpm_why_multiple_packages() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.resolve_why_command(&WhyCommandOptions {
        packages: &["react".to_string(), "lodash".to_string()],
        ..Default::default()
    });
    assert_eq!(args, vec!["why", "react", "lodash"]);
}

#[test]
fn test_pnpm_why_json() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.resolve_why_command(&WhyCommandOptions {
        packages: &["react".to_string()],
        json: true,
        ..Default::default()
    });
    assert_eq!(args, vec!["why", "--json", "react"]);
}

#[test]
fn test_npm_explain_basic() {
    let pm = PackageManager::mock(PackageManagerType::Npm);
    let args = pm.resolve_why_command(&WhyCommandOptions {
        packages: &["react".to_string()],
        ..Default::default()
    });
    assert_eq!(args, vec!["explain", "react"]);
}

#[test]
fn test_yarn_why_basic() {
    let pm = PackageManager::mock(PackageManagerType::Yarn);
    let args = pm.resolve_why_command(&WhyCommandOptions {
        packages: &["react".to_string()],
        ..Default::default()
    });
    assert_eq!(args, vec!["why", "react"]);
}

#[test]
fn test_pnpm_why_with_filter() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.resolve_why_command(&WhyCommandOptions {
        packages: &["react".to_string()],
        filters: Some(&["app".to_string()]),
        ..Default::default()
    });
    assert_eq!(args, vec!["--filter", "app", "why", "react"]);
}

#[test]
fn test_pnpm_why_with_depth() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.resolve_why_command(&WhyCommandOptions {
        packages: &["react".to_string()],
        depth: Some(3),
        ..Default::default()
    });
    assert_eq!(args, vec!["why", "--depth", "3", "react"]);
}
```

### Integration Tests

Create fixtures for testing with each package manager:

```
fixtures/why-test/
  pnpm-workspace.yaml
  package.json
  packages/
    app/
      package.json (with react, lodash deps)
    utils/
      package.json (with lodash dep)
  test-steps.json
```

Test cases:

1. Basic why for single package
2. Multiple packages (pnpm only)
3. JSON output
4. Workspace-specific why
5. Recursive workspace checking
6. Dependency type filtering
7. Depth limiting
8. Global package checking
9. Warning messages for unsupported flags

## CLI Help Output

```bash
$ vite why --help
Show why a package is installed

Usage: vite why [OPTIONS] <PACKAGE>... [-- <PASS_THROUGH_ARGS>...]

Aliases: explain

Arguments:
  <PACKAGE>...           Package(s) to check (required, pnpm/npm support multiple, yarn uses first)

Options:
  --json                 Output in JSON format
  --long                 Show extended information (pnpm-specific)
  --parseable            Show parseable output (pnpm-specific)
  -r, --recursive        Check recursively across all workspaces
  --filter <PATTERN>     Filter packages in monorepo (pnpm-specific, can be used multiple times)
  -w, --workspace-root   Check in workspace root (pnpm-specific)
  -P, --prod             Only production dependencies (pnpm-specific)
  -D, --dev              Only dev dependencies (pnpm-specific)
  --depth <NUMBER>       Limit tree depth (pnpm-specific)
  --no-optional          Exclude optional dependencies (pnpm-specific)
  -g, --global           Check globally installed packages
  --exclude-peers        Exclude peer dependencies (pnpm/yarn@2+-specific)
  --find-by <FINDER_NAME> Use a finder function defined in .pnpmfile.cjs (pnpm-specific)
  -h, --help             Print help

Package Manager Behavior:
  pnpm:    Shows complete dependency tree with all dependents
  npm:     Shows dependency path explaining installation
  yarn@1:  Shows why package exists with disk size info
  yarn@2+: Shows dependency tree in streamlined format

Examples:
  vite why react                       # Show why react is installed
  vite explain lodash                  # Same as above (alias)
  vite why react react-dom             # Check multiple packages (pnpm/npm)
  vite why react --json                # JSON output
  vite why react --long                # Verbose output (pnpm)
  vite why react -r                    # Recursive across workspaces
  vite why react --filter app          # Check in specific workspace (pnpm)
  vite why react --prod                # Only production deps (pnpm)
  vite why react --depth 3             # Limit tree depth (pnpm)
  vite why typescript -g               # Check global packages
  vite why react --find-by myFinder    # Use custom finder (pnpm)
```

## Performance Considerations

1. **No Caching**: Fast query operation, caching not beneficial
2. **Native Performance**: Delegates to package manager's optimized code
3. **Single Execution**: Quick analysis of current state
4. **JSON Output**: Can be parsed for programmatic usage

## Security Considerations

1. **Read-Only**: Only reads installed packages, no modifications
2. **No Code Execution**: Just queries dependency tree
3. **Safe for CI**: Can be run safely in CI/CD pipelines
4. **Audit Integration**: Helps understand security vulnerability origins

## Backward Compatibility

This is a new feature with no breaking changes:

- Existing commands unaffected
- New command is additive
- No changes to task configuration
- No changes to caching behavior

## Migration Path

### Adoption

Users can start using immediately:

```bash
# Old way
pnpm why react
npm explain react

# New way (works with any package manager)
vite why react
vite explain react
```

### CI/CD Integration

```yaml
# Check why specific package is installed
- run: vite why lodash --json > why-lodash.json

# Verify expected dependency paths
- run: vite why react | grep "react-dom"
```

## Real-World Usage Examples

### Debugging Duplicate Dependencies

```bash
# Check why multiple versions are installed
vite why lodash
vite why lodash --json | jq '.[] | .dependencies.lodash.version'

# Check across workspaces
vite why lodash -r
```

### Understanding Transitive Dependencies

```bash
# Why is this indirect dependency here?
vite why core-js
vite why core-js --long

# What's using this deep dependency?
vite why @babel/helper-plugin-utils
```

### Auditing Dependencies

```bash
# Check security vulnerability origins
vite why vulnerable-package
vite why vulnerable-package --prod  # Only production

# Find all dependents in monorepo
vite why legacy-library -r --json
```

### Workspace Analysis

```bash
# Which workspaces use this package?
vite why react -r

# Check specific workspace
vite why lodash --filter utils

# Compare dependency reasons across workspaces
vite why axios --filter "app*" -r
```

### Production Dependency Analysis

```bash
# What production code needs this?
vite why package --prod

# Exclude dev dependencies
vite why package --prod --json
```

## Package Manager Compatibility

| Feature          | pnpm              | npm              | yarn@1           | yarn@2+          | Notes                   |
| ---------------- | ----------------- | ---------------- | ---------------- | ---------------- | ----------------------- |
| Basic command    | `why`             | `explain`        | `why`            | `why`            | npm uses different name |
| Multiple pkgs    | ✅ Supported      | ✅ Supported     | ❌ Single only   | ❌ Single only   | pnpm and npm            |
| Glob patterns    | ✅ Supported      | ❌ Not supported | ❌ Not supported | ❌ Not supported | pnpm only               |
| JSON output      | ✅ `--json`       | ✅ `--json`      | ❌ Not supported | ❌ Not supported | pnpm and npm only       |
| Long output      | ✅ `--long`       | ❌ Not supported | ❌ Not supported | ❌ Not supported | pnpm only               |
| Parseable        | ✅ `--parseable`  | ❌ Not supported | ❌ Not supported | ❌ Not supported | pnpm only               |
| Recursive        | ✅ `-r`           | ❌ Not supported | ❌ Not supported | ✅ `--recursive` | pnpm and yarn@2+        |
| Workspace filter | ✅ `--filter`     | ✅ `--workspace` | ❌ Not supported | ❌ Not supported | pnpm and npm            |
| Dep type filter  | ✅ `--prod/--dev` | ❌ Not supported | ❌ Not supported | ❌ Not supported | pnpm only               |
| Depth limit      | ✅ `--depth`      | ❌ Not supported | ❌ Not supported | ❌ Not supported | pnpm only               |
| Global check     | ✅ `-g`           | ❌ Not supported | ❌ Not supported | ❌ Not supported | pnpm only               |

## Future Enhancements

### 1. Dependency Graph Visualization

Generate visual dependency graphs:

```bash
vite why react --graph > dep-graph.html

# ASCII tree visualization
vite why react --tree
```

### 2. Compare Why Across Versions

Show how dependency changed:

```bash
vite why lodash --compare-version 4.17.20

# Output:
lodash@4.17.21 (was 4.17.20)
└── webpack 5.95.0 (upgraded from 5.90.0)
```

### 3. Why Report

Generate comprehensive dependency report:

```bash
vite why --report-all > dependencies-report.json

# All packages and their dependents
# Useful for auditing and optimization
```

### 4. Circular Dependency Detection

Highlight circular dependencies:

```bash
vite why package-a --detect-circular

# Output:
⚠️  Circular dependency detected:
package-a → package-b → package-c → package-a
```

### 5. Size Analysis Integration

Show size impact:

```bash
vite why lodash --with-size

# Output:
lodash@4.17.21 (285KB gzipped)
└── webpack (brings in 15MB)
└── babel (brings in 8MB)
Total impact: 23.3MB
```

## Open Questions

1. **Should we support package path queries (yarn style)?**
   - Proposed: Yes, for yarn compatibility
   - Example: `vite why node_modules/once/once.js`
   - Translate to package name for other PMs

2. **Should we aggregate output when checking multiple packages?**
   - Proposed: No, show separate results
   - Matches pnpm behavior
   - Easier to parse

3. **Should we support interactive mode?**
   - Proposed: Later enhancement
   - Let users explore dependency tree interactively
   - Similar to `npm ls --interactive`

4. **Should we cache why results?**
   - Proposed: No, always query current state
   - Dependency tree changes frequently
   - Fast operation doesn't need caching

5. **Should we integrate with audit?**
   - Proposed: Later enhancement
   - Show security info inline
   - Example: `vite why package --with-audit`

## Success Metrics

1. **Adoption**: % of users using `vite why` vs direct package manager
2. **Debugging Efficiency**: Time to identify dependency issues
3. **CI Integration**: Usage in CI/CD for dependency validation
4. **User Feedback**: Survey/issues about command usefulness

## Conclusion

This RFC proposes adding `vite why` command to provide a unified interface for understanding dependency relationships across pnpm/npm/yarn. The design:

- ✅ Automatically adapts to detected package manager
- ✅ Supports multiple packages (pnpm) with graceful degradation
- ✅ Full pnpm feature support (json, long, parseable, filters)
- ✅ npm and yarn compatibility with appropriate warnings
- ✅ Workspace-aware operations
- ✅ Clear output showing dependency paths
- ✅ No caching (reads current state)
- ✅ Simple implementation leveraging existing infrastructure
- ✅ Extensible for future enhancements (graphs, size analysis)

The implementation follows the same patterns as other package management commands while providing the dependency analysis features developers need to understand, debug, and optimize their dependency trees.
