# RFC: Vite+ Outdated Package Command

## Summary

Add `vite outdated` command that automatically adapts to the detected package manager (pnpm/npm/yarn) for checking outdated packages. This helps developers identify packages that have newer versions available, maintain up-to-date dependencies, and manage security vulnerabilities by showing which packages can be updated.

## Motivation

Currently, developers must manually use package manager-specific commands to check for outdated packages:

```bash
pnpm outdated [<pattern>...]
npm outdated [[@scope/]<package>...]
yarn outdated [<package>...]
```

This creates friction in dependency management workflows and requires remembering different syntaxes. A unified interface would:

1. **Simplify dependency updates**: One command works across all package managers
2. **Auto-detection**: Automatically uses the correct package manager
3. **Consistency**: Same syntax regardless of underlying tool
4. **Integration**: Works seamlessly with existing Vite+ features

### Current Pain Points

```bash
# Developer needs to know which package manager is used
pnpm outdated                         # pnpm project
npm outdated                          # npm project
yarn outdated                         # yarn project

# Different output formats
pnpm outdated --format json           # pnpm - JSON output
npm outdated --json                   # npm - JSON output
yarn outdated                         # yarn - table format (no JSON in v1)

# Different workspace targeting
pnpm outdated --filter app            # pnpm - filter workspaces
npm outdated --workspace app          # npm - specify workspace
yarn outdated                         # yarn - no workspace filtering in v1

# Different dependency type filtering
pnpm outdated --prod                  # pnpm - only production deps
npm outdated                          # npm - no filtering option
yarn outdated                         # yarn - no filtering option
```

### Proposed Solution

```bash
# Works for all package managers
vite outdated                         # Check all packages
vite outdated <package>               # Check specific packages

# Output formats
vite outdated --format json           # JSON output (maps to pnpm --format json, npm --json, yarn --json)
vite outdated --format list           # List output (maps to pnpm --format list, npm --parseable)
vite outdated --format table          # Table format (default)
vite outdated --long                  # Verbose output

# Workspace operations
vite outdated --filter app            # Check in specific workspace (maps to pnpm --filter, npm --workspace)
vite outdated -r                      # Check recursively across workspaces (maps to pnpm -r, npm --all)
vite outdated -w                      # Include workspace root (pnpm)
vite outdated -w -r                   # Include workspace root and check recursively (pnpm)

# Dependency type filtering
vite outdated -P                      # Only production dependencies (pnpm)
vite outdated --prod                  # Only production dependencies (pnpm)
vite outdated -D                      # Only dev dependencies (pnpm)
vite outdated --dev                   # Only dev dependencies (pnpm)
vite outdated --compatible            # Only versions satisfying package.json (pnpm)

# Sorting and filtering
vite outdated --sort-by name          # Sort results by name (pnpm)
vite outdated --no-optional           # Exclude optional dependencies (pnpm)
```

## Proposed Solution

### Command Syntax

```bash
vite outdated [PACKAGE...] [OPTIONS]
```

**Examples:**

```bash
# Basic usage
vite outdated
vite outdated react
vite outdated "*gulp-*" @babel/core

# Output formats
vite outdated --format json           # JSON output
vite outdated --format list           # List output
vite outdated --long                  # Verbose output

# Workspace operations
vite outdated -r                      # Recursive across all workspaces
vite outdated --recursive             # Recursive across all workspaces
vite outdated --filter app            # Check in specific workspace
vite outdated -w                      # Include workspace root (pnpm)
vite outdated -w -r                   # Include workspace root and check recursively (pnpm)

# Dependency type filtering
vite outdated -P                      # Only production dependencies (pnpm)
vite outdated --prod                  # Only production dependencies (pnpm)
vite outdated -D                      # Only dev dependencies (pnpm)
vite outdated --dev                   # Only dev dependencies (pnpm)
vite outdated --no-optional           # Exclude optional dependencies (pnpm)
vite outdated --compatible            # Only compatible versions (pnpm)

# Sorting
vite outdated --sort-by name          # Sort results by name (pnpm)

# Global packages
vite outdated -g                      # Check globally installed packages
```

### Global packages checking

Only use `npm` to check globally installed packages, because `vite install -g` uses `npm` cli to install global packages.

```bash
vite outdated -g                      # Check globally installed packages

-> npm outdated -g
```

### Command Mapping

**pnpm references:**

- https://pnpm.io/cli/outdated
- Checks for outdated packages with pattern support

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-outdated
- Lists outdated packages

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/outdated (yarn@1)
- https://yarnpkg.com/cli/upgrade-interactive (yarn@2+)
- Checks for outdated package dependencies

