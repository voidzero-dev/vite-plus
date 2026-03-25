# RFC: Vite+ Update Package Command

## Summary

Add `vp update` (alias: `vp up`) command that automatically adapts to the detected package manager (pnpm/yarn/npm/bun) for updating packages to their latest versions within the specified semver range, with support for updating to absolute latest versions, workspace-aware operations, and interactive mode.

## Motivation

Currently, developers must manually use package manager-specific commands to update dependencies:

```bash
pnpm update react
yarn upgrade react
npm update react
```

This creates friction in monorepo workflows and requires remembering different syntaxes. A unified interface would:

1. **Simplify workflows**: One command works across all package managers
2. **Auto-detection**: Automatically uses the correct package manager
3. **Consistency**: Same syntax regardless of underlying tool
4. **Integration**: Works seamlessly with existing Vite+ features

### Current Pain Points

```bash
# Developer needs to know which package manager is used
pnpm update react --latest          # pnpm project
yarn upgrade react --latest         # yarn project
npm update react                    # npm project (no --latest flag)

# Different commands for updating all packages
pnpm update                         # pnpm
yarn upgrade                        # yarn@1 / yarn upgrade-interactive for yarn@2+
npm update                          # npm
```

### Proposed Solution

```bash
# Works for all package managers
vp update react             # Update to latest within semver range
vp up react --latest        # Update to absolute latest version
vp update                   # Update all packages

# Workspace operations
vp update --filter app                    # Update in specific package
vp update react --latest --filter "app*"  # Update to latest in multiple packages
vp update -r                              # Update recursively in all workspaces
```

## Proposed Solution

### Command Syntax

#### Update Command

```bash
vp update [PACKAGES]... [OPTIONS]
vp up [PACKAGES]... [OPTIONS]        # Alias
```

**Examples:**

```bash
# Update to latest version within semver range
vp update react react-dom

# Update to absolute latest version
vp update react --latest
vp up react -L

# Update all dependencies
vp update

# Update all to latest
vp update --latest

# Update only dev dependencies
vp update -D

# Update only production dependencies
vp update -P

# Workspace operations
vp update --filter app                    # Update in specific package
vp update react --latest --filter "app*"  # Update in multiple packages
vp update -r                              # Update in all workspace packages
vp update -g typescript                   # Update global package

# Interactive mode (pnpm only)
vp update --interactive
vp up -i

# Advanced options
vp update --no-optional                   # Skip optional dependencies
vp update --no-save                       # Update lockfile only
vp update react --latest --no-save        # Test latest version without saving
```

### Command Mapping

#### Update Command Mapping

- https://pnpm.io/cli/update
- https://yarnpkg.com/cli/up (yarn@2+)
- https://classic.yarnpkg.com/en/docs/cli/upgrade (yarn@1)
- https://docs.npmjs.com/cli/v11/commands/npm-update
- https://bun.sh/docs/cli/update

| Vite+ Flag             | pnpm                        | yarn@1               | yarn@2+                                     | npm                            | bun                    | Description                                                |
| ---------------------- | --------------------------- | -------------------- | ------------------------------------------- | ------------------------------ | ---------------------- | ---------------------------------------------------------- |
| `[packages]`           | `update [packages]`         | `upgrade [packages]` | `up [packages]`                             | `update [packages]`            | `update [packages]`    | Update specific packages (or all if omitted)               |
| `-L, --latest`         | `--latest` / `-L`           | `--latest`           | N/A (default behavior)                      | N/A                            | `--latest`             | Update to latest version (ignore semver range)             |
| `-g, --global`         | N/A                         | N/A                  | N/A                                         | `--global` / `-g`              | N/A                    | Update global packages                                     |
| `-r, --recursive`      | `-r, --recursive`           | N/A                  | `--recursive` / `-R`                        | `--workspaces`                 | `--recursive` / `-r`   | Update recursively in all workspace packages               |
| `--filter <pattern>`   | `--filter <pattern> update` | N/A                  | `workspaces foreach --include <pattern> up` | `update --workspace <pattern>` | N/A                    | Target specific workspace package(s)                       |
| `-w, --workspace-root` | `-w`                        | N/A                  | N/A                                         | `--include-workspace-root`     | N/A                    | Include workspace root                                     |
| `-D, --dev`            | `--dev` / `-D`              | N/A                  | N/A                                         | `--include=dev`                | N/A                    | Update only devDependencies                                |
| `-P, --prod`           | `--prod` / `-P`             | N/A                  | N/A                                         | `--include=prod`               | `--production`         | Update only dependencies and optionalDependencies          |
| `-i, --interactive`    | `--interactive` / `-i`      | N/A                  | `--interactive` / `-i`                      | N/A                            | `--interactive` / `-i` | Show outdated packages and choose which to update          |
| `--no-optional`        | `--no-optional`             | N/A                  | N/A                                         | `--no-optional`                | `--omit optional`      | Don't update optionalDependencies                          |
| `--no-save`            | `--no-save`                 | N/A                  | N/A                                         | `--no-save`                    | `--no-save`            | Update lockfile only, don't modify package.json            |
| `--workspace`          | `--workspace`               | N/A                  | N/A                                         | N/A                            | N/A                    | Only update if package exists in workspace (pnpm-specific) |

