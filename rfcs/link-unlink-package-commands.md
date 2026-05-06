# RFC: Vite+ Link and Unlink Package Commands

## Summary

Add `vp link` (alias: `vp ln`) and `vp unlink` commands that automatically adapt to the detected package manager (pnpm/yarn/npm/bun) for creating and removing symlinks to local packages, making them accessible system-wide or in other locations. This enables local package development and testing workflows.

## Motivation

Currently, developers must manually use package manager-specific commands to link local packages:

```bash
pnpm link --global
pnpm link --global <pkg>
yarn link
yarn link <package>
npm link
npm link <package>
```

This creates friction in local development workflows and requires remembering different syntaxes. A unified interface would:

1. **Simplify local development**: One command works across all package managers
2. **Auto-detection**: Automatically uses the correct package manager
3. **Consistency**: Same syntax regardless of underlying tool
4. **Integration**: Works seamlessly with existing Vite+ features

### Current Pain Points

```bash
# Developer needs to know which package manager is used
pnpm link --global                    # pnpm project - register current package
pnpm link --global react              # pnpm project - link global package
yarn link                             # yarn project - register current package
yarn link react                       # yarn project - link global package
npm link                              # npm project - register current package
npm link react                        # npm project - link global package

# Different unlink commands
pnpm unlink --global
pnpm unlink --global react
yarn unlink
yarn unlink react
npm unlink
npm unlink react
```

### Proposed Solution

```bash
# Works for all package managers

# Register current package globally
vp link
vp ln

# Link a global package to current project
vp link react
vp ln lodash

# Link a package from a specific directory
vp link ./packages/my-lib
vp link ../other-project

# Workspace operations
vp link --filter app                # Link in specific package
vp link react --filter "app*"       # Link in multiple packages

# Unlink operations
vp unlink                           # Unlink current package
vp unlink react                     # Unlink specific package
vp unlink --filter app              # Unlink in specific workspace
```

## Proposed Solution

### Command Syntax

#### Link Command

```bash
vp link [PACKAGE]
vp ln [PACKAGE]        # Alias
```

**Examples:**

```bash
# Register current package globally (make it linkable)
vp link
vp ln

# Link a global package to current project
vp link react
vp link @types/node

# Link a local directory as a package
vp link ./packages/utils
vp link ../my-other-project
```

#### Unlink Command

```bash
vp unlink [PACKAGE] [OPTIONS]
```

**Examples:**

```bash
# Unregister current package from global
vp unlink

# Unlink a package from current project
vp unlink react
vp unlink @types/node

# Unlink in every workspace package (pnpm only)
vp unlink --recursive
vp unlink -r
```

### Command Mapping

#### Link Command Mapping

**pnpm references:**

- https://pnpm.io/cli/link
- pnpm link creates symlinks to local packages or links global packages

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/link (yarn@1)
- https://yarnpkg.com/cli/link (yarn@2+)
- yarn link registers/links packages

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-link
- npm link creates symlinks between packages

**bun references:**

- https://bun.sh/docs/cli/link
- bun link creates symlinks for local packages

| Vite+ Command   | pnpm              | yarn@1            | yarn@2+           | npm              | bun              | Description                                             |
| --------------- | ----------------- | ----------------- | ----------------- | ---------------- | ---------------- | ------------------------------------------------------- |
| `vp link`       | `pnpm link`       | `yarn link`       | `yarn link`       | `npm link`       | `bun link`       | Register current package or link to local directory     |
| `vp link <pkg>` | `pnpm link <pkg>` | `yarn link <pkg>` | `yarn link <pkg>` | `npm link <pkg>` | `bun link <pkg>` | Links package to current project                        |
| `vp link <dir>` | `pnpm link <dir>` | `yarn link <dir>` | `yarn link <dir>` | `npm link <dir>` | `bun link <dir>` | Links package from `<dir>` directory to current project |

#### Unlink Command Mapping

**pnpm references:**

- https://pnpm.io/cli/unlink
- Unlinks packages from node_modules and removes global links

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/unlink (yarn@1)
- https://yarnpkg.com/cli/unlink (yarn@2+)
- Unlinks previously linked packages

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-uninstall
- npm unlink removes symlinks

