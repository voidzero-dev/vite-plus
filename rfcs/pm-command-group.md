# RFC: Vite+ Package Manager Utilities Command Group

## Summary

Add `vite pm` command group that provides a set of utilities for working with package managers. The `pm` command group offers direct access to package manager utilities like cache management, package publishing, configuration, and more. These are pass-through commands that delegate to the detected package manager (pnpm/npm/yarn) with minimal processing, providing a unified interface across different package managers.

## Motivation

Currently, developers must use package manager-specific commands for various utilities:

```bash
# Cache management
pnpm store path
npm cache dir
yarn cache dir

# Package publishing
pnpm publish
npm publish
yarn publish

# Package information
pnpm list
npm list
yarn list

# Configuration
pnpm config get
npm config get
yarn config get
```

This creates several issues:

1. **Cognitive Load**: Developers must remember different commands and flags for each package manager
2. **Context Switching**: When working across projects with different package managers, developers need to switch mental models
3. **Script Portability**: Scripts that use package manager utilities are tied to a specific package manager
4. **Missing Abstraction**: While vite+ provides abstractions for install/add/remove/update, it lacks utilities for cache, publish, config, etc.

### Current Pain Points

```bash
# Developer needs to know which package manager is used
pnpm store path                       # pnpm project
npm cache dir                         # npm project
yarn cache dir                        # yarn project

# Different command names
pnpm list --depth 0                   # pnpm - list packages
npm list --depth 0                    # npm - list packages
yarn list --depth 0                   # yarn - list packages

# Different config commands
pnpm config get registry              # pnpm
npm config get registry               # npm
yarn config get registry              # yarn

# Different cache cleaning
pnpm store prune                      # pnpm
npm cache clean --force               # npm
yarn cache clean                      # yarn
```

### Proposed Solution

```bash
# Works for all package managers
vite pm cache                         # Show cache directory
vite pm cache clean                   # Clean cache
vite pm list                          # List installed packages
vite pm config get registry           # Get config value
vite pm publish                       # Publish package
vite pm pack                          # Create package tarball
vite pm prune                         # Remove unnecessary packages
vite pm owner list <pkg>              # List package owners
vite pm view <pkg>                    # View package information
```

## Proposed Solution

### Command Syntax

```bash
vite pm <subcommand> [OPTIONS] [ARGS...]
```

**Subcommands:**

1. **prune**: Remove unnecessary packages
2. **pack**: Create a tarball of the package
3. **list** (alias: **ls**): List installed packages
4. **view**: View package information from registry
5. **publish**: Publish package to registry
6. **owner**: Manage package owners
7. **cache**: Manage package cache
8. **config**: Manage package manager configuration

### Subcommand Details

#### 1. vite pm prune

Remove unnecessary packages from node_modules.

```bash
vite pm prune [OPTIONS]
```

**Examples:**

```bash
# Remove all extraneous packages
vite pm prune

# Remove devDependencies (production only)
vite pm prune --prod

# Remove optional dependencies
vite pm prune --no-optional
```

**Options:**

- `--prod`: Remove devDependencies
- `--no-optional`: Remove optional dependencies

#### 2. vite pm pack

Create a tarball archive of the package.

```bash
vite pm pack [OPTIONS]
```

**Examples:**

```bash
# Create tarball in current directory
vite pm pack

# Specify output file path
vite pm pack --out ./dist/package.tgz

# Use placeholders for package name and version (pnpm/yarn@2+ only)
vite pm pack --out ./dist/%s-%v.tgz

# Specify output directory
vite pm pack --pack-destination ./dist

# Custom gzip compression level
vite pm pack --pack-gzip-level 9

# Pack all workspace packages
vite pm pack -r

# Pack specific workspace packages
vite pm pack --filter app --filter web
```

**Options:**

- `-r, --recursive`: Pack all workspace packages
- `--filter <pattern>`: Filter packages to pack (can be used multiple times)
- `--out <path>`: Customizes the output path for the tarball. Use `%s` and `%v` to include the package name and version (pnpm and yarn@2+ only), e.g., `%s.tgz` or `some-dir/%s-%v.tgz`. By default, the tarball is saved in the current working directory with the name `<package-name>-<version>.tgz`
- `--pack-destination <dir>`: Directory where the tarball will be saved (pnpm and npm only)
- `--pack-gzip-level <level>`: Gzip compression level 0-9 (pnpm only)
- `--json`: Output in JSON format

#### 3. vite pm list / vite pm ls

List installed packages.

```bash
vite pm list [PATTERN] [OPTIONS]
vite pm ls [PATTERN] [OPTIONS]
```

**Examples:**

```bash
# List all direct dependencies
vite pm list

# List dependencies matching pattern
vite pm list react

# Show dependency tree
vite pm list --depth 2

# JSON output
vite pm list --json

# List in specific workspace
vite pm list --filter app

# List across all workspaces
vite pm list -r

# List only production dependencies
vite pm list --prod

# List globally installed packages
vite pm list -g
```

**Options:**

- `--depth <n>`: Maximum depth of dependency tree
- `--json`: JSON output format
- `--long`: Extended information
- `--parseable`: Parseable output
- `--prod`: Only production dependencies
- `--dev`: Only dev dependencies
- `-r, --recursive`: List across all workspaces
- `--filter <pattern>`: Filter by workspace (can be used multiple times)
- `-g, --global`: List global packages

#### 4. vite pm view / vite pm info / vite pm show

View package information from the registry.

```bash
vite pm view [<package-spec>] [<field>[.subfield]...] [OPTIONS]
vite pm info [<package-spec>] [<field>[.subfield]...] [OPTIONS]
vite pm show [<package-spec>] [<field>[.subfield]...] [OPTIONS]
```

**Examples:**

```bash
# View package information
vite pm view react

# View specific version
vite pm view react@18.3.1

# View specific field
vite pm view react version
vite pm view react dist.tarball

# View nested field
vite pm view react dependencies.prop-types

# JSON output
vite pm view react --json

# Use aliases
vite pm info lodash
vite pm show express
```

**Options:**

