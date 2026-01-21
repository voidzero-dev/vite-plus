# RFC: Vite+ Install Command

## Summary

Add `vite install` command (alias: `vite i`) that automatically adapts to the detected package manager (pnpm/yarn/npm) for installing all dependencies in a project, with support for common flags and workspace-aware operations based on pnpm's API design.

## Motivation

Currently, developers must manually use package manager-specific commands:

```bash
pnpm install
yarn install
npm install
```

This creates friction in monorepo workflows and requires remembering different syntaxes. A unified interface would:

1. **Simplify workflows**: One command works across all package managers
2. **Auto-detection**: Automatically uses the correct package manager
3. **Consistency**: Same syntax regardless of underlying tool
4. **Integration**: Works seamlessly with existing Vite+ features

### Current Pain Points

```bash
# Developer needs to know which package manager is used
pnpm install --frozen-lockfile  # pnpm project
yarn install --frozen-lockfile  # yarn project (v1) or --immutable (v2+)
npm ci                          # npm project (clean install)

# Different flags for production install
pnpm install --prod
yarn install --production
npm install --omit=dev
```

### Proposed Solution

```bash
# Works for all package managers
vite install
vite i

# With flags
vite install --frozen-lockfile
vite install --prod
vite install --ignore-scripts

# Workspace operations
vite install --filter app
```

### Command Syntax

```bash
vite install [OPTIONS]
vite i [OPTIONS]
```

**Examples:**

```bash
# Install all dependencies
vite install
vite i

# Production install (no devDependencies)
vite install --prod
vite install -P

# Frozen lockfile (CI mode)
vite install --frozen-lockfile

# Prefer offline (use cache when available)
vite install --prefer-offline

# Force reinstall
vite install --force

# Ignore scripts
vite install --ignore-scripts

# Workspace operations
vite install --filter app              # Install for specific package
```

### Command Options

| Option                 | Short | Description                                              |
| ---------------------- | ----- | -------------------------------------------------------- |
| `--prod`               | `-P`  | Do not install devDependencies                           |
| `--dev`                | `-D`  | Only install devDependencies                             |
| `--no-optional`        |       | Do not install optionalDependencies                      |
| `--frozen-lockfile`    |       | Fail if lockfile needs to be updated                     |
| `--no-frozen-lockfile` |       | Allow lockfile updates (opposite of --frozen-lockfile)   |
| `--lockfile-only`      |       | Only update lockfile, don't install                      |
| `--prefer-offline`     |       | Use cached packages when available                       |
| `--offline`            |       | Only use packages already in cache                       |
| `--force`              | `-f`  | Force reinstall all dependencies                         |
| `--ignore-scripts`     |       | Do not run lifecycle scripts                             |
| `--no-lockfile`        |       | Don't read or generate lockfile                          |
| `--fix-lockfile`       |       | Fix broken lockfile entries                              |
| `--shamefully-hoist`   |       | Create flat node_modules (pnpm)                          |
| `--resolution-only`    |       | Re-run resolution for peer dependency analysis           |
| `--silent`             |       | Suppress output (silent mode)                            |
| `--filter <pattern>`   |       | Filter packages in monorepo                              |
| `--workspace-root`     | `-w`  | Install in workspace root only                           |
| `--save-exact`         | `-E`  | Save exact version (only when adding packages)           |
| `--save-peer`          |       | Save to peerDependencies (only when adding packages)     |
| `--save-optional`      | `-O`  | Save to optionalDependencies (only when adding packages) |
| `--save-catalog`       |       | Save to default catalog (only when adding packages)      |
| `--global`             | `-g`  | Install globally (only when adding packages)             |

### Command Mapping

#### Install Command Mapping

- https://pnpm.io/cli/install
- https://yarnpkg.com/cli/install
- https://classic.yarnpkg.com/en/docs/cli/install
- https://docs.npmjs.com/cli/v11/commands/npm-install