| Vite+ Command           | pnpm                      | yarn@1              | yarn@2+             | npm                | bun          | Description                        |
| ----------------------- | ------------------------- | ------------------- | ------------------- | ------------------ | ------------ | ---------------------------------- |
| `vp unlink`             | `pnpm unlink`             | `yarn unlink`       | `yarn unlink`       | `npm unlink`       | `bun unlink` | Unlinks current package            |
| `vp unlink <pkg>`       | `pnpm unlink <pkg>`       | `yarn unlink <pkg>` | `yarn unlink <pkg>` | `npm unlink <pkg>` | `bun unlink` | Unlinks specific package           |
| `vp unlink --recursive` | `pnpm unlink --recursive` | N/A                 | `yarn unlink --all` | N/A                | N/A          | Unlinks in every workspace package |

### Link/Unlink Behavior Differences Across Package Managers

#### pnpm

**Link behavior:**

- `pnpm link`: Links current package dependencies to local directory
- `pnpm link <pkg>`: Links a package to current project (searches globally and locally)
- `pnpm link <dir>`: Links a local directory directly (no global registration)

**Unlink behavior:**

- `pnpm unlink`: Unlinks current package dependencies (removes symlinks)
- `pnpm unlink <pkg>`: Unlinks specific package
- `pnpm unlink --global`: Unlinks current package from global store

#### yarn

**Link behavior (yarn@1):**

- `yarn link`: Registers current package globally
- `yarn link <pkg>`: Links a global package to current project
- No direct directory linking (need to `yarn link` in target first)

**Link behavior (yarn@2+):**

- `yarn link`: Creates link for current package
- `yarn link <pkg>`: Links package
- `yarn link <dir>`: Links local directory

**Unlink behavior:**

- `yarn unlink`: Unlinks current package
- `yarn unlink <pkg>`: Unlinks specific package

#### npm

**Link behavior:**

- `npm link`: Creates global symlink to current package
- `npm link <pkg>`: Links global package to current project
- `npm link <dir>`: Links local directory package

**Unlink behavior:**

- `npm unlink`: Removes global symlink for current package
- `npm unlink <pkg>`: Removes package from current project

#### bun

**Link behavior:**

- `bun link`: Registers current package as a linkable package
- `bun link <pkg>`: Links a registered package to current project
- `--save`: Adds `link:` prefix to package.json dependency entry

**Unlink behavior:**

- `bun unlink`: Unlinks current package

### Implementation Architecture

#### 1. Command Structure

**File**: `crates/vite_task/src/lib.rs`

Add new command variants:

```rust
#[derive(Subcommand, Debug)]
pub enum Commands {
    // ... existing commands

    /// Link packages for local development
    #[command(disable_help_flag = true, alias = "ln")]
    Link {
        /// Package name or directory to link
        /// If empty, registers current package globally
        package: Option<String>,

        /// Arguments to pass to package manager
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Unlink packages
    #[command(disable_help_flag = true)]
    Unlink {
        /// Package name to unlink
        /// If empty, unlinks current package globally
        package: Option<String>,

        /// Unlink in every workspace package (pnpm only)
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Arguments to pass to package manager
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
}
```

#### 2. Package Manager Adapter

**File**: `crates/vite_package_manager/src/commands/link.rs` (new file)

```rust
use std::{collections::HashMap, process::ExitStatus};

use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env, run_command,
};

#[derive(Debug, Default)]
pub struct LinkCommandOptions<'a> {
    pub package: Option<&'a str>,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the link command with the package manager.
    #[must_use]
    pub async fn run_link_command(
        &self,
        options: &LinkCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_link_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the link command.
    #[must_use]
    pub fn resolve_link_command(&self, options: &LinkCommandOptions) -> ResolveCommandResult {
        let bin_name: String;
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                bin_name = "pnpm".into();
                args.push("link".into());
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();
                args.push("link".into());
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();
                args.push("link".into());
            }
        }

        // Add package/directory if specified
        if let Some(package) = options.package {
            args.push(package.to_string());
        }

        // Add pass-through args
        if let Some(pass_through_args) = options.pass_through_args {
            args.extend_from_slice(pass_through_args);
        }

        ResolveCommandResult { bin_path: bin_name, args, envs }
    }
}
```

**File**: `crates/vite_package_manager/src/commands/unlink.rs` (new file)