- `--json`: JSON output format

#### 5. vite pm publish

Publish package to the registry.

```bash
vite pm publish [TARBALL|FOLDER] [OPTIONS]
```

**Examples:**

```bash
# Publish current package
vite pm publish

# Publish specific tarball
vite pm publish package.tgz

# Dry run
vite pm publish --dry-run

# Set tag
vite pm publish --tag beta

# Set access level
vite pm publish --access public

# Recursive publish in monorepo
vite pm publish -r

# Publish with filter
vite pm publish --filter app
```

**Options:**

- `--dry-run`: Preview without actually publishing
- `--tag <tag>`: Publish with specific tag (default: latest)
- `--access <public|restricted>`: Access level
- `--no-git-checks`: Skip git checks
- `--force`: Force publish even if already exists
- `-r, --recursive`: Publish all workspace packages
- `--filter <pattern>`: Filter workspaces (pnpm)
- `--workspace <name>`: Specific workspace (npm)

#### 6. vite pm owner

Manage package owners.

```bash
vite pm owner <subcommand> <package>
```

**Subcommands:**

- `list <package>`: List package owners
- `add <user> <package>`: Add owner
- `rm <user> <package>`: Remove owner

**Examples:**

```bash
# List package owners
vite pm owner list my-package

# Add owner
vite pm owner add username my-package

# Remove owner
vite pm owner rm username my-package
```

#### 7. vite pm cache

Manage package cache.

```bash
vite pm cache [SUBCOMMAND] [OPTIONS]
```

**Subcommands:**

- `dir` / `path`: Show cache directory
- `clean` / `clear`: Clean cache
- `verify`: Verify cache integrity (npm)
- `list`: List cached packages (pnpm)

**Examples:**

```bash
# Show cache directory
vite pm cache dir
vite pm cache path

# Clean cache
vite pm cache clean
vite pm cache clear

# Force clean (npm)
vite pm cache clean --force

# Verify cache (npm)
vite pm cache verify

# List cached packages (pnpm)
vite pm cache list
```

**Options:**

- `--force`: Force cache clean (npm)

#### 8. vite pm config / vite pm c

Manage package manager configuration.

```bash
vite pm config <subcommand> [key] [value] [OPTIONS]
vite pm c <subcommand> [key] [value] [OPTIONS]
```

**Subcommands:**

- `list`: List all configuration
- `get <key>`: Get configuration value
- `set <key> <value>`: Set configuration value
- `delete <key>`: Delete configuration key

**Examples:**

```bash
# List all config
vite pm config list

# Get config value
vite pm config get registry

# Set config value
vite pm config set registry https://registry.npmjs.org

# Set with JSON format (pnpm/yarn@2+)
vite pm config set registry https://registry.npmjs.org --json

# Set global config
vite pm config set registry https://registry.npmjs.org --global

# Set global config with location parameter (alternative)
vite pm config set registry https://registry.npmjs.org --location global

# Delete config key
vite pm config delete registry

# Use alias
vite pm c get registry
```

**Options:**

- `--json`: JSON output format (pnpm/yarn@2+)
- `-g, --global`: Use global config (shorthand for `--location global`)
- `--location <location>`: Config location: project (default) or global

### Command Mapping

#### Prune Command

**pnpm references:**

- https://pnpm.io/cli/prune

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-prune

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/prune
- The prune command isn't necessary. yarn install will prune extraneous packages.

| Vite+ Flag      | pnpm            | npm               | yarn | Description                 |
| --------------- | --------------- | ----------------- | ---- | --------------------------- |
| `vite pm prune` | `pnpm prune`    | `npm prune`       | N/A  | Remove unnecessary packages |
| `--prod`        | `--prod`        | `--omit=dev`      | N/A  | Remove devDependencies      |
| `--no-optional` | `--no-optional` | `--omit=optional` | N/A  | Remove optional deps        |

**Note:**

- npm supports prune with `--omit=dev` (for prod) and `--omit=optional` (for no-optional)
- yarn doesn't have a prune command (automatic during install)

#### Pack Command

**pnpm references:**

- https://pnpm.io/cli/pack

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-pack

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/pack
- https://yarnpkg.com/cli/pack
- https://yarnpkg.com/cli/workspaces/foreach (for yarn@2+ recursive packing)

| Vite+ Flag                  | pnpm                 | npm                  | yarn@1       | yarn@2+                                       | Description                       |
| --------------------------- | -------------------- | -------------------- | ------------ | --------------------------------------------- | --------------------------------- |
| `vite pm pack`              | `pnpm pack`          | `npm pack`           | `yarn pack`  | `yarn pack`                                   | Create package tarball            |
| `-r, --recursive`           | `--recursive`        | `--workspaces`       | N/A          | `workspaces foreach --all pack`               | Pack all workspace packages       |
| `--filter <pattern>`        | `--filter`           | `--workspace`        | N/A          | `workspaces foreach --include <pattern> pack` | Filter packages to pack           |
| `--out <path>`              | `--out`              | N/A                  | `--filename` | `--out`                                       | Output file path (supports %s/%v) |
| `--pack-destination <dir>`  | `--pack-destination` | `--pack-destination` | N/A          | N/A                                           | Output directory                  |
| `--pack-gzip-level <level>` | `--pack-gzip-level`  | N/A                  | N/A          | N/A                                           | Gzip compression level (0-9)      |
| `--json`                    | `--json`             | `--json`             | `--json`     | `--json`                                      | JSON output                       |

**Note:**

- `-r, --recursive`: Pack all workspace packages
  - pnpm uses `--recursive`
  - npm uses `--workspaces`
  - yarn@1 does not support (prints warning and ignores)
  - yarn@2+ uses `yarn workspaces foreach --all pack`
- `--filter <pattern>`: Filter packages to pack (can be used multiple times)
  - pnpm uses `--filter <pattern>`
  - npm uses `--workspace <pattern>`
  - yarn@1 does not support (prints warning and ignores)
  - yarn@2+ always uses `yarn workspaces foreach --all --include <pattern> pack`
