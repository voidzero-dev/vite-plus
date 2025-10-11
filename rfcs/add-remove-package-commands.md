# RFC: Vite+ Add and Remove Package Commands

## Summary

Add `vite add` and `vite remove` commands that automatically adapt to the detected package manager (pnpm/yarn/npm) for adding and removing packages, with support for multiple packages, common flags, and workspace-aware operations based on pnpm's API design.

## Motivation

Currently, developers must manually use package manager-specific commands:

```bash
pnpm add react
yarn add react
npm install react
```

This creates friction in monorepo workflows and requires remembering different syntaxes. A unified interface would:

1. **Simplify workflows**: One command works across all package managers
2. **Auto-detection**: Automatically uses the correct package manager
3. **Consistency**: Same syntax regardless of underlying tool
4. **Integration**: Works seamlessly with existing vite+ features

### Current Pain Points

```bash
# Developer needs to know which package manager is used
pnpm add -D typescript  # pnpm project
yarn add --dev typescript  # yarn project
npm install --save-dev typescript  # npm project

# Different remove commands
pnpm remove lodash
yarn remove lodash
npm uninstall lodash
```

### Proposed Solution

```bash
# Works for all package managers
vite add typescript -D
vite remove lodash

# Multiple packages
vite add react react-dom
vite remove axios lodash

# Workspace operations
vite add react --filter app
vite add @myorg/utils --workspace --filter app
vite add lodash -w  # Add to workspace root
```

## Proposed Solution

### Command Syntax

#### Add Command

```bash
vite add <PACKAGES>... [OPTIONS]
```

**Examples:**

```bash
# Add production dependency
vite add react react-dom

# Add dev dependency
vite add -D typescript @types/react

# Add with exact version
vite add react -E

# Add peer dependency
vite add --save-peer react

# Add optional dependency
vite add -O sharp

# Workspace operations
vite add react --filter app              # Add to specific package
vite add @myorg/utils --workspace --filter app  # Add workspace dependency
vite add lodash -w                       # Add to workspace root
vite add react --filter "app*"           # Add to multiple packages (pattern)
vite add utils --filter "!@myorg/core"   # Exclude packages
```

##### `vite install` Command with `PACKAGES` arguments

To accommodate the user habits and experience of `npm install <PACKAGES>…`, `vite install <PACKAGES>...` will be specially treated as an alias for the add command.

The following commands will be automatically converted to the add command for processing:

```bash
vite install <PACKAGES>... [OPTIONS]

-> vite add <PACKAGES>... [OPTIONS]
```

##### Install global packages with npm cli only

For global packages, we will use npm cli only.