```rust
use std::{collections::HashMap, process::ExitStatus};

use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env, run_command,
};

#[derive(Debug, Default)]
pub struct UnlinkCommandOptions<'a> {
    pub package: Option<&'a str>,
    pub recursive: bool,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the unlink command with the package manager.
    #[must_use]
    pub async fn run_unlink_command(
        &self,
        options: &UnlinkCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_unlink_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the unlink command.
    #[must_use]
    pub fn resolve_unlink_command(&self, options: &UnlinkCommandOptions) -> ResolveCommandResult {
        let bin_name: String;
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                bin_name = "pnpm".into();
                args.push("unlink".into());

                if options.recursive {
                    args.push("--recursive".into());
                }
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();
                args.push("unlink".into());

                if options.recursive {
                    args.push("--all".into());
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();
                args.push("unlink".into());

                if options.recursive {
                    println!("Warning: npm doesn't support --recursive for unlink command");
                }
            }
        }

        // Add package if specified
        if let Some(package) = options.package {
            args.push(package.to_string());
        }

        // Add pass-through args
        if let Some(pass_through_args) = options.pass_through_args {
            args.extend_from_slice(pass_through_args);
        }

        ResolveCommandResult { bin_path: bin_name, args, envs }
    }
}
```

#### 3. Link Command Implementation

**File**: `crates/vite_task/src/link.rs` (new file)

```rust
pub struct LinkCommand {
    workspace_root: AbsolutePathBuf,
}

impl LinkCommand {
    pub fn new(workspace_root: AbsolutePathBuf) -> Self {
        Self { workspace_root }
    }

    pub async fn execute(
        self,
        package: Option<String>,
        extra_args: Vec<String>,
    ) -> Result<ExecutionSummary, Error> {
        let package_manager = PackageManager::builder(&self.workspace_root).build().await?;
        let workspace = Workspace::partial_load(self.workspace_root)?;

        let resolve_command = package_manager.resolve_command();

        // Build link command options
        let link_options = LinkCommandOptions {
            package: package.as_deref(),
            pass_through_args: if extra_args.is_empty() { None } else { Some(&extra_args) },
        };

        let full_args = package_manager.build_link_args(&link_options);

        let resolved_task = ResolvedTask::resolve_from_builtin_with_command_result(
            &workspace,
            "link",
            full_args.iter().map(String::as_str),
            ResolveCommandResult {
                bin_path: resolve_command.bin_path,
                envs: resolve_command.envs,
            },
            false,
        )?;

        let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
        task_graph.add_node(resolved_task);
        let summary = ExecutionPlan::plan(task_graph, false)?.execute(&workspace).await?;
        workspace.unload().await?;

        Ok(summary)
    }
}
```

#### 4. Unlink Command Implementation

**File**: `crates/vite_task/src/unlink.rs` (new file)

```rust
pub struct UnlinkCommand {
    workspace_root: AbsolutePathBuf,
}

impl UnlinkCommand {
    pub fn new(workspace_root: AbsolutePathBuf) -> Self {
        Self { workspace_root }
    }

    pub async fn execute(
        self,
        package: Option<String>,
        recursive: bool,
        extra_args: Vec<String>,
    ) -> Result<ExecutionSummary, Error> {
        let package_manager = PackageManager::builder(&self.workspace_root).build().await?;
        let workspace = Workspace::partial_load(self.workspace_root)?;

        let resolve_command = package_manager.resolve_command();

        // Build unlink command options
        let unlink_options = UnlinkCommandOptions {
            package: package.as_deref(),
            recursive,
            pass_through_args: if extra_args.is_empty() { None } else { Some(&extra_args) },
        };

        let full_args = package_manager.build_unlink_args(&unlink_options);

        let resolved_task = ResolvedTask::resolve_from_builtin_with_command_result(
            &workspace,
            "unlink",
            full_args.iter().map(String::as_str),
            ResolveCommandResult {
                bin_path: resolve_command.bin_path,
                envs: resolve_command.envs,
            },
            false,
        )?;

        let mut task_graph: StableGraph<ResolvedTask, ()> = Default::default();
        task_graph.add_node(resolved_task);
        let summary = ExecutionPlan::plan(task_graph, false)?.execute(&workspace).await?;
        workspace.unload().await?;

        Ok(summary)
    }
}
```

## Design Decisions

### 1. No Caching

**Decision**: Do not cache link/unlink operations.

**Rationale**:

- These commands create/remove symlinks
- Side effects make caching inappropriate
- Each execution should run fresh
- Similar to how add/remove/install work

### 2. Local Directory Linking

**Decision**: Support linking local directories directly.

**Rationale**:

- Common use case for monorepo development
- Allows testing packages before publishing
- pnpm, yarn, and npm all support this
- Simpler than global registration workflow