| Vite+ Flag             | pnpm                   | npm                                 | yarn@1          | yarn@2+                    | Description                                   |
| ---------------------- | ---------------------- | ----------------------------------- | --------------- | -------------------------- | --------------------------------------------- |
| `vite outdated`        | `pnpm outdated`        | `npm outdated`                      | `yarn outdated` | `yarn upgrade-interactive` | Check for outdated packages                   |
| `<pattern>...`         | `<pattern>...`         | `[[@scope/]<pkg>]`                  | `[<package>]`   | N/A                        | Package patterns to check                     |
| `--long`               | `--long`               | `--long`                            | N/A             | N/A                        | Extended output format                        |
| `--format <format>`    | `--format <format>`    | json: `--json`/ list: `--parseable` | `--json`        | N/A                        | Output format (table/list/json)               |
| `-r, --recursive`      | `-r, --recursive`      | `--all`                             | N/A             | N/A                        | Check across all workspaces                   |
| `--filter <pattern>`   | `--filter <pattern>`   | `--workspace <pattern>`             | N/A             | N/A                        | Target specific workspace                     |
| `-w, --workspace-root` | `-w, --workspace-root` | `--include-workspace-root`          | N/A             | N/A                        | Include workspace root                        |
| `-P, --prod`           | `-P, --prod`           | N/A                                 | N/A             | N/A                        | Only production dependencies (pnpm-specific)  |
| `-D, --dev`            | `-D, --dev`            | N/A                                 | N/A             | N/A                        | Only dev dependencies (pnpm-specific)         |
| `--no-optional`        | `--no-optional`        | N/A                                 | N/A             | N/A                        | Exclude optional dependencies (pnpm-specific) |
| `--compatible`         | `--compatible`         | N/A                                 | N/A             | N/A                        | Only show compatible versions (pnpm-specific) |
| `--sort-by <field>`    | `--sort-by <field>`    | N/A                                 | N/A             | N/A                        | Sort results by field (pnpm-specific)         |
| `-g, --global`         | `-g, --global`         | `-g, --global`                      | N/A             | N/A                        | Check globally installed packages             |

**Note:**

- pnpm supports pattern matching for selective package checking
- npm accepts package names but not glob patterns
- yarn@1 accepts package names but limited filtering options
- yarn@2+ uses interactive mode (`upgrade-interactive`) instead of traditional `outdated`
- pnpm has the most comprehensive filtering and output options

### Outdated Behavior Differences Across Package Managers

#### pnpm

**Outdated behavior:**

- Checks for outdated packages with pattern support
- Supports glob patterns: `pnpm outdated "*gulp-*" @babel/core`
- Shows current, wanted, and latest versions
- Supports workspace filtering with `--filter`
- Can filter by dependency type (prod, dev, optional)
- Multiple output formats (table, list, json)
- Shows only compatible versions with `--compatible`

**Output format:**

```
Package         Current  Wanted  Latest
react           18.2.0   18.3.1  18.3.1
lodash          4.17.20  4.17.21 4.17.21
@babel/core     7.20.0   7.20.12 7.25.8
```

**Options:**

- `--format`: Output format (table, list, json)
- `--long`: Extended information
- `-r`: Recursive across workspaces
- `--filter`: Workspace filtering
- `--prod`/`--dev`: Dependency type filtering
- `--compatible`: Only compatible versions
- `--sort-by`: Sort results by field
- `--no-optional`: Exclude optional dependencies

#### npm

**Outdated behavior:**

- Lists outdated packages
- Shows current, wanted, latest, location, and depended by
- Supports workspace targeting with `--workspace`
- Can show all dependencies with `--all` (including transitive)
- JSON and parseable output available
- Color-coded output (red = should update, yellow = major version)

**Output format:**

```
Package         Current  Wanted  Latest  Location             Depended by
react           18.2.0   18.3.1  18.3.1  node_modules/react   my-app
lodash          4.17.20  4.17.21 4.17.21 node_modules/lodash  my-app
```

**Options:**

- `--json`: JSON format
- `--long`: Extended information (shows package type)
- `--parseable`: Parseable format
- `--all`: Show all outdated packages including transitive
- `--workspace`: Target specific workspace

#### yarn@1 (Classic)

**Outdated behavior:**

- Checks for outdated package dependencies
- Shows package name, current, wanted, latest, package type, and URL
- Simple table output
- Can check specific packages
- No JSON output support
- No workspace filtering

**Output format:**

```
Package         Current  Wanted  Latest  Package Type  URL
react           18.2.0   18.3.1  18.3.1  dependencies  https://...
lodash          4.17.20  4.17.21 4.17.21 dependencies  https://...
```

**Options:**

- No command-line options for filtering or formatting
- Accepts package names as arguments

#### yarn@2+ (Berry)

**Outdated behavior:**

- Uses `yarn upgrade-interactive` instead of `outdated`
- Opens fullscreen terminal interface
- Shows out-of-date packages with status comparison
- Allows selective upgrading
- Different paradigm from traditional `outdated` command

**Output format:**

Interactive terminal UI showing:

- Package names
- Current versions
- Available versions
- Selection checkboxes

**Options:**