| Vite+ Flag             | pnpm                   | yarn@1                 | yarn@2+                                     | npm                         | Description                          |
| ---------------------- | ---------------------- | ---------------------- | ------------------------------------------- | --------------------------- | ------------------------------------ |
| `vite install`         | `pnpm install`         | `yarn install`         | `yarn install`                              | `npm install`               | Install all dependencies             |
| `--prod, -P`           | `--prod`               | `--production`         | N/A (use `.yarnrc.yml`)                     | `--omit=dev`                | Skip devDependencies                 |
| `--dev, -D`            | `--dev`                | N/A                    | N/A                                         | `--include=dev --omit=prod` | Only devDependencies                 |
| `--no-optional`        | `--no-optional`        | `--ignore-optional`    | N/A                                         | `--omit=optional`           | Skip optionalDependencies            |
| `--frozen-lockfile`    | `--frozen-lockfile`    | `--frozen-lockfile`    | `--immutable`                               | `ci` (use `npm ci`)         | Fail if lockfile outdated            |
| `--no-frozen-lockfile` | `--no-frozen-lockfile` | `--no-frozen-lockfile` | `--no-immutable`                            | `install` (not `ci`)        | Allow lockfile updates               |
| `--lockfile-only`      | `--lockfile-only`      | N/A                    | `--mode update-lockfile`                    | `--package-lock-only`       | Only update lockfile                 |
| `--prefer-offline`     | `--prefer-offline`     | `--prefer-offline`     | N/A                                         | `--prefer-offline`          | Prefer cached packages               |
| `--offline`            | `--offline`            | `--offline`            | N/A                                         | `--offline`                 | Only use cache                       |
| `--force, -f`          | `--force`              | `--force`              | N/A                                         | `--force`                   | Force reinstall                      |
| `--ignore-scripts`     | `--ignore-scripts`     | `--ignore-scripts`     | `--mode skip-build`                         | `--ignore-scripts`          | Skip lifecycle scripts               |
| `--no-lockfile`        | `--no-lockfile`        | `--no-lockfile`        | N/A                                         | `--no-package-lock`         | Skip lockfile                        |
| `--fix-lockfile`       | `--fix-lockfile`       | N/A                    | `--refresh-lockfile`                        | N/A                         | Fix broken lockfile entries          |
| `--shamefully-hoist`   | `--shamefully-hoist`   | N/A                    | N/A                                         | N/A                         | Flat node_modules (pnpm)             |
| `--resolution-only`    | `--resolution-only`    | N/A                    | N/A                                         | N/A                         | Re-run resolution only (pnpm)        |
| `--silent`             | `--silent`             | `--silent`             | N/A (use env var)                           | `--loglevel silent`         | Suppress output                      |
| `--filter <pattern>`   | `--filter <pattern>`   | N/A                    | `workspaces foreach -A --include <pattern>` | `--workspace <pattern>`     | Target specific workspace package(s) |
| `-w, --workspace-root` | `-w`                   | `-W`                   | N/A                                         | `--include-workspace-root`  | Install in root only                 |

**Notes:**

- `--frozen-lockfile`: For npm, this maps to `npm ci` command instead of `npm install`
- `--no-frozen-lockfile`: Takes higher priority over `--frozen-lockfile` when both are specified. Passed through to the actual package manager (pnpm: `--no-frozen-lockfile`, yarn@1: `--no-frozen-lockfile`, yarn@2+: `--no-immutable`, npm: uses `npm install` instead of `npm ci`)
- `--prod`: yarn@2+ requires configuration in `.yarnrc.yml` instead of CLI flag
- `--ignore-scripts`: For yarn@2+, this maps to `--mode skip-build`
- `--fix-lockfile`: Automatically fixes broken lockfile entries (pnpm and yarn@2+ only, npm does not support)
- `--resolution-only`: Re-runs dependency resolution without installing packages. Useful for peer dependency analysis (pnpm only)
- `--shamefully-hoist`: pnpm-specific, creates flat node_modules like npm/yarn
- `--silent`: Suppresses output. For yarn@2+, use `YARN_ENABLE_PROGRESS=false` environment variable instead. For npm, maps to `--loglevel silent`

**Add Package Mode:**

When packages are provided as arguments (e.g., `vite install react`), the command acts as an alias for `vite add`:

- `--save-exact, -E`: Save exact version rather than semver range
- `--save-peer`: Save to peerDependencies (and devDependencies)
- `--save-optional, -O`: Save to optionalDependencies
- `--save-catalog`: Save to the default catalog (pnpm only)
- `--global, -g`: Install globally

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

**Multiple Filters:**

```bash
vite install --filter app --filter web  # Install for both app and web
vite install --filter "app*" --filter "!app-test"  # app* except app-test
```

**Note**: For pnpm, `--filter` must come before the command (e.g., `pnpm --filter app install`). For yarn/npm, it's integrated into the command structure.