**Note**:

- For pnpm, `--filter` must come before the command (e.g., `pnpm --filter app update react`)
- Yarn@2+ uses `up` or `upgrade` command, and updates to latest by default
- Yarn@1 uses `upgrade` command
- npm doesn't support `--latest` flag, it always updates within semver range
- `--no-optional` skips updating optional dependencies (pnpm/npm/bun)
- `--no-save` updates lockfile without modifying package.json (pnpm/npm/bun)
- bun supports `--recursive`, `--latest`, `--interactive`, `--production`, `--omit optional`, and `--no-save` flags

**Aliases:**

- `vp up` = `vp update`

### Command Translation Strategy

#### Global Package Updates

For global packages, use npm cli only (same as add/remove):

```bash
vp update -g typescript
-> npm update --global typescript
```

#### Latest Version Updates

Different package managers handle "latest" differently:

**pnpm**: Has explicit `--latest` flag

```bash
vp update react --latest
-> pnpm update --latest react
```

**yarn@1**: Has `--latest` flag

```bash
vp update react --latest
-> yarn upgrade --latest react
```

**yarn@2+**: Updates to latest by default, use `^` or `~` for range updates

```bash
vp update react --latest
-> yarn up react                    # Already updates to latest
```

**npm**: No `--latest` flag, only updates within semver range

```bash
vp update react --latest
-> npx npm-check-updates -u react && npm install
# OR warn user and update within range
-> npm update react
```

### Implementation Architecture

#### 1. Command Structure

**File**: `crates/vite_task/src/lib.rs`

Add new command variant:

```rust
#[derive(Subcommand, Debug)]
pub enum Commands {
    // ... existing commands

    /// Update packages to their latest versions
    #[command(alias = "up")]
    Update {
        /// Update to latest version (ignore semver range)
        #[arg(short = 'L', long)]
        latest: bool,

        /// Update global packages
        #[arg(short = 'g', long)]
        global: bool,

        /// Update recursively in all workspace packages
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Filter packages in monorepo (can be used multiple times)
        #[arg(long, value_name = "PATTERN")]
        filter: Option<Vec<String>>,

        /// Include workspace root
        #[arg(short = 'w', long)]
        workspace_root: bool,

        /// Update only devDependencies
        #[arg(short = 'D', long)]
        dev: bool,

        /// Update only dependencies (production)
        #[arg(short = 'P', long)]
        prod: bool,

        /// Interactive mode - show outdated packages and choose
        #[arg(short = 'i', long)]
        interactive: bool,

        /// Don't update optionalDependencies
        #[arg(long)]
        no_optional: bool,

        /// Update lockfile only, don't modify package.json
        #[arg(long)]
        no_save: bool,

        /// Packages to update (optional - updates all if omitted)
        packages: Vec<String>,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },
}
```

#### 2. Package Manager Adapter

**File**: `crates/vite_package_manager/src/update.rs` (new file)