- Interactive mode only
- `yarn upgrade-interactive` for checking and upgrading

### Implementation Architecture

#### 1. Command Structure

**File**: `crates/vite_task/src/lib.rs`

Add new command variant:

```rust
#[derive(Subcommand, Debug)]
pub enum Commands {
    // ... existing commands

    /// Check for outdated packages
    #[command(disable_help_flag = true)]
    Outdated {
        /// Package name(s) to check (supports glob patterns in pnpm)
        #[arg(value_name = "PACKAGE")]
        packages: Vec<String>,

        /// Show extended information
        #[arg(long)]
        long: bool,

        /// Output format: table (default), list, or json
        /// Maps to: pnpm: --format <format>, npm: --json/--parseable, yarn@1: --json
        #[arg(long, value_name = "FORMAT")]
        format: Option<String>,

        /// Check recursively across all workspaces
        /// Maps to: pnpm: -r, npm: --all
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Filter packages in monorepo (can be used multiple times)
        /// Maps to: pnpm: --filter <pattern>, npm: --workspace <pattern>
        #[arg(long, value_name = "PATTERN")]
        filter: Vec<String>,

        /// Include workspace root
        /// Maps to: pnpm: -w/--workspace-root, npm: --include-workspace-root
        #[arg(short = 'w', long)]
        workspace_root: bool,

        /// Only production and optional dependencies (pnpm-specific)
        #[arg(short = 'P', long)]
        prod: bool,

        /// Only dev dependencies (pnpm-specific)
        #[arg(short = 'D', long)]
        dev: bool,

        /// Exclude optional dependencies (pnpm-specific)
        #[arg(long)]
        no_optional: bool,

        /// Only show compatible versions (pnpm-specific)
        #[arg(long)]
        compatible: bool,

        /// Sort results by field (pnpm-specific)
        #[arg(long, value_name = "FIELD")]
        sort_by: Option<String>,

        /// Check globally installed packages
        #[arg(short = 'g', long)]
        global: bool,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },
}
```

#### 2. Package Manager Adapter

**File**: `crates/vite_package_manager/src/commands/outdated.rs` (new file)