#### Pass-Through Arguments

Additional parameters not covered by Vite+ can be handled through pass-through arguments.

All arguments after `--` will be passed through to the package manager.

```bash
vite install -- --use-stderr

-> pnpm install --use-stderr
-> yarn install --use-stderr
-> npm install --use-stderr
```

### Implementation Architecture

#### 1. Command Structure

**File**: `crates/vite_global/src/lib.rs`

Add new command variant:

```rust
#[derive(Subcommand, Debug)]
pub enum Commands {
    // ... existing commands

    /// Install all dependencies
    #[command(disable_help_flag = true, alias = "i")]
    Install {
        /// Do not install devDependencies
        #[arg(short = 'P', long)]
        prod: bool,

        /// Only install devDependencies
        #[arg(short = 'D', long)]
        dev: bool,

        /// Do not install optionalDependencies
        #[arg(long)]
        no_optional: bool,

        /// Fail if lockfile needs to be updated (CI mode)
        #[arg(long)]
        frozen_lockfile: bool,

        /// Only update lockfile, don't install
        #[arg(long)]
        lockfile_only: bool,

        /// Use cached packages when available
        #[arg(long)]
        prefer_offline: bool,

        /// Only use packages already in cache
        #[arg(long)]
        offline: bool,

        /// Force reinstall all dependencies
        #[arg(short = 'f', long)]
        force: bool,

        /// Do not run lifecycle scripts
        #[arg(long)]
        ignore_scripts: bool,

        /// Don't read or generate lockfile
        #[arg(long)]
        no_lockfile: bool,

        /// Fix broken lockfile entries
        #[arg(long)]
        fix_lockfile: bool,

        /// Create flat node_modules (pnpm only)
        #[arg(long)]
        shamefully_hoist: bool,

        /// Re-run resolution for peer dependency analysis
        #[arg(long)]
        resolution_only: bool,

        /// Filter packages in monorepo (can be used multiple times)
        #[arg(long, value_name = "PATTERN")]
        filter: Vec<String>,

        /// Install in workspace root only
        #[arg(short = 'w', long)]
        workspace_root: bool,

        /// Arguments to pass to package manager
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
}
```

#### 2. Package Manager Adapter

**File**: `crates/vite_package_manager/src/commands/install.rs`

Add methods to translate commands:

```rust
impl PackageManager {
    /// Build install command arguments
    pub fn build_install_args(&self, options: &InstallOptions) -> InstallCommandResult {
        let mut args = Vec::new();
        let mut use_ci = false;

        match self.client {
            PackageManagerType::Pnpm => {
                // pnpm: --filter must come before command
                for filter in &options.filters {
                    args.push("--filter".to_string());
                    args.push(filter.clone());
                }

                args.push("install".to_string());

                if options.prod {
                    args.push("--prod".to_string());
                }
                if options.dev {
                    args.push("--dev".to_string());
                }
                if options.no_optional {
                    args.push("--no-optional".to_string());
                }
                if options.frozen_lockfile {
                    args.push("--frozen-lockfile".to_string());
                }
                if options.lockfile_only {
                    args.push("--lockfile-only".to_string());
                }
                if options.prefer_offline {
                    args.push("--prefer-offline".to_string());
                }
                if options.offline {
                    args.push("--offline".to_string());
                }
                if options.force {
                    args.push("--force".to_string());
                }
                if options.ignore_scripts {
                    args.push("--ignore-scripts".to_string());
                }
                if options.no_lockfile {
                    args.push("--no-lockfile".to_string());
                }
                if options.fix_lockfile {
                    args.push("--fix-lockfile".to_string());
                }
                if options.shamefully_hoist {
                    args.push("--shamefully-hoist".to_string());
                }
                if options.resolution_only {
                    args.push("--resolution-only".to_string());
                }
                if options.workspace_root {
                    args.push("-w".to_string());
                }
            }

            PackageManagerType::Yarn => {
                args.push("install".to_string());

                if self.is_yarn_berry() {
                    // yarn@2+ (Berry)
                    if options.frozen_lockfile {
                        args.push("--immutable".to_string());
                    }
                    if options.lockfile_only {
                        args.push("--mode".to_string());
                        args.push("update-lockfile".to_string());
                    }
                    if options.fix_lockfile {
                        args.push("--refresh-lockfile".to_string());
                    }
                    if options.ignore_scripts {
                        args.push("--mode".to_string());
                        args.push("skip-build".to_string());
                    }
                    if options.resolution_only {
                        eprintln!("Warning: yarn@2+ does not support --resolution-only");
                    }
                    // Note: yarn@2+ uses .yarnrc.yml for prod
                    if options.prod {
                        eprintln!("Warning: yarn@2+ requires configuration in .yarnrc.yml for --prod behavior");
                    }
                    // yarn@2+ filter is handled differently - needs workspaces foreach
                    if !options.filters.is_empty() {
                        // For yarn@2+, we need to use: yarn workspaces foreach -A --include <pattern> install
                        // This requires restructuring the command
                        args.clear();
                        args.push("workspaces".to_string());
                        args.push("foreach".to_string());
                        args.push("-A".to_string());
                        for filter in &options.filters {
                            args.push("--include".to_string());
                            args.push(filter.clone());
                        }
                        args.push("install".to_string());
                    }
                } else {
                    // yarn@1 (Classic)
                    if options.prod {
                        args.push("--production".to_string());
                    }
                    if options.no_optional {
                        args.push("--ignore-optional".to_string());
                    }
                    if options.frozen_lockfile {
                        args.push("--frozen-lockfile".to_string());
                    }
                    if options.prefer_offline {
                        args.push("--prefer-offline".to_string());
                    }
                    if options.offline {
                        args.push("--offline".to_string());
                    }
                    if options.force {
                        args.push("--force".to_string());
                    }
                    if options.ignore_scripts {
                        args.push("--ignore-scripts".to_string());
                    }
                    if options.no_lockfile {
                        args.push("--no-lockfile".to_string());
                    }
                    if options.fix_lockfile {
                        eprintln!("Warning: yarn@1 does not support --fix-lockfile");
                    }
                    if options.resolution_only {
                        eprintln!("Warning: yarn@1 does not support --resolution-only");
                    }
                    if options.workspace_root {
                        args.push("-W".to_string());
                    }
                }
            }

            PackageManagerType::Npm => {
                // npm: Use `npm ci` for frozen-lockfile
                if options.frozen_lockfile {
                    args.push("ci".to_string());
                    use_ci = true;
                } else {
                    args.push("install".to_string());
                }

                if options.prod {
                    args.push("--omit=dev".to_string());
                }
                if options.dev && !use_ci {
                    args.push("--include=dev".to_string());
                    args.push("--omit=prod".to_string());
                }
                if options.no_optional {
                    args.push("--omit=optional".to_string());
                }
                if options.lockfile_only && !use_ci {
                    args.push("--package-lock-only".to_string());
                }
                if options.prefer_offline {
                    args.push("--prefer-offline".to_string());
                }
                if options.offline {
                    args.push("--offline".to_string());
                }
                if options.force && !use_ci {
                    args.push("--force".to_string());
                }
                if options.ignore_scripts {
                    args.push("--ignore-scripts".to_string());
                }
                if options.no_lockfile && !use_ci {
                    args.push("--no-package-lock".to_string());
                }
                if options.fix_lockfile {
                    eprintln!("Warning: npm does not support --fix-lockfile");
                }
                if options.resolution_only {
                    eprintln!("Warning: npm does not support --resolution-only");
                }
                if options.workspace_root {
                    args.push("--include-workspace-root".to_string());
                }
                for filter in &options.filters {
                    args.push("--workspace".to_string());
                    args.push(filter.clone());
                }
            }
        }

        // Pass through extra args
        args.extend_from_slice(&options.extra_args);

        InstallCommandResult {
            command: if use_ci { "ci".to_string() } else { "install".to_string() },
            args,
        }
    }

    fn is_yarn_berry(&self) -> bool {
        // yarn@2+ is called "Berry"
        !self.version.starts_with("1.")
    }
}

pub struct InstallOptions {
    pub prod: bool,
    pub dev: bool,
    pub no_optional: bool,
    pub frozen_lockfile: bool,
    pub lockfile_only: bool,
    pub prefer_offline: bool,
    pub offline: bool,
    pub force: bool,
    pub ignore_scripts: bool,
    pub no_lockfile: bool,
    pub fix_lockfile: bool,
    pub shamefully_hoist: bool,
    pub resolution_only: bool,
    pub filters: Vec<String>,
    pub workspace_root: bool,
    pub extra_args: Vec<String>,
}

pub struct InstallCommandResult {
    pub command: String,
    pub args: Vec<String>,
}
```