**Example**:

```bash
# Link local package without global registration
vp link ./packages/my-lib
vp link ../other-project/packages/utils
```

### 3. Global vs Local Linking

**Decision**: Support both global registration and local directory linking.

**Rationale**:

- Different workflows need different approaches
- Global: For packages used across multiple projects
- Local: For monorepo/related project development
- Matches native package manager capabilities

### 4. Recursive Unlink Support

**Decision**: Support `--recursive` flag for unlink (pnpm and yarn@2+) with graceful degradation.

**Rationale**:

- pnpm supports `--recursive` flag to unlink in every workspace package
- yarn@2+ supports `--all` flag for similar functionality
- Provides workspace-wide cleanup capability
- Warn users when unavailable on npm and yarn@1
- Consistent with other workspace features

## Error Handling

### No Package Manager Detected

```bash
$ vp link react
Error: No package manager detected
Please run one of:
  - vp install (to set up package manager)
  - Add packageManager field to package.json
```

### Feature Not Supported

```bash
$ vp unlink --recursive
Warning: npm doesn't support --recursive for unlink command
# Proceeds with standard unlink (without --recursive flag)
```

## User Experience

### Link Success Output

```bash
$ vp link
Detected package manager: pnpm@10.15.0
Running: pnpm link --global

+ my-package@1.0.0

Done in 0.5s
```

```bash
$ vp link my-package
Detected package manager: pnpm@10.15.0
Running: pnpm link --global my-package

Packages: +1
+
Progress: resolved 1, reused 0, downloaded 0, added 1, done

dependencies:
+ my-package link:~/.pnpm-store/my-package

Done in 1.2s
```

```bash
$ vp link ./packages/utils
Detected package manager: npm@11.0.0
Running: npm link ./packages/utils

npm WARN EBADENGINE Unsupported engine
added 1 package

Done in 2.1s
```

### Unlink Success Output

```bash
$ vp unlink
Detected package manager: pnpm@10.15.0
Running: pnpm unlink

- my-package@1.0.0

Done in 0.3s
```

```bash
$ vp unlink react
Detected package manager: yarn@4.0.0
Running: yarn unlink react

Removed react

Done in 0.8s
```

## Alternative Designs Considered

### Alternative 1: Separate Global and Local Commands

```bash
vp link:global          # Register globally
vp link:local <dir>     # Link local directory
```

**Rejected because**:

- More commands to remember
- Doesn't match native package manager APIs
- Less intuitive than flag-based approach

### Alternative 2: Auto-Detect Link Type

```bash
vp link              # Auto-detect: global if no package, local if directory
vp link react        # Auto-detect: global package or local directory
```

**Rejected because**:

- Ambiguous behavior
- Hard to predict what will happen
- Explicit flags are clearer

### Alternative 3: Interactive Mode

```bash
$ vp link
? What would you like to link?
  > Register current package globally
    Link a global package
    Link a local directory
```

**Rejected for initial version**:

- Slower for experienced users
- Not scriptable
- Can be added later as optional mode

## Implementation Plan

### Phase 1: Core Functionality

1. Add `Link` and `Unlink` command variants to `Commands` enum
2. Create `link.rs` and `unlink.rs` modules in both crates
3. Implement package manager command resolution
4. Add basic error handling

### Phase 2: Advanced Features

1. Support local directory linking
2. Implement pnpm-specific `--dir` flag
3. Add npm save flags support
4. Handle workspace filtering (pnpm only)

### Phase 3: Testing

1. Unit tests for command resolution
2. Integration tests with mock package managers
3. Test global and local linking
4. Test workspace operations

### Phase 4: Documentation

1. Update CLI documentation
2. Add examples to README
3. Document package manager compatibility
4. Add troubleshooting guide

## Testing Strategy

### Test Package Manager Versions

- pnpm@9.x
- pnpm@10.x
- pnpm@11.x (pass `-- --no-frozen-lockfile` to `vp unlink` under CI=true; see snap-tests `command-unlink-pnpm11`)
- yarn@1.x
- yarn@4.x
- npm@10.x
- npm@11.x
- bun@1.x [WIP]

### Unit Tests