```rust
use std::{collections::HashMap, process::ExitStatus};

use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env, run_command,
};

#[derive(Debug, Default)]
pub struct OutdatedCommandOptions<'a> {
    pub packages: &'a [String],
    pub long: bool,
    pub format: Option<&'a str>,
    pub recursive: bool,
    pub filters: Option<&'a [String]>,
    pub workspace_root: bool,
    pub prod: bool,
    pub dev: bool,
    pub no_optional: bool,
    pub compatible: bool,
    pub sort_by: Option<&'a str>,
    pub global: bool,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the outdated command with the package manager.
    #[must_use]
    pub async fn run_outdated_command(
        &self,
        options: &OutdatedCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_outdated_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the outdated command.
    #[must_use]
    pub fn resolve_outdated_command(&self, options: &OutdatedCommandOptions) -> ResolveCommandResult {
        let bin_name: String;
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        // Global packages should use npm cli only
        if options.global {
            bin_name = "npm".into();
            args.push("outdated".into());
            args.push("-g".into());
            args.extend_from_slice(options.packages);
            if let Some(pass_through_args) = options.pass_through_args {
                args.extend_from_slice(pass_through_args);
            }
            return ResolveCommandResult { bin_path: bin_name, args, envs };
        }

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

                args.push("outdated".into());

                // Handle format option
                if let Some(format) = options.format {
                    args.push("--format".into());
                    args.push(format.into());
                }

                if options.long {
                    args.push("--long".into());
                }

                if options.workspace_root {
                    args.push("--workspace-root".into());
                }

                if options.recursive {
                    args.push("--recursive".into());
                }

                if options.prod {
                    args.push("--prod".into());
                }

                if options.dev {
                    args.push("--dev".into());
                }

                if options.no_optional {
                    args.push("--no-optional".into());
                }

                if options.compatible {
                    args.push("--compatible".into());
                }

                if let Some(sort_by) = options.sort_by {
                    args.push("--sort-by".into());
                    args.push(sort_by.into());
                }

                if options.global {
                    args.push("--global".into());
                }

                // Add packages (pnpm supports glob patterns)
                args.extend_from_slice(options.packages);
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();

                // Check if yarn@2+ (uses upgrade-interactive)
                if !self.version.starts_with("1.") {
                    println!("Note: yarn@2+ uses 'yarn upgrade-interactive' for checking outdated packages");
                    args.push("upgrade-interactive".into());

                    // Warn about unsupported flags
                    if options.format.is_some() {
                        println!("Warning: --format not supported by yarn@2+");
                    }
                } else {
                    // yarn@1
                    args.push("outdated".into());

                    // Add packages (yarn@1 supports package names)
                    args.extend_from_slice(options.packages);

                    // yarn@1 supports --json format
                    if let Some(format) = options.format {
                        if format == "json" {
                            args.push("--json".into());
                        } else {
                            println!("Warning: yarn@1 only supports json format, not {}", format);
                        }
                    }
                }

                // Common warnings
                if options.long {
                    println!("Warning: --long not supported by yarn");
                }
                if options.workspace_root {
                    println!("Warning: --workspace-root not supported by yarn");
                }
                if options.recursive {
                    println!("Warning: --recursive not supported by yarn");
                }
                if let Some(filters) = options.filters {
                    if !filters.is_empty() {
                        println!("Warning: --filter not supported by yarn");
                    }
                }
                if options.prod || options.dev {
                    println!("Warning: --prod/--dev not supported by yarn");
                }
                if options.no_optional {
                    println!("Warning: --no-optional not supported by yarn");
                }
                if options.compatible {
                    println!("Warning: --compatible not supported by yarn");
                }
                if options.sort_by.is_some() {
                    println!("Warning: --sort-by not supported by yarn");
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();
                args.push("outdated".into());

                // npm format flags - translate from --format
                if let Some(format) = options.format {
                    match format {
                        "json" => args.push("--json".into()),
                        "list" => args.push("--parseable".into()),
                        "table" => {}, // Default, no flag needed
                        _ => println!("Warning: npm only supports formats: json, list, table"),
                    }
                }

                if options.long {
                    args.push("--long".into());
                }

                // npm workspace flags - translate from --filter
                if let Some(filters) = options.filters {
                    for filter in filters {
                        args.push("--workspace".into());
                        args.push(filter.clone());
                    }
                }

                // npm uses --include-workspace-root when workspace_root is set
                if options.workspace_root {
                    args.push("--include-workspace-root".into());
                }

                // npm --all translates from -r/--recursive
                if options.recursive {
                    args.push("--all".into());
                }

                if options.global {
                    args.push("--global".into());
                }

                // Add packages (npm supports package names)
                args.extend_from_slice(options.packages);

                // Warn about pnpm-specific flags
                if options.prod || options.dev {
                    println!("Warning: --prod/--dev not supported by npm");
                }
                if options.no_optional {
                    println!("Warning: --no-optional not supported by npm");
                }
                if options.compatible {
                    println!("Warning: --compatible not supported by npm");
                }
                if options.sort_by.is_some() {
                    println!("Warning: --sort-by not supported by npm");
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

Update to include outdated module:

```rust
pub mod add;
mod install;
pub mod remove;
pub mod update;
pub mod link;
pub mod unlink;
pub mod dedupe;
pub mod why;
pub mod outdated;  // Add this line
```

#### 3. Outdated Command Implementation

**File**: `crates/vite_task/src/outdated.rs` (new file)

```rust
use vite_error::Error;
use vite_path::AbsolutePathBuf;
use vite_package_manager::{
    PackageManager,
    commands::outdated::OutdatedCommandOptions,
};
use vite_workspace::Workspace;

pub struct OutdatedCommand {
    workspace_root: AbsolutePathBuf,
}

impl OutdatedCommand {
    pub fn new(workspace_root: AbsolutePathBuf) -> Self {
        Self { workspace_root }
    }