#### 3. Install Command Implementation

**File**: `crates/vite_global/src/install.rs` (new file)

```rust
use vite_error::Error;
use vite_path::AbsolutePathBuf;
use vite_package_manager::{PackageManager, InstallOptions};

pub struct InstallCommand {
    workspace_root: AbsolutePathBuf,
}

impl InstallCommand {
    pub fn new(workspace_root: AbsolutePathBuf) -> Self {
        Self { workspace_root }
    }

    pub async fn execute(self, options: InstallOptions) -> Result<(), Error> {
        let package_manager = PackageManager::builder(&self.workspace_root).build().await?;

        let resolve_command = package_manager.resolve_command();
        let install_result = package_manager.build_install_args(&options);

        let status = package_manager
            .run_command(&install_result.args, &self.workspace_root)
            .await?;

        if !status.success() {
            return Err(Error::CommandFailed {
                command: format!("install"),
                exit_code: status.code(),
            });
        }

        Ok(())
    }
}
```

## Design Decisions

### 1. No Caching

**Decision**: Do not cache install operations.

**Rationale**:

- Install commands modify node_modules and lockfiles
- Side effects make caching inappropriate
- Each execution should run fresh
- Package managers have their own caching mechanisms

### 2. Frozen Lockfile for CI

**Decision**: Map `--frozen-lockfile` to `npm ci` for npm.

**Rationale**:

- `npm ci` is the recommended way to do clean installs in CI
- It's faster than `npm install --frozen-lockfile`
- Automatically removes existing node_modules
- Better aligns with CI best practices

### 3. Pass-Through Arguments

**Decision**: Pass all arguments after `--` directly to package manager.

**Rationale**:

- Package managers have many flags (40+ for npm)
- Maintaining complete flag mapping is error-prone
- Pass-through allows accessing all features
- Only translate critical differences

### 4. Workspace Support

**Decision**: Support workspace filtering with `--filter` flag.

**Rationale**:

- Monorepo workflows need selective installation
- pnpm's filter syntax is most powerful
- Graceful degradation for other package managers
- Consistent with other Vite+ commands

### 5. Alias Support

**Decision**: Support `vite i` as alias for `vite install`.

**Rationale**:

- Matches npm/yarn/pnpm convention (`npm i`, `yarn`, `pnpm i`)
- Faster to type
- Familiar to developers

## Error Handling

### No Package Manager Detected

```bash
$ vite install
Error: No package manager detected
Please run one of:
  - vite install (after adding packageManager to package.json)
  - Add packageManager field to package.json
```

### Lockfile Out of Date

```bash
$ vite install --frozen-lockfile
Detected package manager: pnpm@10.15.0
Running: pnpm install --frozen-lockfile

ERR_PNPM_OUTDATED_LOCKFILE  Cannot install with "frozen-lockfile" because pnpm-lock.yaml is not up to date with package.json

Error: Command failed with exit code 1
```

### Network Error

```bash
$ vite install --offline
Detected package manager: npm@11.0.0
Running: npm install --offline

npm ERR! code E404
npm ERR! 404 Not Found - GET https://registry.npmjs.org/some-package - Package not found in cache

Error: Command failed with exit code 1
```

## User Experience

### Basic Install

```bash
$ vite install
Detected package manager: pnpm@10.15.0
Running: pnpm install

Lockfile is up to date, resolution step is skipped
Packages: +150
+++++++++++++++++++++++++++++++++++
Progress: resolved 150, reused 150, downloaded 0, added 150, done

Done in 1.2s
```

### CI Install

```bash
$ vite install --frozen-lockfile
Detected package manager: npm@11.0.0
Running: npm ci

added 150 packages in 2.3s

Done in 2.3s
```

### Production Install

```bash
$ vite install --prod
Detected package manager: pnpm@10.15.0
Running: pnpm install --prod

Packages: +80
++++++++++++++++++++
Progress: resolved 80, reused 80, downloaded 0, added 80, done

Done in 0.8s
```

### Workspace Install

```bash
$ vite install --filter app
Detected package manager: pnpm@10.15.0
Running: pnpm --filter app install

Scope: 1 of 5 workspace projects
Packages: +50
++++++++++++++
Progress: resolved 50, reused 50, downloaded 0, added 50, done

Done in 0.5s
```