```rust
#[derive(Debug, Default)]
pub struct UpdateCommandOptions<'a> {
    pub packages: &'a [String],
    pub latest: bool,
    pub global: bool,
    pub recursive: bool,
    pub filters: Option<&'a [String]>,
    pub workspace_root: bool,
    pub dev: bool,
    pub prod: bool,
    pub interactive: bool,
    pub no_optional: bool,
    pub no_save: bool,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    pub fn resolve_update_command(&self, options: &UpdateCommandOptions) -> ResolveCommandResult {
        let bin_name: String;
        let mut args: Vec<String> = Vec::new();

        // Global packages use npm only
        if options.global {
            bin_name = "npm".into();
            args.push("update".into());
            args.push("--global".into());
            args.extend_from_slice(options.packages);
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
                args.push("update".into());

                if options.latest {
                    args.push("--latest".into());
                }
                if options.workspace_root {
                    args.push("--workspace-root".into());
                }
                if options.recursive {
                    args.push("--recursive".into());
                }
                if options.dev {
                    args.push("--dev".into());
                }
                if options.prod {
                    args.push("--prod".into());
                }
                if options.interactive {
                    args.push("--interactive".into());
                }
                if options.no_optional {
                    args.push("--no-optional".into());
                }
                if options.no_save {
                    args.push("--no-save".into());
                }
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();

                // Determine yarn version
                let is_yarn_v1 = self.version.starts_with("1.");

                if is_yarn_v1 {
                    // yarn@1: yarn upgrade [--latest]
                    if let Some(filters) = options.filters {
                        // yarn@1 doesn't support workspace filtering well
                        // Use basic workspace command
                        args.push("workspace".into());
                        args.push(filters[0].clone());
                    }
                    args.push("upgrade".into());
                    if options.latest {
                        args.push("--latest".into());
                    }
                } else {
                    // yarn@2+: yarn up (already updates to latest by default)
                    if let Some(filters) = options.filters {
                        args.push("workspaces".into());
                        args.push("foreach".into());
                        args.push("--all".into());
                        for filter in filters {
                            args.push("--include".into());
                            args.push(filter.clone());
                        }
                    }
                    args.push("up".into());
                    if options.recursive {
                        args.push("--recursive".into());
                    }
                    if options.interactive {
                        args.push("--interactive".into());
                    }
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();
                args.push("update".into());

                if let Some(filters) = options.filters {
                    for filter in filters {
                        args.push("--workspace".into());
                        args.push(filter.clone());
                    }
                }
                if options.workspace_root {
                    args.push("--include-workspace-root".into());
                }
                if options.recursive {
                    args.push("--workspaces".into());
                }
                if options.no_optional {
                    args.push("--no-optional".into());
                }
                if options.no_save {
                    args.push("--no-save".into());
                }

                // npm doesn't have --latest flag
                // Warn user or handle differently
                if options.latest {
                    eprintln!("Warning: npm doesn't support --latest flag. Use 'npm outdated' to check for updates.");
                }
            }
        }

        args.extend_from_slice(options.packages);
        if let Some(pass_through_args) = options.pass_through_args {
            args.extend_from_slice(pass_through_args);
        }

        ResolveCommandResult { bin_path: bin_name, args, envs }
    }
}
```

#### 3. Update Command Implementation

**File**: `crates/vite_task/src/update.rs` (new file)

```rust
pub struct UpdateCommand {
    workspace_root: AbsolutePathBuf,
}

impl UpdateCommand {
    pub fn new(workspace_root: AbsolutePathBuf) -> Self {
        Self { workspace_root }
    }

    pub async fn execute(
        self,
        packages: &[String],
        latest: bool,
        global: bool,
        recursive: bool,
        filters: Option<&[String]>,
        workspace_root: bool,
        dev: bool,
        prod: bool,
        interactive: bool,
        no_optional: bool,
        no_save: bool,
        pass_through_args: Option<&[String]>,
    ) -> Result<ExecutionSummary, Error> {
        // Detect package manager
        let package_manager = PackageManager::builder(&self.workspace_root).build().await?;
        let workspace = Workspace::partial_load(self.workspace_root)?;

        let update_command_options = UpdateCommandOptions {
            packages,
            latest,
            global,
            recursive,
            filters,
            workspace_root,
            dev,
            prod,
            interactive,
            no_optional,
            no_save,
            pass_through_args,
        };
        let resolve_command = package_manager.resolve_update_command(&update_command_options);

        println!("Running: {} {}", resolve_command.bin_path, resolve_command.args.join(" "));

        let resolved_task = ResolvedTask::resolve_from_builtin_with_command_result(
            &workspace,
            "update",
            resolve_command.args.iter(),
            ResolveCommandResult { bin_path: resolve_command.bin_path, envs: resolve_command.envs },
            false,
            None,
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

**Decision**: Do not cache update operations.

**Rationale**:

- Update commands modify package.json and lockfiles
- Side effects make caching inappropriate
- Each execution should run fresh
- Similar to how add/remove/install work

### 2. Default Behavior: Update All vs Specific

**Decision**: When no packages are specified, update all dependencies.

**Rationale**:

- Matches behavior of all three package managers
- Common use case: `vp update` to update everything
- Specific updates: `vp update react`

### 3. Latest Flag Handling for npm

**Decision**: Warn users that npm doesn't support --latest, but still run the command.

**Rationale**:

- npm only updates within semver range
- Alternative tools like `npm-check-updates` exist but require separate installation
- Better to warn and proceed than to fail

**Alternative**: Could integrate with `npx npm-check-updates`:

```bash
vp update react --latest
# For npm: npx npm-check-updates -u react && npm install
```

### 4. Interactive Mode

**Decision**: Support interactive mode for pnpm and yarn@2+.

**Rationale**:

- pnpm has `--interactive` flag
- yarn@2+ has `--interactive` flag
- Provides better UX for reviewing updates
- npm doesn't support this natively

### 5. Workspace Filtering

**Decision**: Use same filtering approach as add/remove commands.

**Rationale**:

- Consistency across commands
- Leverage existing filter patterns
- Works well with pnpm's filter syntax

## Error Handling

### No Package Manager Detected

```bash
$ vp update react
Error: No package manager detected
Please run one of:
  - vp install (to set up package manager)
  - Add packageManager field to package.json