    pub async fn execute(
        self,
        packages: Vec<String>,
        long: bool,
        format: Option<String>,
        recursive: bool,
        filters: Vec<String>,
        prod: bool,
        dev: bool,
        no_optional: bool,
        compatible: bool,
        sort_by: Option<String>,
        global: bool,
        extra_args: Vec<String>,
    ) -> Result<ExecutionSummary, Error> {
        let package_manager = PackageManager::builder(&self.workspace_root).build().await?;
        let workspace = Workspace::partial_load(self.workspace_root)?;

        // Build outdated command options
        let outdated_options = OutdatedCommandOptions {
            packages: &packages,
            long,
            format: format.as_deref(),
            recursive,
            filters: if filters.is_empty() { None } else { Some(&filters) },
            prod,
            dev,
            no_optional,
            compatible,
            sort_by: sort_by.as_deref(),
            global,
            pass_through_args: if extra_args.is_empty() { None } else { Some(&extra_args) },
        };

        let exit_status = package_manager
            .run_outdated_command(&outdated_options, &workspace.root)
            .await?;

        // Note: outdated command may exit with code 1 if outdated packages are found
        // This is expected behavior, not an error
        if !exit_status.success() {
            let exit_code = exit_status.code();
            // Exit code 1 typically means outdated packages found, which is OK
            if exit_code != Some(1) {
                return Err(Error::CommandFailed {
                    command: "outdated".to_string(),
                    exit_code,
                });
            }
        }

        workspace.unload().await?;

        Ok(ExecutionSummary::default())
    }
}
```

## Design Decisions

### 1. No Caching

**Decision**: Do not cache outdated operations.

**Rationale**:

- `outdated` queries remote registry for latest versions
- Results change frequently as new versions are published
- Caching would provide stale information
- Users expect fresh data when checking for updates

### 2. Pattern Support

**Decision**: Accept patterns but warn when package manager doesn't support glob patterns.

**Rationale**:

- pnpm supports glob patterns: `pnpm outdated "*gulp-*" @babel/core`
- npm and yarn accept package names but not glob patterns
- Warn users about limited pattern support
- Better UX than erroring

### 3. Exit Code Handling

**Decision**: Don't treat exit code 1 as an error for outdated command.

**Rationale**:

- Package managers return exit code 1 when outdated packages are found
- This is expected behavior, not a failure
- Only treat other exit codes as errors
- Matches package manager semantics

### 4. Output Format Support

**Decision**: Support pnpm's `--format` flag and npm's `--json`/`--parseable` flags.

**Rationale**:

- pnpm has `--format` with table/list/json options
- npm has separate `--json` and `--parseable` flags
- yarn@1 has fixed table output
- yarn@2+ uses interactive mode
- Translate flags appropriately per package manager

### 5. Workspace Filtering

**Decision**: Support both pnpm's `--filter` and npm's `--workspace` patterns.

**Rationale**:

- Different package managers use different flags
- Translate flags appropriately
- Warn when flag not supported
- Consistent with other Vite+ commands

### 6. Dependency Type Filtering

**Decision**: Support pnpm's `--prod`, `--dev`, `--no-optional` flags with warnings.

**Rationale**:

- pnpm allows filtering by dependency type
- Not available in npm or yarn
- Useful for focused updates
- Warn when not supported

### 7. Yarn@2+ Behavior

**Decision**: Use `upgrade-interactive` for yarn@2+ instead of `outdated`.

**Rationale**:

- yarn@2+ recommends `upgrade-interactive` for checking updates
- Provides interactive UI instead of simple table
- Different paradigm but achieves same goal
- Inform users about different behavior

## Error Handling

### No Package Manager Detected

```bash
$ vite outdated
Error: No package manager detected
Please run one of:
  - vite install (to set up package manager)
  - Add packageManager field to package.json
```

### Invalid Format Option

```bash
$ vite outdated --format invalid
Error: Invalid format 'invalid'
Valid formats: table, list, json
```

### Unsupported Flag Warning

```bash
$ vite outdated --prod
Detected package manager: npm@11.0.0
Warning: --prod not supported by npm
Running: npm outdated
```

## User Experience

### Success Output (pnpm)

```bash
$ vite outdated
Detected package manager: pnpm@10.15.0
Running: pnpm outdated

Package         Current  Wanted  Latest
react           18.2.0   18.3.1  18.3.1
lodash          4.17.20  4.17.21 4.17.21
@babel/core     7.20.0   7.20.12 7.25.8

Done in 1.2s
```

### Success Output (npm)

```bash
$ vite outdated
Detected package manager: npm@11.0.0
Running: npm outdated

Package         Current  Wanted  Latest  Location             Depended by
react           18.2.0   18.3.1  18.3.1  node_modules/react   my-app
lodash          4.17.20  4.17.21 4.17.21 node_modules/lodash  my-app

Done in 0.8s
```

### Success Output (yarn@1)

```bash
$ vite outdated
Detected package manager: yarn@1.22.19
Running: yarn outdated

Package         Current  Wanted  Latest  Package Type  URL
react           18.2.0   18.3.1  18.3.1  dependencies  https://...
lodash          4.17.20  4.17.21 4.17.21 dependencies  https://...

Done in 1.0s
```

### JSON Output (pnpm)

```bash
$ vite outdated --format json
Detected package manager: pnpm@10.15.0
Running: pnpm outdated --format json

[
  {
    "packageName": "react",
    "current": "18.2.0",
    "wanted": "18.3.1",
    "latest": "18.3.1",
    "dependencyType": "dependencies"
  },
  {
    "packageName": "lodash",
    "current": "4.17.20",
    "wanted": "4.17.21",
    "latest": "4.17.21",
    "dependencyType": "dependencies"
  }
]

Done in 1.1s
```

### Pattern Matching (pnpm)

```bash
$ vite outdated "*babel*" "eslint-*"
Detected package manager: pnpm@10.15.0
Running: pnpm outdated "*babel*" "eslint-*"

Package              Current  Wanted   Latest
@babel/core          7.20.0   7.20.12  7.25.8
@babel/preset-env    7.20.0   7.20.12  7.25.8
eslint-config-next   13.0.0   13.0.7   14.2.5
eslint-plugin-react  7.32.0   7.32.2   7.37.2

Done in 1.3s
```

### Workspace Filtering (pnpm)

```bash
$ vite outdated --filter app -r
Detected package manager: pnpm@10.15.0
Running: pnpm --filter app outdated --recursive

Scope: app

Package         Current  Wanted  Latest
react           18.2.0   18.3.1  18.3.1
react-dom       18.2.0   18.3.1  18.3.1