## Alternative Designs Considered

### Alternative 1: Always Use Native Commands

```bash
# Let user call package manager directly
pnpm install
yarn install
npm install
```

**Rejected because**:

- No abstraction benefit
- Scripts not portable
- Requires knowing package manager
- Inconsistent developer experience

### Alternative 2: Custom Install Logic

Implement our own dependency resolution and installation:

```rust
// Custom dependency resolver
let deps = resolve_dependencies(&package_json)?;
download_packages(&deps)?;
link_packages(&deps)?;
```

**Rejected because**:

- Enormous complexity
- Package managers are well-tested
- Would miss PM-specific optimizations
- Maintenance burden

### Alternative 3: Environment Variable Detection

```bash
# Detect package manager from environment
VITE_PM=pnpm vite install
```

**Rejected because**:

- Less convenient than auto-detection
- Requires extra configuration
- Not portable across machines
- Existing lockfile detection works well

## Implementation Plan

### Phase 1: Core Functionality

1. Add `Install` command variant to `Commands` enum
2. Create `install.rs` module
3. Implement package manager command resolution
4. Add basic flag translation

### Phase 2: Advanced Features

1. Implement workspace filtering
2. Add `--frozen-lockfile` to `npm ci` mapping
3. Handle yarn@1 vs yarn@2+ differences
4. Add pass-through argument support

### Phase 3: Testing

1. Unit tests for command resolution
2. Integration tests with mock package managers
3. Manual testing with real package managers
4. CI workflow testing

### Phase 4: Documentation

1. Update CLI documentation
2. Add examples to README
3. Document flag compatibility matrix
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
fn test_pnpm_basic_install() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm, "10.0.0");
    let options = InstallOptions::default();
    let result = pm.build_install_args(&options);
    assert_eq!(result.args, vec!["install"]);
}

#[test]
fn test_pnpm_prod_install() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm, "10.0.0");
    let options = InstallOptions { prod: true, ..Default::default() };
    let result = pm.build_install_args(&options);
    assert_eq!(result.args, vec!["install", "--prod"]);
}

#[test]
fn test_npm_frozen_lockfile_uses_ci() {
    let pm = PackageManager::mock(PackageManagerType::Npm, "11.0.0");
    let options = InstallOptions { frozen_lockfile: true, ..Default::default() };
    let result = pm.build_install_args(&options);
    assert_eq!(result.command, "ci");
}

#[test]
fn test_yarn_berry_frozen_lockfile() {
    let pm = PackageManager::mock(PackageManagerType::Yarn, "4.0.0");
    let options = InstallOptions { frozen_lockfile: true, ..Default::default() };
    let result = pm.build_install_args(&options);
    assert_eq!(result.args, vec!["install", "--immutable"]);
}

#[test]
fn test_pnpm_filter() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm, "10.0.0");
    let options = InstallOptions {
        filters: vec!["app".to_string()],
        ..Default::default()
    };
    let result = pm.build_install_args(&options);
    assert_eq!(result.args, vec!["--filter", "app", "install"]);
}

#[test]
fn test_npm_workspace_filter() {
    let pm = PackageManager::mock(PackageManagerType::Npm, "11.0.0");
    let options = InstallOptions {
        filters: vec!["app".to_string()],
        ..Default::default()
    };
    let result = pm.build_install_args(&options);
    assert_eq!(result.args, vec!["install", "--workspace", "app"]);
}

#[test]
fn test_pnpm_fix_lockfile() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm, "10.0.0");
    let options = InstallOptions { fix_lockfile: true, ..Default::default() };
    let result = pm.build_install_args(&options);
    assert_eq!(result.args, vec!["install", "--fix-lockfile"]);
}

#[test]
fn test_yarn_berry_fix_lockfile() {
    let pm = PackageManager::mock(PackageManagerType::Yarn, "4.0.0");
    let options = InstallOptions { fix_lockfile: true, ..Default::default() };
    let result = pm.build_install_args(&options);
    assert_eq!(result.args, vec!["install", "--refresh-lockfile"]);
}

#[test]
fn test_yarn_berry_ignore_scripts() {
    let pm = PackageManager::mock(PackageManagerType::Yarn, "4.0.0");
    let options = InstallOptions { ignore_scripts: true, ..Default::default() };
    let result = pm.build_install_args(&options);
    assert_eq!(result.args, vec!["install", "--mode", "skip-build"]);
}