```

### Interactive Mode Not Supported

```bash
$ vp update --interactive
Warning: npm doesn't support interactive mode
Running standard update instead...
```

## User Experience

### Success Output

```bash
$ vp update react --latest
Detected package manager: pnpm@10.15.0
Running: pnpm update --latest react

Packages: +0 -0 ~1
~1
Progress: resolved 150, reused 145, downloaded 1, added 0, done

dependencies:
~ react 18.2.0 → 18.3.1

Done in 1.2s
```

### Interactive Mode Output

```bash
$ vp up -i
Detected package manager: pnpm@10.15.0
Running: pnpm update --interactive

? Choose which packages to update: (Press <space> to select, <a> to select all)
❯◯ react 18.2.0 → 18.3.1
 ◯ react-dom 18.2.0 → 18.3.1
 ◯ typescript 5.0.0 → 5.5.0
 ◯ vite 5.0.0 → 6.0.0
```

## Alternative Designs Considered

### Alternative 1: Separate Command for Latest Updates

```bash
vp update react        # Update within range
vp upgrade react       # Update to latest
```

**Rejected because**:

- More commands to remember
- `--latest` flag is clearer
- Matches pnpm's API design

### Alternative 2: Always Update to Latest

```bash
vp update react        # Always updates to latest
vp update react --range # Updates within semver range
```

**Rejected because**:

- Breaks semver expectations
- Different from package manager defaults
- Could cause unexpected breaking changes

## Implementation Plan

### Phase 1: Core Functionality

1. Add `Update` command variant to `Commands` enum
2. Create `update.rs` module in both crates
3. Implement package manager command resolution
4. Add basic error handling

### Phase 2: Advanced Features

1. Add interactive mode support
2. Implement workspace filtering
3. Add dev/prod dependency filtering
4. Handle yarn version detection

### Phase 3: Testing

1. Unit tests for command resolution
2. Integration tests with mock package managers
3. Test interactive mode (where supported)
4. Test workspace operations

### Phase 4: Documentation

1. Update CLI documentation
2. Add examples to README
3. Document package manager compatibility

## Testing Strategy

### Test Package Manager Versions

- pnpm@9.x [WIP]
- pnpm@10.x
- yarn@1.x [WIP]
- yarn@4.x
- npm@10.x
- npm@11.x [WIP]
- bun@1.x [WIP]

### Unit Tests

```rust
#[test]
fn test_pnpm_update_basic() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.resolve_update_command(&UpdateCommandOptions {
        packages: &["react".to_string()],
        latest: false,
        ..Default::default()
    });
    assert_eq!(args, vec!["update", "react"]);
}

#[test]
fn test_pnpm_update_latest() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.resolve_update_command(&UpdateCommandOptions {
        packages: &["react".to_string()],
        latest: true,
        ..Default::default()
    });
    assert_eq!(args, vec!["update", "--latest", "react"]);
}

#[test]
fn test_npm_update_latest_warning() {
    // Should warn but still execute
    let pm = PackageManager::mock(PackageManagerType::Npm);
    let args = pm.resolve_update_command(&UpdateCommandOptions {
        packages: &["react".to_string()],
        latest: true,
        ..Default::default()
    });
    assert_eq!(args, vec!["update", "react"]);
}
```

## CLI Help Output

```bash
$ vp update --help
Update packages to their latest versions

Usage: vp update [PACKAGES]... [OPTIONS]

Aliases: up

Arguments:
  [PACKAGES]...  Packages to update (updates all if omitted)

Options:
  -L, --latest           Update to latest version (ignore semver range)
  -g, --global           Update global packages
  -r, --recursive        Update recursively in all workspace packages
  --filter <PATTERN>     Filter packages in monorepo (can be used multiple times)
  -w, --workspace-root   Include workspace root
  -D, --dev              Update only devDependencies
  -P, --prod             Update only dependencies
  -i, --interactive      Show outdated packages and choose which to update
  --no-optional          Don't update optionalDependencies
  --no-save              Update lockfile only, don't modify package.json
  -h, --help             Print help