- `--out <path>`: Specifies the full output file path
  - pnpm and yarn@2+ support `%s` (package name) and `%v` (version) placeholders
  - yarn@1 uses `--filename` (does not support placeholders)
  - npm does not support this option
- `--pack-destination <dir>`: Specifies the output directory (file name auto-generated)
  - Supported by pnpm and npm
  - yarn does not support this option (prints warning and ignores)
- `--pack-gzip-level <level>`: Gzip compression level (0-9)
  - Only supported by pnpm
  - npm and yarn do not support this option (prints warning and ignores)

#### List Command

**pnpm references:**

- https://pnpm.io/cli/list

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-ls

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/list

| Vite+ Flag           | pnpm              | npm                             | yarn@1        | yarn@2+       | Description                                   |
| -------------------- | ----------------- | ------------------------------- | ------------- | ------------- | --------------------------------------------- |
| `vite pm list`       | `pnpm list`       | `npm list`                      | `yarn list`   | N/A           | List installed packages                       |
| `--depth <n>`        | `--depth <n>`     | `--depth <n>`                   | `--depth <n>` | N/A           | Limit tree depth                              |
| `--json`             | `--json`          | `--json`                        | `--json`      | N/A           | JSON output                                   |
| `--long`             | `--long`          | `--long`                        | N/A           | N/A           | Extended info                                 |
| `--parseable`        | `--parseable`     | `--parseable`                   | N/A           | N/A           | Parseable format                              |
| `-P, --prod`         | `--prod`          | `--include prod --include peer` | N/A           | N/A           | Production deps only                          |
| `-D, --dev`          | `--dev`           | `--include dev`                 | N/A           | N/A           | Dev deps only                                 |
| `--no-optional`      | `--no-optional`   | `--omit optional`               | N/A           | N/A           | Exclude optional deps                         |
| `--exclude-peers`    | `--exclude-peers` | `--omit peer`                   | N/A           | N/A           | Exclude peer deps                             |
| `--only-projects`    | `--only-projects` | N/A                             | N/A           | N/A           | Show only project packages (pnpm)             |
| `--find-by <name>`   | `--find-by`       | N/A                             | N/A           | N/A           | Use finder function from .pnpmfile.cjs (pnpm) |
| `-r, --recursive`    | `--recursive`     | `--workspaces`                  | N/A           | N/A           | List across workspaces                        |
| `--filter <pattern>` | `--filter`        | `--workspace`                   | N/A           | N/A           | Filter workspace                              |
| `-g, --global`       | `npm list -g`     | `npm list -g`                   | `npm list -g` | `npm list -g` | List global packages                          |

**Note:**

- yarn@2+ does not support the `list` command (command is ignored)
- `-r, --recursive`: List across all workspaces
  - pnpm uses `--recursive`
  - npm uses `--workspaces`
  - yarn@1 does not support (prints warning and ignores)
  - yarn@2+ does not support list command at all
- `--filter <pattern>`: Filter by workspace (can be used multiple times)
  - pnpm uses `--filter <pattern>` (comes before `list` command)
  - npm uses `--workspace <pattern>` (comes after `list` command)
  - yarn@1 does not support (prints warning and ignores)
  - yarn@2+ does not support list command at all
- `-P, --prod`: Show only production dependencies (and peer dependencies)
  - pnpm uses `--prod`
  - npm uses `--include prod --include peer`
  - yarn@1 does not support (prints warning and ignores)
  - yarn@2+ does not support list command at all
- `-D, --dev`: Show only dev dependencies
  - pnpm uses `--dev`
  - npm uses `--include dev`
  - yarn@1 does not support (prints warning and ignores)
  - yarn@2+ does not support list command at all
- `--no-optional`: Exclude optional dependencies
  - pnpm uses `--no-optional`
  - npm uses `--omit optional`
  - yarn@1 does not support (prints warning and ignores)
  - yarn@2+ does not support list command at all
- `--exclude-peers`: Exclude peer dependencies
  - pnpm uses `--exclude-peers`
  - npm uses `--omit peer`
  - yarn@1 does not support (prints warning and ignores)
  - yarn@2+ does not support list command at all
- `--only-projects`: Show only project packages (workspace packages only, no dependencies)
  - Only supported by pnpm
  - npm does not support (prints warning and ignores)
  - yarn@1 does not support (prints warning and ignores)
  - yarn@2+ does not support list command at all
- `--find-by <finder_name>`: Use a finder function defined in .pnpmfile.cjs to match dependencies by properties other than name
  - Only supported by pnpm (pnpm-specific feature)
  - npm does not support (prints warning and ignores)
  - yarn@1 does not support (prints warning and ignores)
  - yarn@2+ does not support list command at all
- `-g, --global`: List globally installed packages
  - All package managers delegate to `npm list -g` (since global installs use npm)
  - Uses npm regardless of the detected package manager

#### View Command

**pnpm references:**

- https://pnpm.io/cli/view (delegates to npm view)

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-view

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/info (delegates to npm view)
- https://yarnpkg.com/cli/npm/info (delegates to npm view)

| Vite+ Flag     | pnpm       | npm        | yarn@1     | yarn@2+    | Description       |
| -------------- | ---------- | ---------- | ---------- | ---------- | ----------------- |
| `vite pm view` | `npm view` | `npm view` | `npm view` | `npm view` | View package info |
| `--json`       | `--json`   | `--json`   | `--json`   | `--json`   | JSON output       |

**Note:**

- All package managers delegate to `npm view` for viewing package information
- pnpm and yarn both use npm's view/info functionality internally
- Aliases: `vite pm info` and `vite pm show` work the same as `vite pm view`

#### Publish Command

**pnpm references:**

- https://pnpm.io/cli/publish

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-publish

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/publish (delegates to npm publish)
- https://yarnpkg.com/cli/npm/publish (delegates to npm publish)