#[test]
fn test_pnpm_resolution_only() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm, "10.0.0");
    let options = InstallOptions { resolution_only: true, ..Default::default() };
    let result = pm.build_install_args(&options);
    assert_eq!(result.args, vec!["install", "--resolution-only"]);
}

#[test]
fn test_yarn_berry_filter() {
    let pm = PackageManager::mock(PackageManagerType::Yarn, "4.0.0");
    let options = InstallOptions {
        filters: vec!["app".to_string()],
        ..Default::default()
    };
    let result = pm.build_install_args(&options);
    assert_eq!(result.args, vec!["workspaces", "foreach", "-A", "--include", "app", "install"]);
}
```

### Integration Tests

Create fixtures for testing with each package manager:

```
fixtures/install-test/
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

1. Basic install
2. Production install
3. Frozen lockfile install
4. Workspace filter install
5. Recursive install
6. Offline install
7. Force reinstall
8. Ignore scripts install

## CLI Help Output

```bash
$ vite install --help
Install all dependencies, or add packages if package names are provided

Usage: vite install [OPTIONS] [PACKAGES]...

Aliases: i

Options:
  -P, --prod               Do not install devDependencies
  -D, --dev                Only install devDependencies (install) / Save to devDependencies (add)
      --no-optional        Do not install optionalDependencies
      --frozen-lockfile    Fail if lockfile needs to be updated (CI mode)
      --no-frozen-lockfile Allow lockfile updates (opposite of --frozen-lockfile)
      --lockfile-only      Only update lockfile, don't install
      --prefer-offline     Use cached packages when available
      --offline            Only use packages already in cache
  -f, --force              Force reinstall all dependencies
      --ignore-scripts     Do not run lifecycle scripts
      --no-lockfile        Don't read or generate lockfile
      --fix-lockfile       Fix broken lockfile entries
      --shamefully-hoist   Create flat node_modules (pnpm only)
      --resolution-only    Re-run resolution for peer dependency analysis
      --silent             Suppress output (silent mode)
      --filter <PATTERN>   Filter packages in monorepo (can be used multiple times)
  -w, --workspace-root     Install in workspace root only
  -E, --save-exact         Save exact version (only when adding packages)
      --save-peer          Save to peerDependencies (only when adding packages)
  -O, --save-optional      Save to optionalDependencies (only when adding packages)
      --save-catalog       Save to default catalog (only when adding packages)
  -g, --global             Install globally (only when adding packages)
  -h, --help               Print help

Examples:
  vite install                      # Install all dependencies
  vite i                            # Short alias
  vite install --prod               # Production install
  vite install --frozen-lockfile    # CI mode (strict lockfile)
  vite install --filter app         # Install for specific package
  vite install --silent             # Silent install
  vite install react                # Add react (alias for vite add)
  vite install -D typescript        # Add typescript as devDependency
  vite install --save-peer react    # Add react as peerDependency
```

## Performance Considerations

1. **Delegate to Package Manager**: Leverage PM's built-in optimizations
2. **No Additional Overhead**: Minimal processing before running PM command
3. **Cache Utilization**: Support `--prefer-offline` and `--offline` flags
4. **Parallel Installation**: Package managers handle parallelization

## Security Considerations

1. **Script Execution**: `--ignore-scripts` prevents untrusted script execution
2. **Lockfile Integrity**: `--frozen-lockfile` ensures reproducible installs
3. **Network Security**: Package managers handle registry authentication
4. **Pass-Through Safety**: Arguments are passed through safely

## Backward Compatibility

This is a new feature with no breaking changes:

- Existing commands unaffected
- New command is additive
- No changes to task configuration
- No changes to caching behavior

## Package Manager Compatibility Matrix