Examples:
  vp update                          # Update all packages within semver range
  vp update react react-dom          # Update specific packages
  vp update --latest                 # Update all to latest versions
  vp up react -L                     # Update react to latest
  vp update -i                       # Interactive mode
  vp update --filter app             # Update in specific workspace
  vp update -r                       # Update in all workspaces
  vp update -D                       # Update only dev dependencies
  vp update --no-optional            # Skip optional dependencies
  vp update --no-save                # Update lockfile only
```

## Real-World Usage Examples

### Monorepo Package Updates

```bash
# Update React in all frontend packages to latest
vp update react react-dom --latest --filter "@myorg/app-*"

# Update all dev dependencies in all packages
vp update -D -r

# Interactive update in specific package
vp update -i --filter web

# Update all to latest in workspace root
vp update --latest -w

# Update TypeScript across entire monorepo
vp update typescript --latest -r
```

### Development Workflow

```bash
# Check for updates interactively
vp up -i

# Update all dependencies within semver range
vp update

# Update security patches
vp update

# Update to latest versions (major updates)
vp update --latest

# Update specific package to latest
vp up react -L

# Update global packages
vp update -g typescript

# Test updates without saving to package.json
vp update --no-save

# Update without optional dependencies
vp update --no-optional
```

## Package Manager Compatibility

| Feature          | pnpm               | yarn@1           | yarn@2+          | npm              | bun                    | Notes                      |
| ---------------- | ------------------ | ---------------- | ---------------- | ---------------- | ---------------------- | -------------------------- |
| Update command   | `update`           | `upgrade`        | `up`             | `update`         | `update`               | Different command names    |
| Latest flag      | `--latest` / `-L`  | `--latest`       | N/A (default)    | ❌ Not supported | `--latest`             | npm only updates in range  |
| Interactive      | `--interactive`    | ❌ Not supported | `--interactive`  | ❌ Not supported | `--interactive` / `-i` | Limited support            |
| Workspace filter | `--filter`         | ⚠️ Limited       | ⚠️ Limited       | `--workspace`    | N/A                    | pnpm most flexible         |
| Recursive        | `--recursive`      | ❌ Not supported | `--recursive`    | `--workspaces`   | `--recursive` / `-r`   | bun supports --recursive   |
| Dev/Prod filter  | `--dev` / `--prod` | ❌ Not supported | ❌ Not supported | ❌ Not supported | ❌ Not supported       | pnpm only                  |
| Global           | `-g`               | `global upgrade` | ❌ Not supported | `-g`             | ❌ Not supported       | Use npm for global         |
| No optional      | `--no-optional`    | ❌ Not supported | ❌ Not supported | `--no-optional`  | `--omit optional`      | Skip optional dependencies |
| No save          | `--no-save`        | ❌ Not supported | ❌ Not supported | `--no-save`      | `--no-save`            | Lockfile only updates      |

## Future Enhancements

### 1. Outdated Command

Show outdated packages before updating:

```bash
vp outdated
vp outdated --filter app
```

### 2. Smart Update Suggestions

```bash
$ vp update
Analyzing dependencies...
⚠️  Major updates available:
  react 17.0.0 → 18.3.1 (breaking changes)

✓ Minor updates:
  lodash 4.17.20 → 4.17.21

Run 'vp update --latest' to update to latest versions
Run 'vp update -i' for interactive mode
```

### 3. Changelog Display

```bash
$ vp update react --latest
Updating react 18.2.0 → 18.3.1

📝 Changelog:
  - New useOptimistic hook
  - Performance improvements
  - Bug fixes

Continue? (Y/n)
```

## Success Metrics

1. **Adoption**: % of users using `vp update` vs direct package manager
2. **Update Frequency**: Track how often dependencies are kept up-to-date
3. **User Feedback**: Survey/issues about command ergonomics
4. **Error Rate**: Track command failures vs package manager direct usage

## Conclusion

This RFC proposes adding `vp update` command to provide a unified interface for updating packages across pnpm/yarn/npm/bun. The design:

- ✅ Automatically adapts to detected package manager
- ✅ Supports updating specific packages or all packages
- ✅ Provides `--latest` flag to update beyond semver range
- ✅ Full workspace support with filtering
- ✅ Interactive mode for better UX (where supported)
- ✅ Graceful degradation for package manager-specific features
- ✅ No caching overhead
- ✅ Simple implementation leveraging existing infrastructure

The implementation follows the same patterns as add/remove commands while providing the update-specific features developers need.