| Vite+ Flag                  | pnpm               | npm                | yarn@1             | yarn@2+            | Description                 |
| --------------------------- | ------------------ | ------------------ | ------------------ | ------------------ | --------------------------- |
| `vite pm publish`           | `pnpm publish`     | `npm publish`      | `npm publish`      | `npm publish`      | Publish package             |
| `--dry-run`                 | `--dry-run`        | `--dry-run`        | `--dry-run`        | `--dry-run`        | Preview without publishing  |
| `--tag <tag>`               | `--tag <tag>`      | `--tag <tag>`      | `--tag <tag>`      | `--tag <tag>`      | Publish tag                 |
| `--access <level>`          | `--access <level>` | `--access <level>` | `--access <level>` | `--access <level>` | Public/restricted           |
| `--otp <otp>`               | `--otp`            | `--otp`            | `--otp`            | `--otp`            | One-time password           |
| `--no-git-checks`           | `--no-git-checks`  | N/A                | N/A                | N/A                | Skip git checks (pnpm)      |
| `--publish-branch <branch>` | `--publish-branch` | N/A                | N/A                | N/A                | Set publish branch (pnpm)   |
| `--report-summary`          | `--report-summary` | N/A                | N/A                | N/A                | Save publish summary (pnpm) |
| `--force`                   | `--force`          | `--force`          | `--force`          | `--force`          | Force publish               |
| `--json`                    | `--json`           | N/A                | N/A                | N/A                | JSON output (pnpm)          |
| `-r, --recursive`           | `--recursive`      | `--workspaces`     | N/A                | N/A                | Publish workspaces          |
| `--filter <pattern>`        | `--filter`         | `--workspace`      | N/A                | N/A                | Filter workspace            |

**Note:**