> Because yarn do not support global packages install on [version>=2.x](https://yarnpkg.com/migration/guide#use-yarn-dlx-instead-of-yarn-global), and pnpm global install has some bugs like `wrong bin file` issues.

```bash
vite install -g <PACKAGES>...
vite add -g <PACKAGES>...

-> npm install -g <PACKAGES>...
```

#### Remove Command

```bash
vite remove <PACKAGES>... [OPTIONS]
vite rm <PACKAGES>... [OPTIONS]        # Alias
```

**Examples:**

```bash
# Remove packages
vite remove lodash axios

# Remove dev dependency
vite rm typescript

# Alias support
vite rm old-package

# Workspace operations
vite remove lodash --filter app          # Remove from specific package
vite rm utils --filter "app*"            # Remove from multiple packages
vite remove -g typescript                # Remove global package
```

### Command Mapping

#### Add Command Mapping

- https://pnpm.io/cli/add#options
- https://yarnpkg.com/cli/add#options
- https://docs.npmjs.com/cli/v11/commands/npm-install#description

| Vite+ Flag                           | pnpm                     | yarn                                            | npm                             | Description                                             |
| ------------------------------------ | ------------------------ | ----------------------------------------------- | ------------------------------- | ------------------------------------------------------- |
| `<packages>`                         | `add <packages>`         | `add <packages>`                                | `install <packages>`            | Add packages                                            |
| `--filter <pattern>`                 | `--filter <pattern> add` | `workspaces foreach -A --include <pattern> add` | `install --workspace <pattern>` | Target specific workspace package(s)                    |
| `-w, --workspace-root`               | `-w`                     | `-W` for v1, v2+ N/A                            | `--include-workspace-root`      | Add to workspace root (ignore-workspace-root-check)     |
| `--workspace`                        | `--workspace`            | N/A                                             | N/A                             | Only add if package exists in workspace (pnpm-specific) |
| `-P, --save-prod`                    | `--save-prod` / `-P`     | N/A                                             | `--save-prod` / `-P`            | Save to `dependencies`. The default behavior            |
| `-D, --save-dev`                     | `-D`                     | `--dev` / `-D`                                  | `--save-dev` / `-D`             | Save to `devDependencies`                               |
| `--save-peer`                        | `--save-peer`            | `--peer` / `-P`                                 | `--save-peer`                   | Save to `peerDependencies` and `devDependencies`        |
| `-O, --save-optional`                | `-O`                     | `--optional` / `-O`                             | `--save-optional` / `-O`        | Save to `optionalDependencies`                          |
| `-E, --save-exact`                   | `-E`                     | `--exact` / `-E`                                | `--save-exact` / `-E`           | Save exact version                                      |
| `-g, --global`                       | `-g`                     | `global add`                                    | `--global` / `-g`               | Install globally                                        |
| `--save-catalog`                     | pnpm@10+ only            | N/A                                             | N/A                             | Save the new dependency to the default catalog          |
| `--save-catalog-name <catalog_name>` | pnpm@10+ only            | N/A                                             | N/A                             | Save the new dependency to the specified catalog        |
| `--allow-build <names>`              | pnpm@10+ only            | N/A                                             | N/A                             | A list of package names allowed to run postinstall      |

**Note**: For pnpm, `--filter` must come before the command (e.g., `pnpm --filter app add react`). For yarn/npm, it's integrated into the command structure.

#### Remove Command Mapping

| Vite+ Flag             | pnpm                        | yarn                     | npm                           | Description                                    |
| ---------------------- | --------------------------- | ------------------------ | ----------------------------- | ---------------------------------------------- |
| `<packages>`           | `remove <packages>`         | `remove <packages>`      | `uninstall <packages>`        | Remove packages                                |
| `-D, --save-dev`       | `-D`                        | `--dev` / `-D`           | `--save-dev` / `-D`           | Only remove from `devDependencies`             |
| `-O, --save-optional`  | `-O`                        | `--optional` / `-O`      | `--save-optional` / `-O`      | Only remove from `optionalDependencies`        |
| `-P, --save-prod`      | `-P`                        | `--save-prod` / `-P`     | `--save-prod` / `-P`          | Only remove from `dependencies`                |
| `--filter <pattern>`   | `--filter <pattern> remove` | `workspace <pkg> remove` | `uninstall --workspace <pkg>` | Target specific workspace package(s)           |
| `-w, --workspace-root` | `-w`                        | (default)                | (default)                     | Remove from workspace root                     |
| `-r, --recursive`      | `-r`                        | `--recursive` / `-r`     | `--recursive` / `-r`          | Remove recursively from all workspace packages |
| `-g, --global`         | `-g`                        | `global remove`          | `--global` / `-g`             | Remove global packages                         |

**Note**: Similar to add, `--filter` must precede the command for pnpm.

**Aliases:**

- `vite rm` = `vite remove`
- `vite un` = `vite remove`
- `vite uninstall` = `vite remove`

#### Workspace Filter Patterns

Based on pnpm's filter syntax:

| Pattern      | Description              | Example                                    |
| ------------ | ------------------------ | ------------------------------------------ |
| `<pkg-name>` | Exact package name       | `--filter app`                             |
| `<pattern>*` | Wildcard match           | `--filter "app*"` matches app, app-web     |
| `@<scope>/*` | Scope match              | `--filter "@myorg/*"`                      |
| `!<pattern>` | Exclude pattern          | `--filter "!test*"` excludes test packages |
| `<pkg>...`   | Package and dependencies | `--filter "app..."`                        |
| `...<pkg>`   | Package and dependents   | `--filter "...utils"`                      |

**Multiple Filters**:

```bash
vite add react --filter app --filter web  # Add to both app and web
vite add react --filter "app*" --filter "!app-test"  # Add to app* except app-test
```

#### Pass-Through Arguments

Additional parameters not covered by Vite+ can all be handled through pass-through arguments.

All arguments after `--` will be passed through to the package manager.

```bash
vite add react --allow-build=react,napi -- --use-stderr

-> pnpm add --allow-build=react,napi --use-stderr react
-> yarn add --use-stderr react
-> npm install --use-stderr react
```

### Implementation Architecture

#### 1. Command Structure

**File**: `crates/vite_task/src/lib.rs`

Add new command variants:

```rust
#[derive(Subcommand, Debug)]
pub enum Commands {
    // ... existing commands

    /// Add packages to dependencies
    #[command(disable_help_flag = true)]
    Add {
        /// Packages to add
        packages: Vec<String>,

        /// Filter packages in monorepo (can be used multiple times)
        #[arg(long, value_name = "PATTERN")]
        filter: Vec<String>,

        /// Add to workspace root (ignore-workspace-root-check)
        #[arg(short = 'w', long)]
        workspace_root: bool,

        /// Only add if package exists in workspace
        #[arg(long)]
        workspace: bool,

        /// Arguments to pass to package manager
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Remove packages from dependencies
    #[command(disable_help_flag = true, alias = "rm", alias = "un", alias = "uninstall")]
    Remove {
        /// Packages to remove
        packages: Vec<String>,

        /// Filter packages in monorepo (can be used multiple times)
        #[arg(long, value_name = "PATTERN")]
        filter: Vec<String>,

        /// Remove from workspace root
        #[arg(short = 'w', long)]
        workspace_root: bool,

        /// Arguments to pass to package manager
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
}
```

#### 2. Package Manager Adapter

**File**: `crates/vite_package_manager/src/package_manager.rs`

Add methods to translate commands:

```rust
impl PackageManager {
    /// Resolve add command for the package manager
    pub fn resolve_add_command(&self) -> &'static str {
        match self.client {
            PackageManagerType::Pnpm => "add",
            PackageManagerType::Yarn => "add",
            PackageManagerType::Npm => "install",
        }
    }

    /// Resolve remove command for the package manager
    pub fn resolve_remove_command(&self) -> &'static str {
        match self.client {
            PackageManagerType::Pnpm => "remove",
            PackageManagerType::Yarn => "remove",
            PackageManagerType::Npm => "uninstall",
        }
    }

    /// Build command arguments with workspace support
    pub fn build_add_args(
        &self,
        packages: &[String],
        filters: &[String],
        workspace_root: bool,
        workspace_only: bool,
        extra_args: &[String],
    ) -> Vec<String> {
        let mut args = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                // pnpm: --filter must come before command
                for filter in filters {
                    args.push("--filter".to_string());
                    args.push(filter.clone());
                }
                args.push("add".to_string());
                args.extend_from_slice(packages);
                if workspace_root {
                    args.push("-w".to_string());
                }
                if workspace_only {
                    args.push("--workspace".to_string());
                }
                args.extend_from_slice(extra_args);
            }
            PackageManagerType::Yarn => {
                // yarn: workspace <pkg> add
                if !filters.is_empty() {
                    // yarn workspace <name> add
                    for filter in filters {
                        args.push("workspace".to_string());
                        args.push(filter.clone());
                    }
                }
                args.push("add".to_string());
                args.extend_from_slice(packages);
                if workspace_root {
                    args.push("-W".to_string());
                }
                args.extend_from_slice(extra_args);
            }
            PackageManagerType::Npm => {
                // npm: --workspace must come before install
                if !filters.is_empty() {
                    for filter in filters {
                        args.push("--workspace".to_string());
                        args.push(filter.clone());
                    }
                }
                args.push("install".to_string());
                args.extend_from_slice(packages);
                if workspace_root {
                    args.push("-w".to_string());
                }
                args.extend_from_slice(extra_args);
            }
        }

        args
    }

    /// Build remove command arguments with workspace support
    pub fn build_remove_args(
        &self,
        packages: &[String],
        filters: &[String],
        workspace_root: bool,
        extra_args: &[String],
    ) -> Vec<String> {
        let mut args = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                for filter in filters {
                    args.push("--filter".to_string());
                    args.push(filter.clone());
                }
                args.push("remove".to_string());
                args.extend_from_slice(packages);
                if workspace_root {
                    args.push("-w".to_string());
                }
                args.extend_from_slice(extra_args);
            }
            PackageManagerType::Yarn => {
                if !filters.is_empty() {
                    for filter in filters {
                        args.push("workspace".to_string());
                        args.push(filter.clone());
                    }
                }
                args.push("remove".to_string());
                args.extend_from_slice(packages);
                args.extend_from_slice(extra_args);
            }
            PackageManagerType::Npm => {
                if !filters.is_empty() {
                    for filter in filters {
                        args.push("--workspace".to_string());
                        args.push(filter.clone());
                    }
                }
                args.push("uninstall".to_string());
                args.extend_from_slice(packages);
                args.extend_from_slice(extra_args);
            }
        }

        args
    }
}
```

#### 3. Add Command Implementation

**File**: `crates/vite_task/src/add.rs` (new file)

```rust
pub struct AddCommand {
    workspace_root: AbsolutePathBuf,
}

impl AddCommand {
    pub fn new(workspace_root: AbsolutePathBuf) -> Self {
        Self { workspace_root }
    }

    pub async fn execute(
        self,
        packages: Vec<String>,
        filters: Vec<String>,
        workspace_root: bool,
        workspace_only: bool,
        extra_args: Vec<String>,
    ) -> Result<ExecutionSummary, Error> {
        if packages.is_empty() {
            return Err(Error::NoPackagesSpecified);
        }

        let package_manager = PackageManager::builder(&self.workspace_root).build().await?;
        let workspace = Workspace::partial_load(self.workspace_root)?;

        let resolve_command = package_manager.resolve_command();

        // Build command with workspace support
        let full_args = package_manager.build_add_args(
            &packages,
            &filters,
            workspace_root,
            workspace_only,
            &extra_args,
        );

        let resolved_task = ResolvedTask::resolve_from_builtin_with_command_result(
            &workspace,
            "add",
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

#### 4. Remove Command Implementation

**File**: `crates/vite_task/src/remove.rs` (new file)

```rust
pub struct RemoveCommand {
    workspace_root: AbsolutePathBuf,
}

impl RemoveCommand {
    pub fn new(workspace_root: AbsolutePathBuf) -> Self {
        Self { workspace_root }
    }

    pub async fn execute(
        self,
        packages: Vec<String>,
        filters: Vec<String>,
        workspace_root: bool,
        extra_args: Vec<String>,
    ) -> Result<ExecutionSummary, Error> {
        if packages.is_empty() {
            return Err(Error::NoPackagesSpecified);
        }

        let package_manager = PackageManager::builder(&self.workspace_root).build().await?;
        let workspace = Workspace::partial_load(self.workspace_root)?;

        let resolve_command = package_manager.resolve_command();

        // Build command with workspace support
        let full_args = package_manager.build_remove_args(
            &packages,
            &filters,
            workspace_root,
            &extra_args,
        );

        let resolved_task = ResolvedTask::resolve_from_builtin_with_command_result(
            &workspace,
            "remove",
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

### Special Handling

#### 1. Global Packages

Yarn requires different command structure for global operations:

```rust
// pnpm/npm: <bin> add -g <package>
// yarn: <bin> global add <package>

fn handle_global_flag(args: &[String], pm_type: PackageManagerType) -> (Vec<String>, bool) {
    let has_global = args.contains(&"-g".to_string()) || args.contains(&"--global".to_string());
    let filtered_args: Vec<String> = args.iter()
        .filter(|a| *a != "-g" && *a != "--global")
        .cloned()
        .collect();

    (filtered_args, has_global)
}
```

#### 2. Workspace Filters

pnpm uses `--filter` before command, yarn/npm use different approaches:

```rust
fn build_workspace_command(
    pm_type: PackageManagerType,
    filters: &[String],
    operation: &str,
    packages: &[String],
) -> Vec<String> {
    match pm_type {
        PackageManagerType::Pnpm => {
            // pnpm --filter <pkg> add <deps>
            let mut args = Vec::new();
            for filter in filters {
                args.push("--filter".to_string());
                args.push(filter.clone());
            }
            args.push(operation.to_string());
            args.extend_from_slice(packages);
            args
        }
        PackageManagerType::Yarn => {
            // yarn workspace <pkg> add <deps>
            let mut args = Vec::new();
            if let Some(filter) = filters.first() {
                args.push("workspace".to_string());
                args.push(filter.clone());
            }
            args.push(operation.to_string());
            args.extend_from_slice(packages);
            args
        }
        PackageManagerType::Npm => {
            // npm install <deps> --workspace <pkg>
            let mut args = vec![operation.to_string()];
            args.extend_from_slice(packages);
            for filter in filters {
                args.push("--workspace".to_string());
                args.push(filter.clone());
            }
            args
        }
    }
}
```

#### 3. Workspace Dependencies

When adding workspace dependencies with `--workspace` flag:

```bash
# pnpm: Adds with workspace: protocol
vite add @myorg/utils --workspace --filter app
# → pnpm --filter app add @myorg/utils --workspace
# → Adds: "@myorg/utils": "workspace:*"

# Without --workspace: Tries to install from registry
vite add @myorg/utils --filter app
# → pnpm --filter app add @myorg/utils
# → Tries npm registry (may fail if not published)
```

## Design Decisions

### 1. No Caching

**Decision**: Do not cache add/remove operations.

**Rationale**:

- These commands modify package.json and lockfiles
- Side effects make caching inappropriate
- Each execution should run fresh
- Similar to how `vite install` works

**Implementation**: Set `cacheable: false` or skip cache entirely.

### 2. Pass-Through Arguments

**Decision**: Pass all arguments after packages directly to package manager.

**Rationale**:

- Package managers have many flags (40+ for npm)
- Maintaining complete flag mapping is error-prone
- Pass-through allows accessing all features
- Only translate critical command name differences

**Example**:

```bash
vite add react --save-exact
# → pnpm add react --save-exact
# → yarn add react --save-exact
# → npm install react --save-exact
```

### 3. Common Flags Only

**Decision**: Only explicitly support most common flags with automatic translation.

**Common Flags**:

- `-D, --save-dev` - universally supported
- `-g, --global` - needs special handling for yarn
- `-E, --save-exact` - universally supported
- `-P, --save-peer` - universally supported
- `-O, --save-optional` - universally supported

**Advanced Flags**: Pass through as-is

### 4. Command Aliases

**Decision**: Support multiple aliases for remove command.

**Aliases**:

- `vite remove` (primary)
- `vite rm` (short)
- `vite un` (short, matches pnpm)
- `vite uninstall` (explicit, matches npm)

**Rationale**: Matches user expectations from other tools.

### 5. Multiple Package Support

**Decision**: Allow specifying multiple packages in single command.

**Example**:

```bash
vite add react react-dom @types/react -D
vite remove lodash axios underscore
```

**Implementation**: Packages are positional arguments before flags.

## Error Handling

### No Packages Specified

```bash
$ vite add
Error: No packages specified
Usage: vite add <PACKAGES>... [OPTIONS]
```

### Package Manager Not Detected

```bash
$ vite add react
Error: No package manager detected
Please run one of:
  - vite install (to set up package manager)
  - Add packageManager field to package.json
```

### Invalid Package Names

Let the underlying package manager handle validation and provide clear errors.

## User Experience

### Success Output

```bash
$ vite add react react-dom
Detected package manager: pnpm@10.15.0
Running: pnpm add react react-dom

 WARN  deprecated inflight@1.0.6: ...

Packages: +2
++
Progress: resolved 150, reused 140, downloaded 10, added 2, done

dependencies:
+ react 18.3.1
+ react-dom 18.3.1

Done in 2.3s
```

### Error Output

```bash
$ vite add invalid-package-that-does-not-exist
Detected package manager: pnpm@10.15.0
Running: pnpm add invalid-package-that-does-not-exist

 ERR_PNPM_FETCH_404  GET https://registry.npmjs.org/invalid-package-that-does-not-exist: Not Found - 404

This error happened while installing the dependencies of undefined@undefined

Error: Command failed with exit code 1
```

## Alternative Designs Considered

### Alternative 1: Flag Translation Layer

Translate all flags to package manager-specific equivalents:

```bash
vite add react --dev
# → pnpm add react -D
# → yarn add react --dev
# → npm install react --save-dev
```

**Rejected because**:

- Maintenance burden (40+ npm flags)
- Package managers evolve with new flags
- Pass-through is simpler and more flexible
- Users can use native flags directly

### Alternative 2: Separate Commands per Package Manager

```bash
vite pnpm:add react
vite yarn:add react
vite npm:install react
```

**Rejected because**:

- Defeats purpose of unified interface
- More verbose
- Doesn't leverage auto-detection

### Alternative 3: Interactive Mode

Prompt for packages and options interactively:

```bash
$ vite add
? Which packages to add? react
? Add as dev dependency? Yes
```

**Rejected for initial version**:

- Slower for experienced users
- Not scriptable
- Can be added later as optional mode

## Implementation Plan

### Phase 1: Core Functionality

1. Add `Add` and `Remove` command variants to `Commands` enum
2. Create `add.rs` and `remove.rs` modules
3. Implement package manager command resolution
4. Add basic error handling

### Phase 2: Special Cases

1. Handle yarn global commands differently
2. Validate package names (optional)
3. Support workspace-specific operations

### Phase 3: Testing

1. Unit tests for command resolution
2. Integration tests with mock package managers
3. Manual testing with real package managers

### Phase 4: Documentation

1. Update CLI documentation
2. Add examples to README
3. Document flag compatibility matrix

## Testing Strategy

### Test Package Manager Versions

- pnpm@9.x [WIP]
- pnpm@10.x
- yarn@1.x [WIP]
- yarn@4.x
- npm@10.x
- npm@11.x [WIP]

### Unit Tests

```rust
#[test]
fn test_add_command_resolution() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    assert_eq!(pm.resolve_add_command(), "add");

    let pm = PackageManager::mock(PackageManagerType::Npm);
    assert_eq!(pm.resolve_add_command(), "install");
}

#[test]
fn test_remove_command_resolution() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    assert_eq!(pm.resolve_remove_command(), "remove");

    let pm = PackageManager::mock(PackageManagerType::Npm);
    assert_eq!(pm.resolve_remove_command(), "uninstall");
}

#[test]
fn test_build_add_args_pnpm() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.build_add_args(
        &["react".to_string()],
        &["app".to_string()],
        false,
        false,
        &[],
    );
    assert_eq!(args, vec!["--filter", "app", "add", "react"]);
}

#[test]
fn test_build_add_args_with_workspace_root() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.build_add_args(
        &["typescript".to_string()],
        &[],
        true,  // workspace_root
        false,
        &["-D".to_string()],
    );
    assert_eq!(args, vec!["add", "typescript", "-w", "-D"]);
}

#[test]
fn test_build_add_args_yarn_workspace() {
    let pm = PackageManager::mock(PackageManagerType::Yarn);
    let args = pm.build_add_args(
        &["react".to_string()],
        &["app".to_string()],
        false,
        false,
        &[],
    );
    assert_eq!(args, vec!["workspace", "app", "add", "react"]);
}

#[test]
fn test_build_remove_args_with_filter() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.build_remove_args(
        &["lodash".to_string()],
        &["utils".to_string()],
        false,
        &[],
    );
    assert_eq!(args, vec!["--filter", "utils", "remove", "lodash"]);
}
```

### Integration Tests

Create fixtures for testing with each package manager:

```
fixtures/add-remove-test/
  pnpm-workspace.yaml
  package.json
  packages/
    app/
      package.json
    utils/
      package.json
  test-steps.json
```

Test cases:

1. Add single package
2. Add multiple packages
3. Add with -D flag
4. Add with --filter to specific package
5. Add with --filter wildcard pattern
6. Add to workspace root with -w
7. Add workspace dependency with --workspace
8. Remove single package
9. Remove multiple packages
10. Remove with --filter
11. Error handling for invalid packages
12. Error handling for incompatible filters on yarn/npm

## CLI Help Output

### Add Command

```bash
$ vite add --help
Add packages to dependencies

Usage: vite add <PACKAGES>... [OPTIONS]

Arguments:
  <PACKAGES>...  Packages to add

Options:
  --filter <PATTERN>   Filter packages in monorepo (can be used multiple times)
  -w, --workspace-root Add to workspace root (ignore-workspace-root-check)
  --workspace          Only add if package exists in workspace
  -D, --save-dev       Add as dev dependency
  -P, --save-peer      Add as peer dependency
  -O, --save-optional  Add as optional dependency
  -E, --save-exact     Save exact version
  -g, --global         Install globally
  -h, --help           Print help

Filter Patterns:
  <name>           Exact package name match
  <pattern>*       Wildcard match (pnpm only)
  @<scope>/*       Scope match (pnpm only)
  !<pattern>       Exclude pattern (pnpm only)
  <pkg>...         Package and dependencies (pnpm only)
  ...<pkg>         Package and dependents (pnpm only)

Examples:
  vite add react react-dom
  vite add -D typescript @types/react
  vite add react --filter app
  vite add react --filter "app*" --filter "!app-test"
  vite add @myorg/utils --workspace --filter web
  vite add lodash -w
```

### Remove Command

```bash
$ vite remove --help
Remove packages from dependencies

Usage: vite remove <PACKAGES>... [OPTIONS]

Aliases: rm, un, uninstall

Arguments:
  <PACKAGES>...  Packages to remove

Options:
  --filter <PATTERN>   Filter packages in monorepo (can be used multiple times)
  -w, --workspace-root Remove from workspace root
  -g, --global         Remove global packages
  -h, --help           Print help

Filter Patterns:
  <name>           Exact package name match
  <pattern>*       Wildcard match (pnpm only)
  @<scope>/*       Scope match (pnpm only)
  !<pattern>       Exclude pattern (pnpm only)

Examples:
  vite remove lodash
  vite remove axios underscore lodash
  vite rm lodash --filter app
  vite remove utils --filter "app*"
  vite rm old-package
```

## Performance Considerations

1. **No Caching**: Operations run directly without cache overhead
2. **Single Execution**: Unlike task runner, these are one-off operations
3. **Pass-Through**: Minimal processing, just command translation
4. **Auto-Detection**: Reuses existing package manager detection (already cached)

## Security Considerations

1. **Package Name Validation**: Let package manager handle validation
2. **Lockfile Integrity**: Package manager ensures integrity
3. **No Code Execution**: Just passes through to trusted package manager
4. **Audit Flags**: Users can add `--audit` via pass-through

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
pnpm add react

# New way (works with any package manager)
vite add react
```

### Discoverability

Add to:

- CLI help output
- Documentation
- VSCode extension suggestions
- Shell completions

## Documentation Requirements

### User Guide

Add to CLI documentation:

````markdown
### Adding Packages

```bash
vite add <packages>... [OPTIONS]
```
````

Automatically uses the detected package manager (pnpm/yarn/npm).

**Basic Examples:**

- `vite add react` - Add production dependency
- `vite add -D typescript` - Add dev dependency
- `vite add react react-dom` - Add multiple packages

**Workspace Examples:**

- `vite add react --filter app` - Add to specific package
- `vite add react --filter "app*"` - Add to multiple packages (pnpm)
- `vite add @myorg/utils --workspace --filter web` - Add workspace dependency
- `vite add lodash -w` - Add to workspace root

**Common Options:**

- `--filter <pattern>` - Target specific workspace package(s)
- `-w, --workspace-root` - Add to workspace root
- `--workspace` - Add workspace dependency (pnpm)
- `-D, --save-dev` - Add as dev dependency
- `-E, --save-exact` - Save exact version
- `-P, --save-peer` - Add as peer dependency
- `-O, --save-optional` - Add as optional dependency
- `-g, --global` - Install globally

### Removing Packages

```bash
vite remove <packages>... [OPTIONS]
vite rm <packages>... [OPTIONS]
```

Aliases: `rm`, `un`, `uninstall`

**Basic Examples:**

- `vite remove lodash` - Remove package
- `vite rm axios underscore` - Remove multiple packages

**Workspace Examples:**

- `vite remove lodash --filter app` - Remove from specific package
- `vite rm utils --filter "app*"` - Remove from multiple packages (pnpm)
- `vite remove -g typescript` - Remove global package

**Options:**

- `--filter <pattern>` - Target specific workspace package(s)
- `-w, --workspace-root` - Remove from workspace root
- `-g, --global` - Remove global packages

````
### Package Manager Compatibility

Document flag support matrix:

| Flag | pnpm | yarn | npm |
|------|------|------|-----|
| `-D` | ✅ | ✅ | ✅ |
| `-E` | ✅ | ✅ | ✅ |
| `-P` | ✅ | ✅ | ✅ |
| `-O` | ✅ | ✅ | ✅ |
| `-g` | ✅ | ⚠️ (use global) | ✅ |

## Workspace Operations Deep Dive

### Filter Patterns (pnpm-inspired)

Following pnpm's filter API:

**Exact Match:**
```bash
vite add react --filter app
# → pnpm --filter app add react
````

**Wildcard Patterns:**

```bash
vite add react --filter "app*"
# → pnpm --filter "app*" add react
# Matches: app, app-web, app-mobile
```

**Scope Patterns:**

```bash
vite add lodash --filter "@myorg/*"
# → pnpm --filter "@myorg/*" add lodash
# Matches all packages in @myorg scope
```

**Exclusion Patterns:**

```bash
vite add react --filter "!test*"
# → pnpm --filter "!test*" add react
# Adds to all packages EXCEPT those starting with test
```

**Multiple Filters:**

```bash
vite add react --filter app --filter web
# → pnpm --filter app --filter web add react
# Adds to both app AND web packages
```

**Dependency Selectors:**

```bash
# Add to package and all its dependencies
vite add lodash --filter "app..."
# → pnpm --filter "app..." add lodash

# Add to package and all its dependents
vite add utils --filter "...core"
# → pnpm --filter "...core" add utils
```

### Workspace Root Operations

Add dependencies to workspace root (requires special flag):

```bash
vite add -D typescript -w
# → pnpm add -D typescript -w  (pnpm)
# → yarn add -D typescript -W  (yarn)
# → npm install -D typescript -w  (npm)
```

**Why needed**: By default, package managers prevent adding to workspace root to encourage proper package structure.

### Workspace Protocol

For internal monorepo dependencies:

```bash
# Add workspace dependency with workspace: protocol
vite add @myorg/utils --workspace --filter app
# → pnpm --filter app add @myorg/utils --workspace
# → Adds: "@myorg/utils": "workspace:*"

# Specify version
vite add "@myorg/utils@workspace:^" --filter app
# → Adds: "@myorg/utils": "workspace:^"
```

### Package Manager Compatibility

| Feature                    | pnpm               | yarn                 | npm                    | Notes                    |
| -------------------------- | ------------------ | -------------------- | ---------------------- | ------------------------ |
| `--filter <pattern>`       | ✅ Native          | ⚠️ `workspace <name>` | ⚠️ `--workspace <name>` | Syntax differs           |
| Multiple filters           | ✅ Repeatable flag | ❌ Single only       | ⚠️ Limited              | pnpm most flexible       |
| Wildcard patterns          | ✅ Full support    | ⚠️ Limited            | ❌ No wildcards        | pnpm best                |
| Exclusion `!`              | ✅ Supported       | ❌ Not supported     | ❌ Not supported       | pnpm only                |
| Dependency selectors `...` | ✅ Supported       | ❌ Not supported     | ❌ Not supported       | pnpm only                |
| `-w` (root)                | ✅ `-w`            | ✅ `-W`              | ✅ `-w`                | Slightly different flags |
| `--workspace` protocol     | ✅ Supported       | ❌ Manual            | ❌ Manual              | pnpm feature             |

**Graceful Degradation**:

- Advanced pnpm features (wildcard, exclusion, selectors) will error on yarn/npm with helpful message
- Basic `--filter <exact-name>` works across all package managers

## Future Enhancements

### 1. Enhanced Filter Support for yarn/npm

Implement wildcard translation for yarn/npm:

```bash
vite add react --filter "app*"
# → For yarn: Run `yarn workspace app add react` for each matching package
# → For npm: Run `npm install react --workspace app` for each matching package
```

### 2. Interactive Mode

> Referer to ni's interactive mode https://github.com/antfu-collective/ni

```bash
$ vite add --interactive
? Select for package > tsdown
❯   tsdown                         v0.15.7 - git+https://github.com/rolldown/tsdown.git
    tsdown-config-silverwind       v1.4.0 - git+https://github.com/silverwind/tsdown-config-silverwind.git
    @storm-software/tsdown         v0.45.0 - git+https://github.com/storm-software/storm-ops.git
    create-tsdown                  v0.15.7 - git+https://github.com/rolldown/tsdown.git
    shadcn-auv                     v0.0.1 - git+https://github.com/ohojs/shadcn-auv.git
    ts-build-wizard                v1.0.3 - git+https://github.com/Alireza-Tabatabaeian/react-app-registry.git
    vite-plugin-shadcn-registry    v0.0.6 - git+https://github.com/myshkouski/vite-plugin-shadcn-registry.git
    @qds.dev/tools                 v0.3.3 - https://www.npmjs.com/package/@qds.dev/tools
    feishu-bot-notify              v0.1.3 - git+https://github.com/duowb/feishu-bot-notify.git
    @memo28.pro/bundler            v0.0.2 - https://www.npmjs.com/package/@memo28.pro/bundler
    tsdown-jsr-exports-lint        v0.1.4 - git+https://github.com/kazupon/tsdown-jsr-exports-lint.git
    @miloas/tsdown                 v0.13.0 - git+https://github.com/rolldown/tsdown.git
    @socket-synced-state/server    v0.0.9 - https://www.npmjs.com/package/@socket-synced-state/server
    @gamedev-sensei/tsdown-config  v2.0.1 - git+ssh://git@github.com/gamedev-sensei/package-extras.git
  ↓ 0xpresc-test                   v0.1.0 - https://www.npmjs.com/package/0xpresc-test

? install tsdown as › - Use arrow-keys. Return to submit.
❯   prod
    dev
    peer
```

### 3. Upgrade Command

```bash
vite upgrade react
vite upgrade --latest
vite upgrade --interactive
```

### 4. Smart Suggestions

```bash
$ vite add react
Adding react...
💡 Suggestion: Install @types/react for TypeScript support?
   Run: vite add -D @types/react
```

### 5. Dependency Analysis

```bash
$ vite add react
Analyzing dependency impact...
  Will add:
    react@18.3.1 (85KB)
    + scheduler@0.23.0 (5KB)
  Total size: 90KB

Proceed? (Y/n)
```

## Open Questions

1. **Should we warn about peer dependency conflicts?**
   - Proposed: Let package manager handle warnings
   - Can be enhanced later with custom warnings

2. **Should we support version specifiers?**
   - Proposed: Yes, pass through to package manager
   - Example: `vite add react@18.2.0`

3. **Should we support scoped package shortcuts?**
   - Proposed: No special handling, pass through as-is
   - Example: `vite add @types/react` works naturally

4. **Should we prevent adding to wrong dependency types?**
   - Proposed: No validation, trust package manager
   - Package managers handle this well already

5. **How to handle pnpm-specific filter features on yarn/npm?**
   - Proposed: For wildcards/exclusions on yarn/npm:
     - Option A: Error with clear message explaining pnpm-only feature
     - Option B: Resolve wildcard ourselves and run command for each package
   - Recommendation: Start with Option A, add Option B later

6. **Should we support workspace protocol configuration?**
   - Proposed: Pass through to pnpm, document in .npmrc for users
   - Example: `save-workspace-protocol=rolling` in .npmrc
   - vite+ doesn't need to handle this explicitly

7. **Should we validate that filtered packages exist?**
   - Proposed: Let package manager validate
   - Clearer error messages from native tools
   - Avoids duplicating workspace parsing logic

## Success Metrics

1. **Adoption**: % of users using `vite add/remove` vs direct package manager
2. **Error Rate**: Track command failures vs package manager direct usage
3. **User Feedback**: Survey/issues about command ergonomics
4. **Performance**: Measure overhead vs direct package manager calls (<100ms target)

## Implementation Timeline

- **Week 1**: Core implementation (command parsing, package manager adapter)
- **Week 2**: Testing (unit tests, integration tests)
- **Week 3**: Documentation and examples
- **Week 4**: Review, polish, and release

## Dependencies

### New Dependencies

None required - leverages existing:

- `vite_package_manager` - package manager detection
- `clap` - command parsing
- Existing task execution infrastructure

### Modified Files

- `crates/vite_task/src/lib.rs` - Add command enum variants
- `crates/vite_task/src/add.rs` - New file
- `crates/vite_task/src/remove.rs` - New file
- `crates/vite_package_manager/src/package_manager.rs` - Add command resolution methods
- `docs/cli.md` - Documentation updates

## Workspace Feature Implementation Priority

### Phase 1: Core Functionality (MVP)

- ✅ Basic add/remove without filters
- ✅ Multiple package support
- ✅ Auto package manager detection
- ✅ Common flags (-D, -E, -P, -O)

### Phase 2: Workspace Support (pnpm-focused)

- ✅ `--filter <exact-name>` for all package managers
- ✅ `-w` flag for workspace root
- ✅ `--workspace` flag for workspace dependencies (pnpm)
- ✅ Wildcard patterns `*` (pnpm only, error on others)
- ✅ Scope patterns `@scope/*` (pnpm only)

### Phase 3: Advanced Filters (pnpm-focused)

- Exclusion patterns `!<pattern>` (pnpm only)
- Dependency selectors `...` (pnpm only)
- Multiple filter support
- Graceful degradation for yarn/npm

### Phase 4: Cross-PM Compatibility (optional)

- Wildcard resolution for yarn/npm
- Run filtered command for each matching package
- Unified behavior across all package managers

## Real-World Usage Examples

### Monorepo Package Management

```bash
# Add React to all frontend packages
vite add react react-dom --filter "@myorg/app-*"

# Add testing library to all packages
vite add -D vitest --filter "*"

# Add shared utils to app packages (workspace dependency)
vite add @myorg/shared-utils --workspace --filter "@myorg/app-*"

# Remove deprecated package from all packages
vite remove moment --filter "*"

# Add TypeScript to workspace root (shared config)
vite add -D typescript @types/node -w
```

### Development Workflow

```bash
# Clone new monorepo
git clone <repo>
vite install

# Add new feature dependencies to web app
cd packages/web
vite add axios react-query

# Add development tool to specific package
vite add -D webpack-bundle-analyzer --filter web

# Remove unused dependencies from utils package
vite rm lodash underscore --filter utils

# Add workspace package as dependency
vite add @myorg/ui-components --workspace --filter web
```

### Migration from Direct Package Manager

```bash
# Before (package manager specific)
pnpm --filter app add react
yarn workspace app add react
npm install react --workspace app

# After (unified)
vite add react --filter app
```

## Conclusion

This RFC proposes adding `vite add` and `vite remove` commands to provide a unified interface for package management across pnpm/yarn/npm. The design:

- ✅ Automatically adapts to detected package manager
- ✅ Supports multiple packages in single command
- ✅ **Full workspace support following pnpm's API design**
- ✅ **Filter patterns for targeting specific packages**
- ✅ **Workspace root and workspace protocol support**
- ✅ Uses pass-through for maximum flexibility
- ✅ No caching overhead (as requested)
- ✅ Simple implementation leveraging existing infrastructure
- ✅ Graceful degradation for package manager-specific features
- ✅ Extensible for future enhancements

The implementation follows pnpm's battle-tested workspace API design while providing graceful degradation for yarn/npm users. This provides immediate value to monorepo developers with a unified, intuitive interface.