Done in 1.0s
```

## Alternative Designs Considered

### Alternative 1: Always Error on Exit Code 1

```bash
vite outdated
# Exit code 1 when outdated packages found
# Treat as error
```

**Rejected because**:

- Outdated packages found is normal, not an error
- Would break CI/CD workflows
- Matches package manager behavior
- Users expect exit code 1 to indicate packages need updating

### Alternative 2: Custom Output Format

```bash
vite outdated --format vite
# Custom unified format across all package managers
```

**Rejected because**:

- Output format parsing is fragile
- Different package managers provide different data
- Better to pass through native output
- Let users see familiar format from their package manager

### Alternative 3: Auto-Update Option

```bash
vite outdated --update
# Automatically update all outdated packages
```

**Rejected because**:

- Mixing check and update is dangerous
- Users should review before updating
- Separate `vite update` command exists
- Keep commands focused on single purpose

## Implementation Plan

### Phase 1: Core Functionality

1. Add `Outdated` command variant to `Commands` enum
2. Create `outdated.rs` module in both crates
3. Implement package manager command resolution
4. Handle exit code 1 as success case
5. Add basic error handling

### Phase 2: Advanced Features

1. Implement output format options (json, table, list, parseable)
2. Add workspace filtering support
3. Implement dependency type filtering (prod, dev)
4. Add pattern matching support
5. Handle yarn@2+ interactive mode

### Phase 3: Testing

1. Unit tests for command resolution
2. Test pattern matching (pnpm)
3. Test workspace operations
4. Test output format options
5. Test exit code handling
6. Integration tests with mock package managers

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
fn test_pnpm_outdated_basic() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.resolve_outdated_command(&OutdatedCommandOptions {
        ..Default::default()
    });
    assert_eq!(args, vec!["outdated"]);
}

#[test]
fn test_pnpm_outdated_with_packages() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.resolve_outdated_command(&OutdatedCommandOptions {
        packages: &["*babel*".to_string(), "eslint-*".to_string()],
        ..Default::default()
    });
    assert_eq!(args, vec!["outdated", "*babel*", "eslint-*"]);
}

#[test]
fn test_pnpm_outdated_json() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.resolve_outdated_command(&OutdatedCommandOptions {
        format: Some("json"),
        ..Default::default()
    });
    assert_eq!(args, vec!["outdated", "--format", "json"]);
}

#[test]
fn test_npm_outdated_basic() {
    let pm = PackageManager::mock(PackageManagerType::Npm);
    let args = pm.resolve_outdated_command(&OutdatedCommandOptions {
        ..Default::default()
    });
    assert_eq!(args, vec!["outdated"]);
}

#[test]
fn test_npm_outdated_json() {
    let pm = PackageManager::mock(PackageManagerType::Npm);
    let args = pm.resolve_outdated_command(&OutdatedCommandOptions {
        format: Some("json"),
        ..Default::default()
    });
    assert_eq!(args, vec!["outdated", "--json"]);
}

#[test]
fn test_yarn_outdated_basic() {
    let pm = PackageManager::mock(PackageManagerType::Yarn);
    let args = pm.resolve_outdated_command(&OutdatedCommandOptions {
        ..Default::default()
    });
    assert_eq!(args, vec!["outdated"]);
}

#[test]
fn test_pnpm_outdated_with_filter() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.resolve_outdated_command(&OutdatedCommandOptions {
        filters: Some(&["app".to_string()]),
        recursive: true,
        ..Default::default()
    });
    assert_eq!(args, vec!["--filter", "app", "outdated", "--recursive"]);
}

#[test]
fn test_pnpm_outdated_prod_only() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.resolve_outdated_command(&OutdatedCommandOptions {
        prod: true,
        ..Default::default()
    });
    assert_eq!(args, vec!["outdated", "--prod"]);
}
```

### Integration Tests

Create fixtures for testing with each package manager:

```
fixtures/outdated-test/
  pnpm-workspace.yaml
  package.json (with some outdated deps)
  packages/
    app/
      package.json (with outdated deps)
    utils/
      package.json (with outdated deps)
  test-steps.json
```

Test cases:

1. Basic outdated check
2. Pattern matching (pnpm only)
3. JSON output
4. Workspace-specific outdated
5. Recursive workspace checking
6. Dependency type filtering
7. Compatible versions only
8. Global package checking
9. Warning messages for unsupported flags
10. Exit code 1 handling (outdated found)

## CLI Help Output