- All yarn versions delegate to `npm publish` for publishing packages
- yarn@1 and yarn@2+ both use npm's publish functionality internally
- `-r, --recursive`: Publish all workspace packages
  - pnpm uses `--recursive`
  - npm uses `--workspaces`
  - yarn does not support (delegates to npm which doesn't support this in single publish mode)
- `--filter <pattern>`: Filter workspace packages to publish (can be used multiple times)
  - pnpm uses `--filter <pattern>` (comes before `publish` command)
  - npm uses `--workspace <pattern>` (comes after `publish` command)
  - yarn does not support (delegates to npm, can use --workspace)
- `--no-git-checks`: Skip git checks before publishing
  - Only supported by pnpm (pnpm-specific feature)
  - npm does not support (prints warning and ignores)
  - yarn does not support (delegates to npm which doesn't support it)
- `--publish-branch <branch>`: Set the branch name to publish from
  - Only supported by pnpm (pnpm-specific feature)
  - npm does not support (prints warning and ignores)
  - yarn does not support (delegates to npm which doesn't support it)
- `--report-summary`: Save publish summary to pnpm-publish-summary.json
  - Only supported by pnpm (pnpm-specific feature)
  - npm does not support (prints warning and ignores)
  - yarn does not support (delegates to npm which doesn't support it)
- `--json`: JSON output
  - Only supported by pnpm (pnpm-specific feature)
  - npm does not support (prints warning and ignores)
  - yarn does not support (delegates to npm which doesn't support it)
- pnpm-specific features: `--no-git-checks`, `--publish-branch`, `--report-summary`, `--json`

#### Owner Command

**pnpm references:**

- https://pnpm.io/cli/owner (delegates to npm owner)

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-owner

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/owner (delegates to npm owner)
- https://yarnpkg.com/cli/npm/owner (delegates to npm owner)

| Vite+ Flag                  | pnpm             | npm              | yarn@1           | yarn@2+          | Description         |
| --------------------------- | ---------------- | ---------------- | ---------------- | ---------------- | ------------------- |
| `vite pm owner list <pkg>`  | `npm owner list` | `npm owner list` | `npm owner list` | `npm owner list` | List package owners |
| `vite pm owner add <u> <p>` | `npm owner add`  | `npm owner add`  | `npm owner add`  | `npm owner add`  | Add owner           |
| `vite pm owner rm <u> <p>`  | `npm owner rm`   | `npm owner rm`   | `npm owner rm`   | `npm owner rm`   | Remove owner        |
| `--otp <otp>`               | `--otp`          | `--otp`          | `--otp`          | `--otp`          | One-time password   |

**Note:**

- All package managers delegate to `npm owner` for managing package ownership
- pnpm and yarn both use npm's owner functionality internally
- Alias: `vite pm author` works the same as `vite pm owner`

#### Cache Command

**pnpm references:**

- https://pnpm.io/cli/store

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-cache

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/cache
- https://yarnpkg.com/cli/cache

| Vite+ Flag            | pnpm               | npm                    | yarn@1             | yarn@2+                       | Description          |
| --------------------- | ------------------ | ---------------------- | ------------------ | ----------------------------- | -------------------- |
| `vite pm cache dir`   | `pnpm store path`  | `npm config get cache` | `yarn cache dir`   | `yarn config get cacheFolder` | Show cache directory |
| `vite pm cache path`  | Alias for `dir`    | Alias for `dir`        | Alias for `dir`    | Alias for `dir`               | Alias for dir        |
| `vite pm cache clean` | `pnpm store prune` | `npm cache clean`      | `yarn cache clean` | `yarn cache clean`            | Clean cache          |

**Note:**

- `cache dir` / `cache path`: Show cache directory location
  - pnpm uses `pnpm store path`
  - npm uses `npm config get cache` (not `npm cache dir` which doesn't exist in modern npm)
  - yarn@1 uses `yarn cache dir`
  - yarn@2+ uses `yarn config get cacheFolder`
- Subcommand aliases: `path` is an alias for `dir`

#### Config Command

**pnpm references:**

- https://pnpm.io/cli/config

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-config

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/config
- https://yarnpkg.com/cli/config

| Vite+ Flag                    | pnpm                 | npm                 | yarn@1               | yarn@2+                     | Description        |
| ----------------------------- | -------------------- | ------------------- | -------------------- | --------------------------- | ------------------ |
| `vite pm config list`         | `pnpm config list`   | `npm config list`   | `yarn config list`   | `yarn config`               | List configuration |
| `vite pm config get <key>`    | `pnpm config get`    | `npm config get`    | `yarn config get`    | `yarn config get`           | Get config value   |
| `vite pm config set <k> <v>`  | `pnpm config set`    | `npm config set`    | `yarn config set`    | `yarn config set`           | Set config value   |
| `vite pm config delete <key>` | `pnpm config delete` | `npm config delete` | `yarn config delete` | `yarn config unset`         | Delete config key  |
| `--json`                      | `--json`             | `--json`            | `--json`             | `--json`                    | JSON output        |
| `-g, --global`                | `--global`           | `--global`          | `--global`           | `--home`                    | Global config      |
| `--location <location>`       | `--location`         | `--location`        | N/A                  | Maps to `--home` for global | Config location    |

**Note:**

- Alias: `vite pm c` works the same as `vite pm config`
- `-g, --global`: Shorthand for setting global configuration
  - pnpm uses `--global`
  - npm uses `--global`
  - yarn@1 uses `--global`
  - yarn@2+ uses `--home`
  - Equivalent to `--location global`
- `--location`: Specify config location (default: global)
  - pnpm supports: `project`, `global` (default)
  - npm supports: `project`, `user`, `global` (default), etc.
  - yarn@1 does not support (prints warning and ignores, uses global by default)
  - yarn@2+ maps `global` to `--home` flag; `project` is default when no flag specified
- `--json`: JSON output format
  - Supported by all package managers for output formatting (list/get commands)
  - For `set` command with JSON value: pnpm, npm, yarn@2+ support; yarn@1 does not support

### Implementation Architecture

#### 1. Command Structure

**File**: `crates/vite_task/src/lib.rs`

Add new command group:

```rust
#[derive(Subcommand, Debug)]
pub enum Commands {
    // ... existing commands

    /// Package manager utilities
    #[command(disable_help_flag = true, subcommand)]
    Pm(PmCommands),
}

#[derive(Subcommand, Debug)]
pub enum PmCommands {
    /// Remove unnecessary packages
    Prune {
        /// Remove devDependencies
        #[arg(long)]
        prod: bool,

        /// Remove optional dependencies
        #[arg(long)]
        no_optional: bool,

        /// Arguments to pass to package manager
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Create a tarball of the package
    Pack {
        /// Preview without creating tarball
        #[arg(long)]
        dry_run: bool,

        /// Output directory for tarball
        #[arg(long)]
        pack_destination: Option<String>,

        /// Gzip compression level (0-9)
        #[arg(long)]
        pack_gzip_level: Option<u8>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Arguments to pass to package manager
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// List installed packages
    #[command(alias = "ls")]
    List {
        /// Package pattern to filter
        pattern: Option<String>,

        /// Maximum depth of dependency tree
        #[arg(long)]
        depth: Option<u32>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Show extended information
        #[arg(long)]
        long: bool,

        /// Parseable output format
        #[arg(long)]
        parseable: bool,

        /// Only production dependencies
        #[arg(long)]
        prod: bool,

        /// Only dev dependencies
        #[arg(long)]
        dev: bool,

        /// List across all workspaces
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Filter packages in monorepo (pnpm)
        #[arg(long)]
        filter: Vec<String>,

        /// Target specific workspace (npm)
        #[arg(long)]
        workspace: Vec<String>,

        /// List global packages
        #[arg(short = 'g', long)]
        global: bool,

        /// Arguments to pass to package manager
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// View package information from registry
    View {
        /// Package name with optional version
        package: String,

        /// Specific field to view
        field: Option<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Arguments to pass to package manager
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Publish package to registry
    Publish {
        /// Tarball or folder to publish
        target: Option<String>,

        /// Preview without publishing
        #[arg(long)]
        dry_run: bool,

        /// Publish tag (default: latest)
        #[arg(long)]
        tag: Option<String>,

        /// Access level (public/restricted)
        #[arg(long)]
        access: Option<String>,

        /// Skip git checks (pnpm)
        #[arg(long)]
        no_git_checks: bool,

        /// Force publish
        #[arg(long)]
        force: bool,

        /// Publish all workspace packages
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Filter packages in monorepo (pnpm)
        #[arg(long)]
        filter: Vec<String>,

        /// Target specific workspace (npm)
        #[arg(long)]
        workspace: Vec<String>,

        /// Arguments to pass to package manager
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Manage package owners
    Owner {
        /// Subcommand: list, add, rm
        #[command(subcommand)]
        command: OwnerCommands,
    },

    /// Manage package cache
    Cache {
        /// Subcommand: dir, path, clean, clear, verify, list
        subcommand: Option<String>,

        /// Force clean (npm)
        #[arg(long)]
        force: bool,

        /// Arguments to pass to package manager
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Manage package manager configuration
    Config {
        /// Subcommand: list, get, set, delete
        subcommand: Option<String>,

        /// Config key
        key: Option<String>,

        /// Config value (for set)
        value: Option<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Use global config
        #[arg(long)]
        global: bool,

        /// Arguments to pass to package manager
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum OwnerCommands {
    /// List package owners
    List {
        /// Package name
        package: String,
    },

    /// Add package owner
    Add {
        /// Username
        user: String,
        /// Package name
        package: String,
    },

    /// Remove package owner
    Rm {
        /// Username
        user: String,
        /// Package name
        package: String,
    },
}
```

#### 2. Package Manager Adapter

**File**: `crates/vite_package_manager/src/commands/pm.rs` (new file)

```rust
use std::{collections::HashMap, process::ExitStatus};

use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env, run_command,
};

impl PackageManager {
    /// Run a pm subcommand with pass-through arguments.
    #[must_use]
    pub async fn run_pm_command(
        &self,
        subcommand: &str,
        args: &[String],
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_pm_command(subcommand, args);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve pm command with minimal processing.
    /// Most arguments are passed through directly to the package manager.
    #[must_use]
    pub fn resolve_pm_command(&self, subcommand: &str, args: &[String]) -> ResolveCommandResult {
        let bin_name: String;
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut cmd_args: Vec<String> = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                bin_name = "pnpm".into();

                // Map vite pm commands to pnpm commands
                match subcommand {
                    "prune" => cmd_args.push("prune".into()),
                    "pack" => cmd_args.push("pack".into()),
                    "list" | "ls" => cmd_args.push("list".into()),
                    "view" => cmd_args.push("view".into()),
                    "publish" => cmd_args.push("publish".into()),
                    "owner" => cmd_args.push("owner".into()),
                    "cache" => {
                        // Map cache subcommands
                        if !args.is_empty() {
                            match args[0].as_str() {
                                "dir" | "path" => cmd_args.push("store".into()),
                                "clean" | "clear" => {
                                    cmd_args.push("store".into());
                                    cmd_args.push("prune".into());
                                    return ResolveCommandResult { bin_path: bin_name, args: cmd_args, envs };
                                }
                                "list" => {
                                    cmd_args.push("store".into());
                                    cmd_args.push("list".into());
                                    return ResolveCommandResult { bin_path: bin_name, args: cmd_args, envs };
                                }
                                _ => cmd_args.push("store".into()),
                            }
                        } else {
                            cmd_args.push("store".into());
                            cmd_args.push("path".into());
                            return ResolveCommandResult { bin_path: bin_name, args: cmd_args, envs };
                        }
                    }
                    "config" => cmd_args.push("config".into()),
                    _ => cmd_args.push(subcommand.into()),
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();

                match subcommand {
                    "prune" => {
                        eprintln!("Warning: npm removed 'prune' command in v6. Use 'vite install --prod' instead.");
                        return ResolveCommandResult {
                            bin_path: "echo".into(),
                            args: vec!["npm prune is deprecated".into()],
                            envs,
                        };
                    }
                    "pack" => cmd_args.push("pack".into()),
                    "list" | "ls" => cmd_args.push("list".into()),
                    "view" => cmd_args.push("view".into()),
                    "publish" => cmd_args.push("publish".into()),
                    "owner" => cmd_args.push("owner".into()),
                    "cache" => {
                        cmd_args.push("cache".into());
                        if !args.is_empty() {
                            match args[0].as_str() {
                                "path" => {
                                    // npm uses 'dir' not 'path'
                                    cmd_args.push("dir".into());
                                    return ResolveCommandResult { bin_path: bin_name, args: cmd_args, envs };
                                }
                                "clear" => {
                                    // npm uses 'clean' not 'clear'
                                    cmd_args.push("clean".into());
                                }
                                _ => {}
                            }
                        }
                    }
                    "config" => cmd_args.push("config".into()),
                    _ => cmd_args.push(subcommand.into()),
                }
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();

                match subcommand {
                    "prune" => {
                        if self.version.starts_with("1.") {
                            cmd_args.push("prune".into());
                        } else {
                            eprintln!("Warning: yarn@2+ does not have 'prune' command");
                            return ResolveCommandResult {
                                bin_path: "echo".into(),
                                args: vec!["yarn@2+ does not support prune".into()],
                                envs,
                            };
                        }
                    }
                    "pack" => cmd_args.push("pack".into()),
                    "list" | "ls" => cmd_args.push("list".into()),
                    "view" => {
                        // yarn uses 'info' instead of 'view'
                        cmd_args.push("info".into());
                    }
                    "publish" => {
                        if self.version.starts_with("1.") {
                            cmd_args.push("publish".into());
                        } else {
                            cmd_args.push("npm".into());
                            cmd_args.push("publish".into());
                        }
                    }
                    "owner" => {
                        if self.version.starts_with("1.") {
                            cmd_args.push("owner".into());
                        } else {
                            cmd_args.push("npm".into());
                            cmd_args.push("owner".into());
                        }
                    }
                    "cache" => {
                        cmd_args.push("cache".into());
                        if !args.is_empty() {
                            match args[0].as_str() {
                                "path" => {
                                    // yarn uses 'dir' not 'path'
                                    cmd_args.push("dir".into());
                                    return ResolveCommandResult { bin_path: bin_name, args: cmd_args, envs };
                                }
                                "clear" => {
                                    // yarn uses 'clean' not 'clear'
                                    cmd_args.push("clean".into());
                                }
                                "verify" => {
                                    eprintln!("Warning: yarn does not support 'cache verify'");
                                    return ResolveCommandResult {
                                        bin_path: "echo".into(),
                                        args: vec!["yarn does not support cache verify".into()],
                                        envs,
                                    };
                                }
                                _ => {}
                            }
                        }
                    }
                    "config" => {
                        cmd_args.push("config".into());
                        // yarn@2+ uses different config commands
                        if !self.version.starts_with("1.") && !args.is_empty() && args[0] == "delete" {
                            cmd_args.push("unset".into());
                            cmd_args.extend_from_slice(&args[1..]);
                            return ResolveCommandResult { bin_path: bin_name, args: cmd_args, envs };
                        }
                    }
                    _ => cmd_args.push(subcommand.into()),
                }
            }
        }

        // Pass through all remaining arguments
        cmd_args.extend_from_slice(args);

        ResolveCommandResult { bin_path: bin_name, args: cmd_args, envs }
    }
}
```

**File**: `crates/vite_package_manager/src/commands/mod.rs`

Update to include pm module:

```rust
pub mod add;
mod install;
pub mod remove;
pub mod update;
pub mod link;
pub mod unlink;
pub mod dedupe;
pub mod why;
pub mod outdated;
pub mod pm;  // Add this line
```

#### 3. PM Command Implementation

**File**: `crates/vite_task/src/pm.rs` (new file)

```rust
use vite_error::Error;
use vite_path::AbsolutePathBuf;
use vite_package_manager::PackageManager;
use vite_workspace::Workspace;

pub struct PmCommand {
    workspace_root: AbsolutePathBuf,
}

impl PmCommand {
    pub fn new(workspace_root: AbsolutePathBuf) -> Self {
        Self { workspace_root }
    }

    pub async fn execute(
        self,
        subcommand: String,
        args: Vec<String>,
    ) -> Result<ExecutionSummary, Error> {
        let package_manager = PackageManager::builder(&self.workspace_root).build().await?;
        let workspace = Workspace::partial_load(self.workspace_root)?;

        let exit_status = package_manager
            .run_pm_command(&subcommand, &args, &workspace.root)
            .await?;

        if !exit_status.success() {
            return Err(Error::CommandFailed {
                command: format!("pm {}", subcommand),
                exit_code: exit_status.code(),
            });
        }

        workspace.unload().await?;

        Ok(ExecutionSummary::default())
    }
}
```

## Design Decisions

### 1. Pass-Through Architecture

**Decision**: Use minimal processing and pass most arguments directly to package managers.

**Rationale**:

- Package managers have many flags and options that change frequently
- Trying to map every option is maintenance-intensive and error-prone
- Pass-through allows users to use any package manager feature
- Vite+ provides the abstraction of which PM to use, not feature mapping
- Users can reference their package manager docs for advanced options

### 2. Command Name Mapping

**Decision**: Map common command name differences (e.g., `view` → `info` for yarn).

**Rationale**:

- Some commands have different names across package managers
- Basic name mapping provides better UX
- Keeps common cases simple
- Advanced users can still use native commands directly

### 3. Cache Command Special Handling

**Decision**: Provide subcommands for cache (dir, clean, verify, list).

**Rationale**:

- Cache commands have very different syntax across package managers
- pnpm uses `store`, npm uses `cache`, yarn uses `cache`
- Unified interface makes cache management easier
- Common operation that benefits from abstraction

### 4. No Caching

**Decision**: Don't cache any pm command results.

**Rationale**:

- PM utilities query current state or modify configuration
- Caching would provide stale data
- Operations are fast enough without caching
- Real-time data is expected

### 5. Deprecation Warnings

**Decision**: Warn users when commands aren't available in their package manager.

**Rationale**:

- npm removed `prune` in v6
- yarn@2+ doesn't have `prune`
- Helpful to educate users about alternatives
- Better than silent failure

### 6. Subcommand Groups

**Decision**: Group related commands under `pm` rather than top-level commands.

**Rationale**:

- Keeps vite+ CLI namespace clean
- Clear categorization (pm utilities vs task running)
- Matches Bun's design pattern
- Extensible for future utilities

## Error Handling

### No Package Manager Detected

```bash
$ vite pm list
Error: No package manager detected
Please run one of:
  - vite install (to set up package manager)
  - Add packageManager field to package.json
```

### Unsupported Command

```bash
$ vite pm prune
Detected package manager: yarn@4.0.0
Warning: yarn does not have 'prune' command. yarn install will prune extraneous packages automatically.
$ echo $?
0
```

### Command Failed

```bash
$ vite pm publish
Detected package manager: pnpm@10.15.0
Running: pnpm publish
Error: You must be logged in to publish packages
Exit code: 1
```

## User Experience

### Prune Packages

```bash
$ vite pm prune
Detected package manager: pnpm@10.15.0
Running: pnpm prune
Packages: -12

$ vite pm prune --prod
Detected package manager: npm@11.0.0
Running: npm prune --omit=dev
removed 45 packages
```

### Cache Management

```bash
$ vite pm cache dir
Detected package manager: pnpm@10.15.0
Running: pnpm store path
/Users/user/Library/pnpm/store

$ vite pm cache clean
Detected package manager: pnpm@10.15.0
Running: pnpm store prune
Removed 145 packages
```

### List Packages

```bash
$ vite pm list --depth 0
Detected package manager: pnpm@10.15.0
Running: pnpm list --depth 0

my-app@1.0.0
├── react@18.3.1
├── react-dom@18.3.1
└── lodash@4.17.21
```

### View Package

```bash
$ vite pm view react version
Detected package manager: npm@11.0.0
Running: npm view react version
18.3.1
```

### Publish Package

```bash
$ vite pm publish --dry-run
Detected package manager: pnpm@10.15.0
Running: pnpm publish --dry-run

npm notice
npm notice package: my-package@1.0.0
npm notice === Tarball Contents ===
npm notice 1.2kB package.json
npm notice 2.3kB README.md
npm notice === Tarball Details ===
npm notice name:          my-package
npm notice version:       1.0.0
```

### Configuration

```bash
$ vite pm config get registry
Detected package manager: pnpm@10.15.0
Running: pnpm config get registry
https://registry.npmjs.org

$ vite pm config set registry https://custom-registry.com
Detected package manager: pnpm@10.15.0
Running: pnpm config set registry https://custom-registry.com
```

## Alternative Designs Considered

### Alternative 1: Individual Top-Level Commands

```bash
vite cache dir
vite publish
vite pack
```

**Rejected because**:

- Clutters top-level namespace
- Mixes task running with PM utilities
- Less clear categorization
- Harder to discover related commands

### Alternative 2: Full Flag Mapping

```bash
# Try to map all package manager flags
vite pm list --production  # Map to --prod (pnpm), --production (npm)
```

**Rejected because**:

- Maintenance burden as PMs add/change flags
- Incomplete mapping would be confusing
- Pass-through is more flexible
- Users can refer to PM docs for advanced usage

### Alternative 3: Single Pass-Through Command

```bash
vite pm -- pnpm store path
vite pm -- npm cache dir
```

**Rejected because**:

- Loses abstraction benefit
- User must know package manager
- No command name translation
- Defeats purpose of unified interface

## Implementation Plan

### Phase 1: Core Infrastructure

1. Add `Pm` command group to `Commands` enum
2. Create `pm.rs` module in vite_package_manager
3. Implement basic pass-through for each subcommand
4. Add command name mapping (view → info, etc.)

### Phase 2: Subcommands

1. Implement `prune` with deprecation warnings
2. Implement `pack` with options
3. Implement `list/ls` with filtering
4. Implement `view` with field selection
5. Implement `publish` with workspace support
6. Implement `owner` subcommands
7. Implement `cache` with subcommands
8. Implement `config` with subcommands

### Phase 3: Testing

1. Unit tests for command resolution
2. Test pass-through arguments
3. Test command name mapping
4. Test deprecation warnings
5. Integration tests with mock package managers
6. Test workspace operations

### Phase 4: Documentation

1. Update CLI documentation
2. Add examples for each subcommand
3. Document package manager compatibility
4. Add troubleshooting guide

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_pnpm_cache_dir() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let result = pm.resolve_pm_command("cache", &["dir".to_string()]);
    assert_eq!(result.args, vec!["store", "path"]);
}

#[test]
fn test_npm_cache_dir() {
    let pm = PackageManager::mock(PackageManagerType::Npm);
    let result = pm.resolve_pm_command("cache", &["dir".to_string()]);
    assert_eq!(result.args, vec!["cache", "dir"]);
}

#[test]
fn test_yarn_view_maps_to_info() {
    let pm = PackageManager::mock(PackageManagerType::Yarn);
    let result = pm.resolve_pm_command("view", &["react".to_string()]);
    assert_eq!(result.args, vec!["info", "react"]);
}

#[test]
fn test_pass_through_args() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let result = pm.resolve_pm_command("list", &["--depth".to_string(), "0".to_string()]);
    assert_eq!(result.args, vec!["list", "--depth", "0"]);
}
```

## CLI Help Output

```bash
$ vite pm --help
Package manager utilities

Usage: vite pm <COMMAND>

Commands:
  prune    Remove unnecessary packages
  pack     Create a tarball of the package
  list     List installed packages (alias: ls)
  view     View package information from registry
  publish  Publish package to registry
  owner    Manage package owners
  cache    Manage package cache
  config   Manage package manager configuration
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help

$ vite pm cache --help
Manage package cache

Usage: vite pm cache [SUBCOMMAND] [OPTIONS]

Subcommands:
  dir      Show cache directory (alias: path)
  path     Alias for dir
  clean    Clean cache (alias: clear)
  clear    Alias for clean
  verify   Verify cache integrity (npm only)
  list     List cached packages (pnpm only)

Options:
  --force              Force cache clean (npm only)
  -h, --help           Print help

Examples:
  vite pm cache dir              # Show cache directory
  vite pm cache clean            # Clean cache
  vite pm cache clean --force    # Force clean (npm)
  vite pm cache verify           # Verify cache (npm)
  vite pm cache list             # List cached packages (pnpm)
```

## Package Manager Compatibility

| Subcommand | pnpm      | npm     | yarn@1    | yarn@2+         | Notes                                   |
| ---------- | --------- | ------- | --------- | --------------- | --------------------------------------- |
| prune      | ✅ Full   | ✅ Full | ❌ N/A    | ❌ N/A          | npm uses --omit flags, yarn auto-prunes |
| pack       | ✅ Full   | ✅ Full | ✅ Full   | ✅ Full         | All supported                           |
| list/ls    | ✅ Full   | ✅ Full | ⚠️ Limited | ❌ N/A          | yarn@1 no -r, yarn@2+ not supported     |
| view       | ✅ Full   | ✅ Full | ⚠️ `info`  | ⚠️ `info`        | yarn uses different name                |
| publish    | ✅ Full   | ✅ Full | ✅ Full   | ⚠️ `npm publish` | yarn@2+ uses npm plugin                 |
| owner      | ✅ Full   | ✅ Full | ✅ Full   | ⚠️ `npm owner`   | yarn@2+ uses npm plugin                 |
| cache      | ⚠️ `store` | ✅ Full | ✅ Full   | ✅ Full         | pnpm uses different command             |
| config     | ✅ Full   | ✅ Full | ✅ Full   | ⚠️ Different     | yarn@2+ has different API               |

## Future Enhancements

### 1. Interactive Cache Management

```bash
vite pm cache --interactive
# Shows cache size, allows selective cleaning
```

### 2. Publish Dry-Run Summary

```bash
vite pm publish --dry-run --summary
# Shows what would be published with sizes
```

### 3. Config Validation

```bash
vite pm config validate
# Checks configuration for issues
```

### 4. Owner Management UI

```bash
vite pm owner --interactive my-package
# Interactive UI for adding/removing owners
```

### 5. Cache Analytics

```bash
vite pm cache stats
# Shows cache usage statistics, size breakdown
```

## Security Considerations

1. **Publish Safety**: Dry-run option allows preview before publishing
2. **Config Isolation**: Respects package manager's configuration hierarchy
3. **Owner Management**: Delegates to package manager's authentication
4. **Cache Integrity**: Verify option (npm) checks for corruption
5. **Pass-Through Safety**: Arguments are passed through shell-escaped

## Backward Compatibility

This is a new feature with no breaking changes:

- Existing commands unaffected
- New command group is additive
- No changes to task configuration
- No changes to caching behavior

## Real-World Usage Examples

### Cache Management in CI

```yaml
# Clean cache before build
- run: vite pm cache clean --force

# Show cache location for debugging
- run: vite pm cache dir
```

### Publishing Workflow

```bash
# Build packages
vite build -r

# Dry run to verify
vite pm publish --dry-run -r

# Publish with beta tag
vite pm publish --tag beta -r

# Publish only specific packages
vite pm publish --filter app
```

### Configuration Management

```bash
# Set custom registry
vite pm config set registry https://custom-registry.com

# Verify configuration
vite pm config get registry

# List all configuration
vite pm config list
```

### Dependency Auditing

```bash
# List dependencies to JSON file
vite pm list --json > deps.json

# List production dependencies
vite pm list --prod

# List specific workspace
vite pm list --filter app
```

## Conclusion

This RFC proposes adding `vite pm` command group to provide unified access to package manager utilities across pnpm/npm/yarn. The design:

- ✅ Pass-through architecture for maximum flexibility
- ✅ Command name translation for common operations
- ✅ Unified cache management interface
- ✅ Support for all major package managers
- ✅ Workspace-aware operations
- ✅ Deprecation warnings for removed commands
- ✅ Extensible for future enhancements
- ✅ Simple implementation leveraging existing infrastructure
- ✅ Matches Bun's pm command design pattern

The implementation follows the same patterns as other package management commands while providing direct access to package manager utilities that developers need for publishing, cache management, configuration, and more.