```rust
#[test]
fn test_pnpm_link_no_package() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.resolve_link_command(&LinkCommandOptions {
        package: None,
        ..Default::default()
    });
    assert_eq!(args, vec!["link"]);
}

#[test]
fn test_pnpm_link_package() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.resolve_link_command(&LinkCommandOptions {
        package: Some("react"),
        ..Default::default()
    });
    assert_eq!(args, vec!["link", "react"]);
}

#[test]
fn test_pnpm_link_directory() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.resolve_link_command(&LinkCommandOptions {
        package: Some("./packages/utils"),
        ..Default::default()
    });
    assert_eq!(args, vec!["link", "./packages/utils"]);
}

#[test]
fn test_yarn_link_basic() {
    let pm = PackageManager::mock(PackageManagerType::Yarn);
    let args = pm.resolve_link_command(&LinkCommandOptions {
        package: None,
        ..Default::default()
    });
    assert_eq!(args, vec!["link"]);
}

#[test]
fn test_npm_link_package() {
    let pm = PackageManager::mock(PackageManagerType::Npm);
    let args = pm.resolve_link_command(&LinkCommandOptions {
        package: Some("react"),
        ..Default::default()
    });
    assert_eq!(args, vec!["link", "react"]);
}

#[test]
fn test_pnpm_unlink_no_package() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.resolve_unlink_command(&UnlinkCommandOptions {
        package: None,
        recursive: false,
        ..Default::default()
    });
    assert_eq!(args, vec!["unlink"]);
}

#[test]
fn test_pnpm_unlink_recursive() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.resolve_unlink_command(&UnlinkCommandOptions {
        package: None,
        recursive: true,
        ..Default::default()
    });
    assert_eq!(args, vec!["unlink", "--recursive"]);
}
```

### Integration Tests

Create fixtures for testing with each package manager:

```
fixtures/link-unlink-test/
  pnpm-workspace.yaml
  package.json
  packages/
    lib-a/
      package.json
    lib-b/
      package.json
  test-steps.json
```

Test cases:

1. Link current package globally
2. Link global package to project
3. Link local directory
4. Unlink current package
5. Unlink specific package
6. Unlink with --recursive (pnpm only)
7. Warning for unsupported --recursive on yarn/npm

## CLI Help Output

### Link Command

```bash
$ vp link --help
Link packages for local development

Usage: vp link [PACKAGE]

Aliases: ln

Arguments:
  [PACKAGE]  Package name or directory to link
             If empty, registers current package globally

Options:
  -h, --help             Print help

Link Types:
  Global Registration:   vp link (no package)
  Link Global Package:   vp link <package-name>
  Link Local Directory:  vp link <path>

Examples:
  vp link                        # Register current package globally
  vp ln                          # Same as above (alias)
  vp link react                  # Link global package 'react'
  vp link ./packages/utils       # Link local directory
  vp link ../my-lib              # Link from parent directory
```

### Unlink Command

```bash
$ vp unlink --help
Unlink packages

Usage: vp unlink [PACKAGE] [OPTIONS]

Arguments:
  [PACKAGE]  Package name to unlink
             If empty, unlinks current package globally

Options:
  -r, --recursive        Unlink in every workspace package (pnpm and yarn@2+)
  -h, --help             Print help

Examples:
  vp unlink                      # Unlink current package
  vp unlink react                # Unlink 'react' from current project
  vp unlink --recursive          # Unlink in all workspace packages (pnpm and yarn@2+)
  vp unlink -r                   # Same as above (short form)
```

## Performance Considerations

1. **No Caching**: Operations run directly without cache overhead
2. **Symlink Creation**: Fast operation, minimal performance impact
3. **Single Execution**: Unlike task runner, these are one-off operations
4. **Auto-Detection**: Reuses existing package manager detection (already cached)

## Security Considerations

1. **Symlink Safety**: Symlinks are standard package manager feature
2. **Path Validation**: Validate that directories exist before linking
3. **No Code Execution**: Just creates/removes symlinks via package manager
4. **Global Store**: Respects package manager's global store location

## Backward Compatibility

This is a new feature with no breaking changes:

- Existing commands unaffected
- New commands are additive
- No changes to task configuration
- No changes to caching behavior

## Migration Path

### Adoption

Users can start using immediately:

```bash
# Old way
pnpm link --global
pnpm link --global react

# New way (works with any package manager)
vp link
vp link react
```

### Discoverability

Add to:

- CLI help output
- Documentation
- VSCode extension suggestions
- Shell completions

## Real-World Usage Examples

### Local Package Development