```bash
$ vite outdated --help
Check for outdated packages

Usage: vite outdated [PACKAGE]... [OPTIONS]

Arguments:
  [PACKAGE]...           Package name(s) to check (pnpm supports glob patterns)

Options:
  --long                 Show extended information
  --format <FORMAT>      Output format: table, list, or json
                         Maps to: pnpm: --format <format>, npm: --json/--parseable, yarn@1: --json
  -r, --recursive        Check recursively across all workspaces
                         Maps to: pnpm: -r, npm: --all
  --filter <PATTERN>     Filter packages in monorepo (can be used multiple times)
                         Maps to: pnpm: --filter <pattern>, npm: --workspace <pattern>
  -w, --workspace-root   Include workspace root
                         Maps to: pnpm: -w/--workspace-root, npm: --include-workspace-root
  -P, --prod             Only production and optional dependencies (pnpm only)
  -D, --dev              Only dev dependencies (pnpm only)
  --no-optional          Exclude optional dependencies (pnpm only)
  --compatible           Only show compatible versions (pnpm only)
  --sort-by <FIELD>      Sort results by field (pnpm only, supports 'name')
  -g, --global           Check globally installed packages
  -h, --help             Print help

Package Manager Behavior:
  pnpm:    Shows current, wanted, and latest versions in table format
  npm:     Shows current, wanted, latest, location, and depended by
  yarn@1:  Shows package info with current, wanted, latest, and URL
  yarn@2+: Uses interactive 'upgrade-interactive' command

Exit Codes:
  0: No outdated packages found
  1: Outdated packages found (not an error)
  Other: Command failed

Examples:
  vite outdated                        # Check all packages
  vite outdated react                  # Check specific package
  vite outdated "*babel*" "eslint-*"   # Check with patterns (pnpm)
  vite outdated --format json          # JSON output
  vite outdated --long                 # Verbose output
  vite outdated -r                     # Recursive across workspaces
  vite outdated --filter app           # Check in specific workspace
  vite outdated -w                     # Include workspace root (pnpm)
  vite outdated -w -r                  # Include workspace root and recursive (pnpm)
  vite outdated --prod                 # Only production deps (pnpm)
  vite outdated --compatible           # Only compatible versions (pnpm)
  vite outdated --sort-by name         # Sort results by name (pnpm)
  vite outdated -g                     # Check global packages
```

## Performance Considerations

1. **No Caching**: Queries remote registry, caching would be stale
2. **Network Dependent**: Performance depends on registry response time
3. **Parallel Checks**: Some package managers parallelize version checks
4. **JSON Output**: Faster to parse programmatically than table format

## Security Considerations

1. **Read-Only**: Only queries package versions, no modifications
2. **Registry Trust**: Relies on package registry for version information
3. **Vulnerability Detection**: Helps identify packages with known vulnerabilities
4. **Safe for CI**: Can be run safely in CI/CD pipelines
5. **Audit Integration**: Results can inform security audits

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
pnpm outdated
npm outdated
yarn outdated

# New way (works with any package manager)
vite outdated
```

### CI/CD Integration

```yaml
# Check for outdated packages
- run: vite outdated --format json > outdated.json

# Fail build if critical packages are outdated
- run: |
    vite outdated --format json > outdated.json
    # Parse JSON and check for critical packages
    node scripts/check-critical-outdated.js

# Weekly outdated report
- run: vite outdated -r --format json > weekly-outdated-report.json
```

## Real-World Usage Examples

### Checking for Updates

```bash
# Check all packages
vite outdated

# Check specific packages
vite outdated react react-dom

# Check with pattern (pnpm)
vite outdated "@babel/*" "eslint-*"
```

### Production Dependency Updates

```bash
# Only production dependencies (pnpm)
vite outdated --prod

# Check with JSON output for automation
vite outdated --prod --format json > prod-outdated.json
```

### Workspace Analysis

```bash
# Check all workspaces
vite outdated -r

# Check specific workspace
vite outdated --filter app

# Compare workspaces
vite outdated --filter "app*" -r
```

### Compatible Version Updates

```bash
# Only show versions that satisfy package.json (pnpm)
vite outdated --compatible

# Show all possible updates
vite outdated
```

### Global Package Updates

```bash
# Check globally installed packages
vite outdated -g