| Feature                | pnpm | yarn@1 | yarn@2+                 | npm             | Notes                     |
| ---------------------- | ---- | ------ | ----------------------- | --------------- | ------------------------- |
| Basic install          | ✅   | ✅     | ✅                      | ✅              | All supported             |
| `--prod`               | ✅   | ✅     | ⚠️                      | ✅              | yarn@2+ needs .yarnrc.yml |
| `--dev`                | ✅   | ❌     | ❌                      | ✅              | Limited support           |
| `--no-optional`        | ✅   | ✅     | ⚠️                      | ✅              | yarn@2+ needs .yarnrc.yml |
| `--frozen-lockfile`    | ✅   | ✅     | ✅ `--immutable`        | ✅ `ci`         | npm uses `npm ci`         |
| `--no-frozen-lockfile` | ✅   | ✅     | ✅ `--no-immutable`     | ✅ `install`    | Pass through to PM        |
| `--lockfile-only`      | ✅   | ❌     | ✅                      | ✅              | yarn@1 not supported      |
| `--prefer-offline`     | ✅   | ✅     | ❌                      | ✅              | yarn@2+ not supported     |
| `--offline`            | ✅   | ✅     | ❌                      | ✅              | yarn@2+ not supported     |
| `--force`              | ✅   | ✅     | ❌                      | ✅              | yarn@2+ not supported     |
| `--ignore-scripts`     | ✅   | ✅     | ✅ `--mode skip-build`  | ✅              | All supported             |
| `--no-lockfile`        | ✅   | ✅     | ❌                      | ✅              | yarn@2+ not supported     |
| `--fix-lockfile`       | ✅   | ❌     | ✅ `--refresh-lockfile` | ❌              | pnpm and yarn@2+ only     |
| `--shamefully-hoist`   | ✅   | ❌     | ❌                      | ❌              | pnpm only                 |
| `--resolution-only`    | ✅   | ❌     | ❌                      | ❌              | pnpm only                 |
| `--silent`             | ✅   | ✅     | ⚠️ (use env var)        | ✅ `--loglevel` | yarn@2+ use env var       |
| `--filter`             | ✅   | ❌     | ✅ `workspaces foreach` | ✅              | yarn@1 not supported      |

## Future Enhancements

### 1. Interactive Mode

```bash
$ vite install --interactive
? Select packages to install:
  [x] dependencies (150 packages)
  [ ] devDependencies (80 packages)
  [x] optionalDependencies (5 packages)
```

### 2. Install Progress

```bash
$ vite install --progress
Installing dependencies...
[============================] 100% | 150/150 packages
```

### 3. Dependency Analysis

```bash
$ vite install --analyze
Installing dependencies...

Added packages:
  react@18.3.1 (85KB)
  react-dom@18.3.1 (120KB)

Total: 150 packages, 12.3MB

Done in 2.3s
```

### 4. Selective Updates

```bash
$ vite install --update react
# Install and update specific package
```

## Real-World Usage Examples

### CI Pipeline

```yaml
# .github/workflows/ci.yml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install dependencies
        run: vite install --frozen-lockfile

      - name: Build
        run: vite build
```

### Docker Production Build

```dockerfile
FROM node:20-alpine

WORKDIR /app
COPY package.json pnpm-lock.yaml ./

# Production install only
RUN npm install -g @voidzero/global && \
    vite install --prod --frozen-lockfile

COPY . .
RUN vite build
```

### Monorepo Development

```bash
# Install dependencies for specific package
vite install --filter @myorg/web-app

# Force reinstall after branch switch
vite install --force
```

### Offline Development

```bash
# Populate cache first
vite install

# Later, work offline
vite install --offline
```

## Open Questions

1. **Should we support `--check` flag?**
   - Proposed: Add `--check` to verify lockfile without installing
   - Similar to `pnpm install --lockfile-only` but without writing

2. **Should we auto-detect CI environment?**
   - Proposed: Auto-enable `--frozen-lockfile` in CI (like pnpm)
   - Could check `CI` environment variable

3. **Should we support package manager version pinning?**
   - Proposed: Respect `packageManager` field in package.json
   - Already implemented in package manager detection

4. **How to handle conflicting flags?**
   - Proposed: Let package manager handle conflicts
   - Example: `--prod` and `--dev` together

## Conclusion

This RFC proposes adding `vite install` command to provide a unified interface for installing dependencies across pnpm/yarn/npm. The design:

- ✅ Automatically adapts to detected package manager
- ✅ Supports common installation flags
- ✅ Full workspace support following pnpm's API design
- ✅ Uses pass-through for maximum flexibility
- ✅ No caching overhead (delegates to package manager)
- ✅ Simple implementation leveraging existing infrastructure
- ✅ CI-friendly with `--frozen-lockfile` support
- ✅ Extensible for future enhancements

The implementation follows the same patterns as other package management commands (`add`, `remove`, `update`) while providing a unified, intuitive interface for dependency installation.