```bash
# Working on a shared library
cd ~/projects/my-monorepo/packages/shared-utils
vp link                           # Register globally

# Use it in another project
cd ~/projects/my-app
vp link shared-utils              # Link the global package

# Or link directly without global registration
cd ~/projects/my-app
vp link ~/projects/my-monorepo/packages/shared-utils
```

### Monorepo Development

```bash
# Unlink in all workspace packages (pnpm only)
vp unlink --recursive             # Unlink current package from all workspaces
vp unlink -r                      # Same as above (short form)
```

### Testing Unpublished Changes

```bash
# Develop a library
cd ~/my-lib
npm version patch
vp link

# Test in consuming project
cd ~/consuming-app
vp link my-lib
npm test

# Unlink when done
vp unlink my-lib
npm install my-lib@latest
```

## Package Manager Compatibility

| Feature              | pnpm                    | yarn@1           | yarn@2+           | npm              | bun              | Notes            |
| -------------------- | ----------------------- | ---------------- | ----------------- | ---------------- | ---------------- | ---------------- |
| Link package/dir     | `link`                  | `link`           | `link`            | `link`           | `link`           | All supported    |
| Link with package    | `link <pkg>`            | `link <pkg>`     | `link <pkg>`      | `link <pkg>`     | `link <pkg>`     | All supported    |
| Link local directory | `link <dir>`            | `link <dir>`     | `link <dir>`      | `link <dir>`     | `link <dir>`     | All supported    |
| Save to package.json | N/A                     | N/A              | N/A               | N/A              | `--save`         | bun-specific     |
| Unlink               | `unlink`                | `unlink`         | `unlink`          | `unlink`         | `unlink`         | All supported    |
| Recursive unlink     | ✅ `unlink --recursive` | ❌ Not supported | ✅ `unlink --all` | ❌ Not supported | ❌ Not supported | pnpm and yarn@2+ |

## Future Enhancements

### 1. Link Status Command

Show which packages are currently linked:

```bash
vp link:status
vp link --list

# Output:
Linked packages:
  react -> ~/.pnpm-global/5/node_modules/react
  my-lib -> ~/projects/my-lib
```

### 2. Auto-Link Workspace Dependencies

Automatically link all workspace dependencies:

```bash
vp link --workspace-deps

# Scans package.json for workspace: protocol dependencies
# and links them automatically
```

### 3. Link Groups

Save and restore link configurations:

```bash
vp link --save-config dev
vp link --load-config dev

# .vite-link.json:
{
  "configs": {
    "dev": {
      "links": [
        { "package": "my-lib", "path": "../my-lib" },
        { "package": "shared-utils", "path": "./packages/utils" }
      ]
    }
  }
}
```

### 4. Link Verification

Verify linked packages are valid:

```bash
vp link --verify

# Checks that all symlinks point to valid directories
# Reports broken links
```

## Open Questions

1. **Should we validate directory existence before linking?**
   - Proposed: Yes, provide clear error if directory doesn't exist
   - Better UX than cryptic package manager errors

2. **Should we support relative paths?**
   - Proposed: Yes, resolve relative paths before passing to package manager
   - Makes commands more intuitive from any location

3. **Should we warn when linking without global registration on yarn/npm?**
   - Proposed: No, this is standard behavior
   - Users expect this workflow

4. **Should we support unlinking all packages at once?**
   - Proposed: Later enhancement, not MVP
   - Use case: "clean slate" before testing

5. **Should we provide better error messages for common issues?**
   - Proposed: Yes, detect common errors and provide helpful suggestions
   - Example: Package not found → "Did you run 'vp link' in the package directory first?"

## Success Metrics

1. **Adoption**: % of users using `vp link/unlink` vs direct package manager
2. **Error Rate**: Track command failures vs package manager direct usage
3. **User Feedback**: Survey/issues about command ergonomics
4. **Performance**: Measure overhead vs direct package manager calls (<100ms target)

## Conclusion

This RFC proposes adding `vp link` and `vp unlink` commands to provide a unified interface for local package development across pnpm/yarn/npm/bun. The design:

- ✅ Automatically adapts to detected package manager
- ✅ Supports both package and local directory linking
- ✅ Minimal options for simplicity (only --recursive for unlink)
- ✅ Consistent behavior across all package managers
- ✅ Clear error messages and warnings
- ✅ No caching overhead
- ✅ Simple implementation leveraging existing infrastructure
- ✅ Extensible for future enhancements

The implementation follows the same patterns as other package manager commands while keeping the interface simple and intuitive for local package development workflows.