# Check specific global package
vite outdated -g typescript
```

## Package Manager Compatibility

| Feature             | pnpm               | npm                           | yarn@1           | yarn@2+             | Notes                    |
| ------------------- | ------------------ | ----------------------------- | ---------------- | ------------------- | ------------------------ |
| Basic command       | ✅ `outdated`      | ✅ `outdated`                 | ✅ `outdated`    | ⚠️ `upgrade-int...` | yarn@2+ uses interactive |
| Pattern matching    | ✅ Glob patterns   | ⚠️ Package names              | ⚠️ Package names | ❌ Not supported    | pnpm supports globs      |
| JSON output         | ✅ `--format json` | ✅ `--json`                   | ❌ Not supported | ❌ Not supported    | Different flags          |
| Long output         | ✅ `--long`        | ✅ `--long`                   | ❌ Not supported | ❌ Not supported    | pnpm and npm only        |
| Parseable           | ❌ Not supported   | ✅ `--parseable`              | ❌ Not supported | ❌ Not supported    | npm only                 |
| Recursive           | ✅ `-r`            | ❌ Not supported              | ❌ Not supported | ❌ Not supported    | pnpm only                |
| Workspace filter    | ✅ `--filter`      | ✅ `--workspace`              | ❌ Not supported | ❌ Not supported    | Different flags          |
| Workspace root      | ✅ `-w`            | ✅ `--include-workspace-root` | ❌ Not supported | ❌ Not supported    | Different flags          |
| Dep type filter     | ✅ `--prod/--dev`  | ❌ Not supported              | ❌ Not supported | ❌ Not supported    | pnpm only                |
| Compatible only     | ✅ `--compatible`  | ❌ Not supported              | ❌ Not supported | ❌ Not supported    | pnpm only                |
| Sort results        | ✅ `--sort-by`     | ❌ Not supported              | ❌ Not supported | ❌ Not supported    | pnpm only                |
| Global check        | ✅ `-g`            | ✅ `-g`                       | ❌ Not supported | ❌ Not supported    | pnpm and npm             |
| Show all transitive | ⚠️ Use `-r`        | ✅ `--all`                    | ❌ Not supported | ❌ Not supported    | Different approaches     |

## Future Enhancements

### 1. Severity Indicators

Show update severity based on semver:

```bash
vite outdated --with-severity

Package         Current  Wanted  Latest  Severity
react           18.2.0   18.3.1  18.3.1  Minor
lodash          4.17.20  4.17.21 4.17.21 Patch
webpack         5.0.0    5.0.0   6.0.0   Major ⚠️
```

### 2. Security Integration

Integrate with security advisories:

```bash
vite outdated --format json --with-security

Package         Current  Latest  Security
lodash          4.17.20  4.17.21 🔴 High severity vulnerability
axios           0.21.0   1.7.0   🟡 Moderate severity issue
react           18.2.0   18.3.1  ✅ No known issues
```

### 3. Update Plan Generation

Generate update plan with dependency analysis:

```bash
vite outdated --format json --plan > update-plan.json

# Output:
{
  "safeUpdates": ["lodash@4.17.21", "react@18.3.1"],
  "breakingUpdates": ["webpack@6.0.0"],
  "blockedBy": {
    "webpack": ["babel-loader requires webpack@5"]
  }
}
```

### 4. Interactive Mode

Add interactive selection mode for all package managers:

```bash
vite outdated --interactive

# Shows interactive UI:
┌─ Outdated Packages ────────────────────┐
│ [x] react       18.2.0 → 18.3.1       │
│ [x] lodash      4.17.20 → 4.17.21     │
│ [ ] webpack     5.0.0 → 6.0.0 (major) │
└────────────────────────────────────────┘
Press <space> to select, <enter> to update
```

### 5. Change Log Integration

Show change logs for updates:

```bash
vite outdated --with-changelog

Package: react 18.2.0 → 18.3.1
Changes:
- Fix: Memory leak in useEffect
- Feat: New useDeferredValue hook
- Perf: Improved rendering performance
```

## Open Questions

1. **Should we handle exit code 1 differently?**
   - Proposed: No, treat as success when outdated packages found
   - Matches package manager behavior
   - Expected by users

2. **Should we add a --fix flag to auto-update?**
   - Proposed: No, use separate `vite update` command
   - Keep commands focused
   - Prevents accidental updates

3. **Should we support custom output formats?**
   - Proposed: No, use native package manager output
   - Simpler implementation
   - Familiar to users
   - Can add in future if needed

4. **Should we cache registry queries?**
   - Proposed: No, always query fresh data
   - Registry data changes frequently
   - Users expect current information

5. **Should we support yarn@2+ differently?**
   - Proposed: Yes, use `upgrade-interactive`
   - Matches yarn@2+ recommendations
   - Provide note to users about different UI

## Success Metrics

1. **Adoption**: % of users using `vite outdated` vs direct package manager
2. **Update Frequency**: How often users update packages after checking
3. **CI Integration**: Usage in CI/CD for outdated checks
4. **User Feedback**: Survey/issues about command usefulness
5. **Security Impact**: Reduction in outdated packages with vulnerabilities

## Conclusion

This RFC proposes adding `vite outdated` command to provide a unified interface for checking outdated packages across pnpm/npm/yarn. The design:

- ✅ Automatically adapts to detected package manager
- ✅ Supports pattern matching (pnpm) with graceful degradation
- ✅ Full pnpm feature support (format, filters, compatible, sorting)
- ✅ npm and yarn compatibility with appropriate warnings
- ✅ Workspace-aware operations
- ✅ Multiple output formats (json, table, list, parseable)
- ✅ Proper exit code handling (1 = outdated found)
- ✅ No caching (always fresh data)
- ✅ Security-conscious (helps identify vulnerable packages)
- ✅ Simple implementation leveraging existing infrastructure
- ✅ Extensible for future enhancements (severity, security, interactive)

The implementation follows the same patterns as other package management commands while providing the dependency update checking features developers need to maintain current, secure dependencies across their projects.
