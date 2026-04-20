# RFC: Vite+ Package Manager Utilities Command Group

## Summary

Add `vp pm` command group that provides a set of utilities for working with package managers. The `pm` command group offers direct access to package manager utilities like cache management, package publishing, configuration, and more. These are pass-through commands that delegate to the detected package manager (pnpm/npm/yarn/bun) with minimal processing, providing a unified interface across different package managers.

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
4. **Missing Abstraction**: While Vite+ provides abstractions for install/add/remove/update, it lacks utilities for cache, publish, config, etc.

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
vp pm cache                         # Show cache directory
vp pm cache clean                   # Clean cache
vp pm list                          # List installed packages
vp pm config get registry           # Get config value
vp pm publish                       # Publish package
vp pm pack                          # Create package tarball
vp pm prune                         # Remove unnecessary packages
vp pm owner list <pkg>              # List package owners
vp pm view <pkg>                    # View package information
```

## Proposed Solution

### Command Syntax

```bash
vp pm <subcommand> [OPTIONS] [ARGS...]
```

**Subcommands:**

1. **prune**: Remove unnecessary packages
2. **pack**: Create a tarball of the package
3. **list** (alias: **ls**): List installed packages
4. **view**: View package information from the registry
5. **publish**: Publish package to registry
6. **owner**: Manage package owners
7. **cache**: Manage package cache
8. **config**: Manage package manager configuration
9. **login**: Log in to the registry
10. **logout**: Log out from the registry
11. **whoami**: Show the currently logged-in user
12. **token**: Manage registry authentication tokens
13. **audit**: Run a security audit on installed packages
14. **dist-tag**: Manage distribution tags on packages
15. **deprecate**: Deprecate a version of a package
16. **search**: Search the registry for packages
17. **rebuild**: Rebuild native addons
18. **fund**: Show funding information for installed packages
19. **ping**: Ping the registry

### Subcommand Details

#### 1. vp pm prune

Remove unnecessary packages from node_modules.

```bash
vp pm prune [OPTIONS]
```

**Examples:**

```bash
# Remove all extraneous packages
vp pm prune

# Remove devDependencies (production only)
vp pm prune --prod

# Remove optional dependencies
vp pm prune --no-optional
```

**Options:**

- `--prod`: Remove devDependencies
- `--no-optional`: Remove optional dependencies

#### 2. vp pm pack

Create a tarball archive of the package.

```bash
vp pm pack [OPTIONS]
```

**Examples:**

```bash
# Create tarball in current directory
vp pm pack

# Specify output file path
vp pm pack --out ./dist/package.tgz

# Use placeholders for package name and version (pnpm/yarn@2+ only)
vp pm pack --out ./dist/%s-%v.tgz

# Specify output directory
vp pm pack --pack-destination ./dist

# Custom gzip compression level
vp pm pack --pack-gzip-level 9

# Pack all workspace packages
vp pm pack -r

# Pack specific workspace packages
vp pm pack --filter app --filter web
```

**Options:**

- `-r, --recursive`: Pack all workspace packages
- `--filter <pattern>`: Filter packages to pack (can be used multiple times)
- `--out <path>`: Customizes the output path for the tarball. Use `%s` and `%v` to include the package name and version (pnpm and yarn@2+ only), e.g., `%s.tgz` or `some-dir/%s-%v.tgz`. By default, the tarball is saved in the current working directory with the name `<package-name>-<version>.tgz`
- `--pack-destination <dir>`: Directory where the tarball will be saved (pnpm and npm only)
- `--pack-gzip-level <level>`: Gzip compression level 0-9 (pnpm only)
- `--json`: Output in JSON format

#### 3. vp pm list / vp pm ls

List installed packages.

```bash
vp pm list [PATTERN] [OPTIONS]
vp pm ls [PATTERN] [OPTIONS]
```

**Examples:**

```bash
# List all direct dependencies
vp pm list

# List dependencies matching pattern
vp pm list react

# Show dependency tree
vp pm list --depth 2

# JSON output
vp pm list --json

# List in specific workspace
vp pm list --filter app

# List across all workspaces
vp pm list -r

# List only production dependencies
vp pm list --prod

# List globally installed packages
vp pm list -g
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

#### 4. vp pm view / vp pm info / vp pm show

View package information from the registry.

```bash
vp pm view [<package-spec>] [<field>[.subfield]...] [OPTIONS]
vp pm info [<package-spec>] [<field>[.subfield]...] [OPTIONS]
vp pm show [<package-spec>] [<field>[.subfield]...] [OPTIONS]
```

**Examples:**

```bash
# View package information
vp pm view react

# View specific version
vp pm view react@18.3.1

# View specific field
vp pm view react version
vp pm view react dist.tarball

# View nested field
vp pm view react dependencies.prop-types

# JSON output
vp pm view react --json

# Use aliases
vp pm info lodash
vp pm show express
```

**Options:**

- `--json`: JSON output format

#### 5. vp pm publish

Publish package to the registry.

```bash
vp pm publish [TARBALL|FOLDER] [OPTIONS]
```

**Examples:**

```bash
# Publish current package
vp pm publish

# Publish specific tarball
vp pm publish package.tgz

# Dry run
vp pm publish --dry-run

# Set tag
vp pm publish --tag beta

# Set access level
vp pm publish --access public

# Recursive publish in monorepo
vp pm publish -r

# Publish with filter
vp pm publish --filter app
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

#### 6. vp pm owner

Manage package owners.

```bash
vp pm owner <subcommand> <package>
```

**Subcommands:**

- `list <package>`: List package owners
- `add <user> <package>`: Add owner
- `rm <user> <package>`: Remove owner

**Examples:**

```bash
# List package owners
vp pm owner list my-package

# Add owner
vp pm owner add username my-package

# Remove owner
vp pm owner rm username my-package
```

#### 7. vp pm cache

Manage package cache.

```bash
vp pm cache [SUBCOMMAND] [OPTIONS]
```

**Subcommands:**

- `dir` / `path`: Show cache directory
- `clean` / `clear`: Clean cache
- `verify`: Verify cache integrity (npm)
- `list`: List cached packages (pnpm)

**Examples:**

```bash
# Show cache directory
vp pm cache dir
vp pm cache path

# Clean cache
vp pm cache clean
vp pm cache clear

# Force clean (npm)
vp pm cache clean --force

# Verify cache (npm)
vp pm cache verify

# List cached packages (pnpm)
vp pm cache list
```

**Options:**

- `--force`: Force cache clean (npm)

#### 8. vp pm config / vp pm c

Manage package manager configuration.

```bash
vp pm config <subcommand> [key] [value] [OPTIONS]
vp pm c <subcommand> [key] [value] [OPTIONS]
```

**Subcommands:**

- `list`: List all configuration
- `get <key>`: Get configuration value
- `set <key> <value>`: Set configuration value
- `delete <key>`: Delete configuration key

**Examples:**

```bash
# List all config
vp pm config list

# Get config value
vp pm config get registry

# Set config value
vp pm config set registry https://registry.npmjs.org

# Set with JSON format (pnpm/yarn@2+)
vp pm config set registry https://registry.npmjs.org --json

# Set global config
vp pm config set registry https://registry.npmjs.org --global

# Set global config with location parameter (alternative)
vp pm config set registry https://registry.npmjs.org --location global

# Delete config key
vp pm config delete registry

# Use alias
vp pm c get registry
```

**Options:**

- `--json`: JSON output format (pnpm/yarn@2+)
- `-g, --global`: Use global config (shorthand for `--location global`)
- `--location <location>`: Config location: project (default) or global

#### 9. vp pm login

Log in to the registry to authenticate for publishing and other protected operations.

```bash
vp pm login [OPTIONS]
```

**Examples:**

```bash
# Log in to the default registry
vp pm login

# Log in to a custom registry
vp pm login --registry https://custom-registry.com

# Log in with scope
vp pm login --scope @myorg
```

**Options:**

- `--registry <url>`: Registry URL to log in to
- `--scope <scope>`: Associate the login with a scope

#### 10. vp pm logout

Log out from the registry, removing stored credentials.

```bash
vp pm logout [OPTIONS]
```

**Examples:**

```bash
# Log out from the default registry
vp pm logout

# Log out from a custom registry
vp pm logout --registry https://custom-registry.com

# Log out with scope
vp pm logout --scope @myorg
```

**Options:**

- `--registry <url>`: Registry URL to log out from
- `--scope <scope>`: Log out from a scoped registry

#### 11. vp pm whoami

Display the username of the currently logged-in user.

```bash
vp pm whoami [OPTIONS]
```

**Examples:**

```bash
# Show logged-in user
vp pm whoami

# Show logged-in user for a custom registry
vp pm whoami --registry https://custom-registry.com
```

**Options:**

- `--registry <url>`: Registry URL to check

#### 12. vp pm token

Manage registry authentication tokens. This command always delegates to `npm token` regardless of the detected package manager.

```bash
vp pm token <subcommand> [OPTIONS]
```

**Subcommands:**

- `list`: List all known tokens
- `create`: Create a new authentication token
- `revoke <token|id>`: Revoke a token

**Examples:**

```bash
# List all tokens
vp pm token list

# Create a new read-only token
vp pm token create --read-only

# Create a CIDR-whitelisted token
vp pm token create --cidr 192.168.1.0/24

# Revoke a token
vp pm token revoke a1b2c3d4
```

**Options:**

- `--read-only`: Create a read-only token
- `--cidr <cidr>`: CIDR range for token restriction
- `--registry <url>`: Registry URL

#### 13. vp pm audit

Run a security audit on installed packages to identify known vulnerabilities.

```bash
vp pm audit [OPTIONS]
```

**Examples:**

```bash
# Run security audit
vp pm audit

# JSON output
vp pm audit --json

# Audit only production dependencies
vp pm audit --prod

# Fix vulnerabilities automatically
vp pm audit fix

# Set minimum severity level
vp pm audit --audit-level high
```

**Options:**

- `--json`: JSON output format
- `--prod`: Audit only production dependencies
- `--audit-level <level>`: Minimum severity to report (low, moderate, high, critical)
- `fix`: Attempt to automatically fix vulnerabilities

#### 14. vp pm dist-tag

Manage distribution tags on packages, allowing you to label specific versions with meaningful names.

```bash
vp pm dist-tag <subcommand> <pkg> [OPTIONS]
```

**Subcommands:**

- `list [<pkg>]`: List distribution tags for a package
- `add <pkg>@<version> <tag>`: Add a tag to a specific version
- `rm <pkg> <tag>`: Remove a tag from a package

**Examples:**

```bash
# List distribution tags
vp pm dist-tag list my-package

# Tag a specific version as beta
vp pm dist-tag add my-package@1.0.0 beta

# Remove a tag
vp pm dist-tag rm my-package beta
```

**Options:**

- `--registry <url>`: Registry URL
- `--otp <otp>`: One-time password for authentication

#### 15. vp pm deprecate

Deprecate a version or range of versions of a package. This command always delegates to `npm deprecate` regardless of the detected package manager.

```bash
vp pm deprecate <package-spec> <message>
```

**Examples:**

```bash
# Deprecate a specific version
vp pm deprecate my-package@1.0.0 "Use v2 instead"

# Deprecate a range of versions
vp pm deprecate "my-package@<2.0.0" "Upgrade to v2 for security fixes"

# Un-deprecate by passing empty message
vp pm deprecate my-package@1.0.0 ""
```

**Options:**

- `--registry <url>`: Registry URL
- `--otp <otp>`: One-time password for authentication

#### 16. vp pm search

Search the registry for packages matching a query. This command always delegates to `npm search` regardless of the detected package manager.

```bash
vp pm search [OPTIONS] <search-terms...>
```

**Examples:**

```bash
# Search for packages
vp pm search vite plugin

# JSON output
vp pm search vite plugin --json

# Long format with description
vp pm search vite plugin --long

# Search with registry
vp pm search vite plugin --registry https://custom-registry.com
```

**Options:**

- `--json`: JSON output format
- `--long`: Show extended information
- `--registry <url>`: Registry URL
- `--searchlimit <number>`: Limit number of results

#### 17. vp pm rebuild

Rebuild native addons (e.g., node-gyp compiled modules) in the current project.

```bash
vp pm rebuild [OPTIONS] [<packages...>]
```

**Examples:**

```bash
# Rebuild all native addons
vp pm rebuild

# Rebuild specific packages
vp pm rebuild node-sass sharp
```

**Options:**

- Packages to rebuild can be specified as positional arguments

#### 18. vp pm fund

Show funding information for installed packages. This command always delegates to `npm fund` regardless of the detected package manager.

```bash
vp pm fund [OPTIONS] [<package>]
```

**Examples:**

```bash
# Show funding info for all dependencies
vp pm fund

# Show funding info for a specific package
vp pm fund lodash

# JSON output
vp pm fund --json

# Limit depth of dependency tree
vp pm fund --depth 1
```

**Options:**

- `--json`: JSON output format
- `--depth <n>`: Maximum depth of dependency tree

#### 19. vp pm ping

Ping the configured or specified registry to verify connectivity. This command always delegates to `npm ping` regardless of the detected package manager.

```bash
vp pm ping [OPTIONS]
```

**Examples:**

```bash
# Ping the default registry
vp pm ping

# Ping a custom registry
vp pm ping --registry https://custom-registry.com
```

**Options:**

- `--registry <url>`: Registry URL to ping

### Bun-Specific Subcommands

Bun provides several `bun pm` subcommands that may not have direct equivalents in other package managers:

- `bun pm ls` / `bun list` - List installed packages
- `bun pm bin` - Show the bin directory for installed binaries
- `bun pm cache` / `bun pm cache rm` - Cache management (show cache path / remove cached packages)
- `bun pm whoami` - Show the currently logged-in npm registry username
- `bun pm pack` - Create a tarball of the package (supports `--destination`, `--dry-run`)
- `bun pm trust` / `bun pm untrusted` - Manage trusted dependencies (allow lifecycle scripts)
- `bun pm version` - Show the installed version of bun
- `bun pm pkg` - Manage package.json fields programmatically
- `bun publish` - Publish package to the npm registry (direct subcommand, not `bun pm publish`)

**Note:** Many npm registry operations (login, logout, owner, dist-tag, deprecate, search, fund, ping, token) do not have native bun equivalents and delegate to `npm` when using bun as the package manager.

### Command Mapping

#### Prune Command

**pnpm references:**

- https://pnpm.io/cli/prune

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-prune

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/prune
- The prune command isn't necessary. yarn install will prune extraneous packages.

| Vite+ Flag      | pnpm            | npm               | yarn | bun | Description                 |
| --------------- | --------------- | ----------------- | ---- | --- | --------------------------- |
| `vp pm prune`   | `pnpm prune`    | `npm prune`       | N/A  | N/A | Remove unnecessary packages |
| `--prod`        | `--prod`        | `--omit=dev`      | N/A  | N/A | Remove devDependencies      |
| `--no-optional` | `--no-optional` | `--omit=optional` | N/A  | N/A | Remove optional deps        |

**Note:**

- npm supports prune with `--omit=dev` (for prod) and `--omit=optional` (for no-optional)
- yarn doesn't have a prune command (automatic during install)
- bun doesn't have a prune command

#### Pack Command

**pnpm references:**

- https://pnpm.io/cli/pack

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-pack

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/pack
- https://yarnpkg.com/cli/pack
- https://yarnpkg.com/cli/workspaces/foreach (for yarn@2+ recursive packing)

| Vite+ Flag                  | pnpm                 | npm                  | yarn@1       | yarn@2+                                       | bun             | Description                       |
| --------------------------- | -------------------- | -------------------- | ------------ | --------------------------------------------- | --------------- | --------------------------------- |
| `vp pm pack`                | `pnpm pack`          | `npm pack`           | `yarn pack`  | `yarn pack`                                   | `bun pm pack`   | Create package tarball            |
| `-r, --recursive`           | `--recursive`        | `--workspaces`       | N/A          | `workspaces foreach --all pack`               | N/A             | Pack all workspace packages       |
| `--filter <pattern>`        | `--filter`           | `--workspace`        | N/A          | `workspaces foreach --include <pattern> pack` | N/A             | Filter packages to pack           |
| `--out <path>`              | `--out`              | N/A                  | `--filename` | `--out`                                       | `--filename`    | Output file path (supports %s/%v) |
| `--pack-destination <dir>`  | `--pack-destination` | `--pack-destination` | N/A          | N/A                                           | `--destination` | Output directory                  |
| `--pack-gzip-level <level>` | `--pack-gzip-level`  | N/A                  | N/A          | N/A                                           | `--gzip-level`  | Gzip compression level (0-9)      |
| `--json`                    | `--json`             | `--json`             | `--json`     | `--json`                                      | N/A             | JSON output                       |
| `--dry-run`                 | N/A                  | `--dry-run`          | N/A          | N/A                                           | `--dry-run`     | Preview without creating tarball  |

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
  - Supported by pnpm and bun (bun uses `--gzip-level`)
  - npm and yarn do not support this option (prints warning and ignores)
- bun uses `bun pm pack` (not `bun pack`), supports `--destination` and `--dry-run` flags

#### List Command

**pnpm references:**

- https://pnpm.io/cli/list

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-ls

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/list

| Vite+ Flag           | pnpm              | npm                             | yarn@1        | yarn@2+       | bun           | Description                                   |
| -------------------- | ----------------- | ------------------------------- | ------------- | ------------- | ------------- | --------------------------------------------- |
| `vp pm list`         | `pnpm list`       | `npm list`                      | `yarn list`   | N/A           | `bun pm ls`   | List installed packages                       |
| `--depth <n>`        | `--depth <n>`     | `--depth <n>`                   | `--depth <n>` | N/A           | N/A           | Limit tree depth                              |
| `--json`             | `--json`          | `--json`                        | `--json`      | N/A           | N/A           | JSON output                                   |
| `--long`             | `--long`          | `--long`                        | N/A           | N/A           | N/A           | Extended info                                 |
| `--parseable`        | `--parseable`     | `--parseable`                   | N/A           | N/A           | N/A           | Parseable format                              |
| `-P, --prod`         | `--prod`          | `--include prod --include peer` | N/A           | N/A           | N/A           | Production deps only                          |
| `-D, --dev`          | `--dev`           | `--include dev`                 | N/A           | N/A           | N/A           | Dev deps only                                 |
| `--no-optional`      | `--no-optional`   | `--omit optional`               | N/A           | N/A           | N/A           | Exclude optional deps                         |
| `--exclude-peers`    | `--exclude-peers` | `--omit peer`                   | N/A           | N/A           | N/A           | Exclude peer deps                             |
| `--only-projects`    | `--only-projects` | N/A                             | N/A           | N/A           | N/A           | Show only project packages (pnpm)             |
| `--find-by <name>`   | `--find-by`       | N/A                             | N/A           | N/A           | N/A           | Use finder function from .pnpmfile.cjs (pnpm) |
| `-r, --recursive`    | `--recursive`     | `--workspaces`                  | N/A           | N/A           | N/A           | List across workspaces                        |
| `--filter <pattern>` | `--filter`        | `--workspace`                   | N/A           | N/A           | N/A           | Filter workspace                              |
| `-g, --global`       | `npm list -g`     | `npm list -g`                   | `npm list -g` | `npm list -g` | `npm list -g` | List global packages                          |

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

| Vite+ Flag   | pnpm       | npm        | yarn@1     | yarn@2+    | bun        | Description       |
| ------------ | ---------- | ---------- | ---------- | ---------- | ---------- | ----------------- |
| `vp pm view` | `npm view` | `npm view` | `npm view` | `npm view` | `bun info` | View package info |
| `--json`     | `--json`   | `--json`   | `--json`   | `--json`   | `--json`   | JSON output       |

**Note:**

- pnpm and yarn delegate to `npm view` for viewing package information
- bun has a native `bun info` command for viewing package information
- Aliases: `vp pm info` and `vp pm show` work the same as `vp pm view`

#### Publish Command

**pnpm references:**

- https://pnpm.io/cli/publish

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-publish

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/publish (delegates to npm publish)
- https://yarnpkg.com/cli/npm/publish (delegates to npm publish)

| Vite+ Flag                  | pnpm               | npm                | yarn@1             | yarn@2+            | bun           | Description                 |
| --------------------------- | ------------------ | ------------------ | ------------------ | ------------------ | ------------- | --------------------------- |
| `vp pm publish`             | `pnpm publish`     | `npm publish`      | `npm publish`      | `npm publish`      | `bun publish` | Publish package             |
| `--dry-run`                 | `--dry-run`        | `--dry-run`        | `--dry-run`        | `--dry-run`        | `--dry-run`   | Preview without publishing  |
| `--tag <tag>`               | `--tag <tag>`      | `--tag <tag>`      | `--tag <tag>`      | `--tag <tag>`      | `--tag <tag>` | Publish tag                 |
| `--access <level>`          | `--access <level>` | `--access <level>` | `--access <level>` | `--access <level>` | `--access`    | Public/restricted           |
| `--otp <otp>`               | `--otp`            | `--otp`            | `--otp`            | `--otp`            | N/A           | One-time password           |
| `--no-git-checks`           | `--no-git-checks`  | N/A                | N/A                | N/A                | N/A           | Skip git checks (pnpm)      |
| `--publish-branch <branch>` | `--publish-branch` | N/A                | N/A                | N/A                | N/A           | Set publish branch (pnpm)   |
| `--report-summary`          | `--report-summary` | N/A                | N/A                | N/A                | N/A           | Save publish summary (pnpm) |
| `--force`                   | `--force`          | `--force`          | `--force`          | `--force`          | N/A           | Force publish               |
| `--json`                    | `--json`           | N/A                | N/A                | N/A                | N/A           | JSON output (pnpm)          |
| `-r, --recursive`           | `--recursive`      | `--workspaces`     | N/A                | N/A                | N/A           | Publish workspaces          |
| `--filter <pattern>`        | `--filter`         | `--workspace`      | N/A                | N/A                | N/A           | Filter workspace            |

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

| Vite+ Flag                | pnpm             | npm              | yarn@1           | yarn@2+          | bun              | Description         |
| ------------------------- | ---------------- | ---------------- | ---------------- | ---------------- | ---------------- | ------------------- |
| `vp pm owner list <pkg>`  | `npm owner list` | `npm owner list` | `npm owner list` | `npm owner list` | `npm owner list` | List package owners |
| `vp pm owner add <u> <p>` | `npm owner add`  | `npm owner add`  | `npm owner add`  | `npm owner add`  | `npm owner add`  | Add owner           |
| `vp pm owner rm <u> <p>`  | `npm owner rm`   | `npm owner rm`   | `npm owner rm`   | `npm owner rm`   | `npm owner rm`   | Remove owner        |
| `--otp <otp>`             | `--otp`          | `--otp`          | `--otp`          | `--otp`          | `--otp`          | One-time password   |

**Note:**

- All package managers delegate to `npm owner` for managing package ownership
- pnpm and yarn both use npm's owner functionality internally
- Alias: `vp pm author` works the same as `vp pm owner`

#### Cache Command

**pnpm references:**

- https://pnpm.io/cli/store

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-cache

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/cache
- https://yarnpkg.com/cli/cache

| Vite+ Flag          | pnpm               | npm                    | yarn@1             | yarn@2+                       | bun               | Description          |
| ------------------- | ------------------ | ---------------------- | ------------------ | ----------------------------- | ----------------- | -------------------- |
| `vp pm cache dir`   | `pnpm store path`  | `npm config get cache` | `yarn cache dir`   | `yarn config get cacheFolder` | `bun pm cache`    | Show cache directory |
| `vp pm cache path`  | Alias for `dir`    | Alias for `dir`        | Alias for `dir`    | Alias for `dir`               | Alias for `dir`   | Alias for dir        |
| `vp pm cache clean` | `pnpm store prune` | `npm cache clean`      | `yarn cache clean` | `yarn cache clean`            | `bun pm cache rm` | Clean cache          |

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

| Vite+ Flag                  | pnpm                 | npm                 | yarn@1               | yarn@2+                     | bun | Description        |
| --------------------------- | -------------------- | ------------------- | -------------------- | --------------------------- | --- | ------------------ |
| `vp pm config list`         | `pnpm config list`   | `npm config list`   | `yarn config list`   | `yarn config`               | N/A | List configuration |
| `vp pm config get <key>`    | `pnpm config get`    | `npm config get`    | `yarn config get`    | `yarn config get`           | N/A | Get config value   |
| `vp pm config set <k> <v>`  | `pnpm config set`    | `npm config set`    | `yarn config set`    | `yarn config set`           | N/A | Set config value   |
| `vp pm config delete <key>` | `pnpm config delete` | `npm config delete` | `yarn config delete` | `yarn config unset`         | N/A | Delete config key  |
| `--json`                    | `--json`             | `--json`            | `--json`             | `--json`                    | N/A | JSON output        |
| `-g, --global`              | `--global`           | `--global`          | `--global`           | `--home`                    | N/A | Global config      |
| `--location <location>`     | `--location`         | `--location`        | N/A                  | Maps to `--home` for global | N/A | Config location    |

**Note:**

- Alias: `vp pm c` works the same as `vp pm config`
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

#### Login Command

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-login

**pnpm references:**

- https://pnpm.io/cli/login (delegates to npm login)

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/login
- https://yarnpkg.com/cli/npm/login

| Vite+ Flag         | pnpm         | npm          | yarn@1       | yarn@2+          | bun          | Description          |
| ------------------ | ------------ | ------------ | ------------ | ---------------- | ------------ | -------------------- |
| `vp pm login`      | `npm login`  | `npm login`  | `yarn login` | `yarn npm login` | `npm login`  | Log in to registry   |
| `--registry <url>` | `--registry` | `--registry` | `--registry` | `--registry`     | `--registry` | Registry URL         |
| `--scope <scope>`  | `--scope`    | `--scope`    | `--scope`    | `--scope`        | `--scope`    | Associate with scope |

**Note:**

- pnpm delegates to `npm login` for authentication
- yarn@1 uses its own `yarn login` command
- yarn@2+ uses `yarn npm login` via the npm plugin

#### Logout Command

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-logout

**pnpm references:**

- https://pnpm.io/cli/logout (delegates to npm logout)

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/logout
- https://yarnpkg.com/cli/npm/logout

| Vite+ Flag         | pnpm         | npm          | yarn@1        | yarn@2+           | bun          | Description           |
| ------------------ | ------------ | ------------ | ------------- | ----------------- | ------------ | --------------------- |
| `vp pm logout`     | `npm logout` | `npm logout` | `yarn logout` | `yarn npm logout` | `npm logout` | Log out from registry |
| `--registry <url>` | `--registry` | `--registry` | `--registry`  | `--registry`      | `--registry` | Registry URL          |
| `--scope <scope>`  | `--scope`    | `--scope`    | `--scope`     | `--scope`         | `--scope`    | Scoped registry       |

**Note:**

- pnpm delegates to `npm logout` for authentication
- yarn@1 uses its own `yarn logout` command
- yarn@2+ uses `yarn npm logout` via the npm plugin

#### Whoami Command

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-whoami

**pnpm references:**

- https://pnpm.io/cli/whoami (delegates to npm whoami)

**yarn references:**

- https://yarnpkg.com/cli/npm/whoami

| Vite+ Flag         | pnpm         | npm          | yarn@1     | yarn@2+           | bun             | Description         |
| ------------------ | ------------ | ------------ | ---------- | ----------------- | --------------- | ------------------- |
| `vp pm whoami`     | `npm whoami` | `npm whoami` | N/A (warn) | `yarn npm whoami` | `bun pm whoami` | Show logged-in user |
| `--registry <url>` | `--registry` | `--registry` | N/A        | `--registry`      | N/A             | Registry URL        |

**Note:**

- pnpm delegates to `npm whoami` for authentication
- yarn@1 does not have a `whoami` command (prints warning and ignores)
- yarn@2+ uses `yarn npm whoami` via the npm plugin

#### Token Command

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-token

| Vite+ Flag           | pnpm               | npm                | yarn@1     | yarn@2+    | bun        | Description      |
| -------------------- | ------------------ | ------------------ | ---------- | ---------- | ---------- | ---------------- |
| `vp pm token list`   | `npm token list`   | `npm token list`   | N/A (warn) | N/A (warn) | N/A (warn) | List tokens      |
| `vp pm token create` | `npm token create` | `npm token create` | N/A (warn) | N/A (warn) | N/A (warn) | Create token     |
| `vp pm token revoke` | `npm token revoke` | `npm token revoke` | N/A (warn) | N/A (warn) | N/A (warn) | Revoke token     |
| `--read-only`        | `--read-only`      | `--read-only`      | N/A        | N/A        | N/A        | Read-only token  |
| `--cidr <cidr>`      | `--cidr`           | `--cidr`           | N/A        | N/A        | N/A        | CIDR restriction |

**Note:**

- All package managers delegate to `npm token` since token management is npm-specific
- yarn@1 and yarn@2+ do not have a `token` command (prints warning and ignores)

#### Audit Command

**pnpm references:**

- https://pnpm.io/cli/audit

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-audit

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/audit
- https://yarnpkg.com/cli/npm/audit

| Vite+ Flag              | pnpm            | npm             | yarn@1          | yarn@2+                    | bun             | Description        |
| ----------------------- | --------------- | --------------- | --------------- | -------------------------- | --------------- | ------------------ |
| `vp pm audit`           | `pnpm audit`    | `npm audit`     | `yarn audit`    | `yarn npm audit`           | `bun audit`     | Run security audit |
| `--json`                | `--json`        | `--json`        | `--json`        | `--json`                   | `--json`        | JSON output        |
| `--prod`                | `--prod`        | `--omit=dev`    | `--groups prod` | `--environment production` | N/A             | Production only    |
| `--audit-level <level>` | `--audit-level` | `--audit-level` | `--level`       | `--severity`               | `--audit-level` | Minimum severity   |
| `fix`                   | `--fix`         | `npm audit fix` | N/A             | N/A                        | N/A             | Auto-fix           |

**Note:**

- pnpm uses `pnpm audit` natively
- npm uses `npm audit` natively
- yarn@1 uses `yarn audit` natively
- yarn@2+ uses `yarn npm audit` via the npm plugin
- `--prod` flag is mapped differently: pnpm uses `--prod`, npm uses `--omit=dev`, yarn@1 uses `--groups prod`, yarn@2+ uses `--environment production`
- `audit fix` is only supported by pnpm (via `--fix`) and npm (via `npm audit fix`); yarn does not support it

#### Dist-Tag Command

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-dist-tag

**yarn references:**

- https://classic.yarnpkg.com/en/docs/cli/tag
- https://yarnpkg.com/cli/npm/tag

| Vite+ Flag                       | pnpm                | npm                 | yarn@1          | yarn@2+             | bun                 | Description       |
| -------------------------------- | ------------------- | ------------------- | --------------- | ------------------- | ------------------- | ----------------- |
| `vp pm dist-tag list <pkg>`      | `npm dist-tag list` | `npm dist-tag list` | `yarn tag list` | `yarn npm tag list` | `npm dist-tag list` | List tags         |
| `vp pm dist-tag add <pkg> <tag>` | `npm dist-tag add`  | `npm dist-tag add`  | `yarn tag add`  | `yarn npm tag add`  | `npm dist-tag add`  | Add tag           |
| `vp pm dist-tag rm <pkg> <tag>`  | `npm dist-tag rm`   | `npm dist-tag rm`   | `yarn tag rm`   | `yarn npm tag rm`   | `npm dist-tag rm`   | Remove tag        |
| `--otp <otp>`                    | `--otp`             | `--otp`             | `--otp`         | `--otp`             | `--otp`             | One-time password |

**Note:**

- pnpm delegates to `npm dist-tag` for tag management
- npm uses `npm dist-tag` natively
- yarn@1 uses `yarn tag` instead of `dist-tag`
- yarn@2+ uses `yarn npm tag` via the npm plugin

#### Deprecate Command

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-deprecate

| Vite+ Flag                    | pnpm            | npm             | yarn@1          | yarn@2+         | bun             | Description         |
| ----------------------------- | --------------- | --------------- | --------------- | --------------- | --------------- | ------------------- |
| `vp pm deprecate <pkg> <msg>` | `npm deprecate` | `npm deprecate` | `npm deprecate` | `npm deprecate` | `npm deprecate` | Deprecate a package |
| `--otp <otp>`                 | `--otp`         | `--otp`         | `--otp`         | `--otp`         | `--otp`         | One-time password   |
| `--registry <url>`            | `--registry`    | `--registry`    | `--registry`    | `--registry`    | `--registry`    | Registry URL        |

**Note:**

- All package managers delegate to `npm deprecate` since deprecation is an npm registry feature
- Pass an empty message to un-deprecate a package version

#### Search Command

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-search

| Vite+ Flag             | pnpm            | npm             | yarn@1          | yarn@2+         | bun             | Description         |
| ---------------------- | --------------- | --------------- | --------------- | --------------- | --------------- | ------------------- |
| `vp pm search <terms>` | `npm search`    | `npm search`    | `npm search`    | `npm search`    | `npm search`    | Search for packages |
| `--json`               | `--json`        | `--json`        | `--json`        | `--json`        | `--json`        | JSON output         |
| `--long`               | `--long`        | `--long`        | `--long`        | `--long`        | `--long`        | Extended info       |
| `--searchlimit <n>`    | `--searchlimit` | `--searchlimit` | `--searchlimit` | `--searchlimit` | `--searchlimit` | Limit results       |

**Note:**

- All package managers delegate to `npm search` since search is an npm registry feature

#### Rebuild Command

**pnpm references:**

- https://pnpm.io/cli/rebuild

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-rebuild

| Vite+ Flag               | pnpm                 | npm                 | yarn@1     | yarn@2+    | bun        | Description           |
| ------------------------ | -------------------- | ------------------- | ---------- | ---------- | ---------- | --------------------- |
| `vp pm rebuild`          | `pnpm rebuild`       | `npm rebuild`       | N/A (warn) | N/A (warn) | N/A (warn) | Rebuild native addons |
| `vp pm rebuild <pkg...>` | `pnpm rebuild <pkg>` | `npm rebuild <pkg>` | N/A (warn) | N/A (warn) | N/A (warn) | Rebuild specific pkgs |

**Note:**

- pnpm uses `pnpm rebuild` natively
- npm uses `npm rebuild` natively
- yarn@1 does not have a `rebuild` command (prints warning and ignores)
- yarn@2+ does not have a `rebuild` command (prints warning and ignores)
- Packages to rebuild can be specified as positional arguments

#### Fund Command

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-fund

| Vite+ Flag         | pnpm       | npm        | yarn@1     | yarn@2+    | bun        | Description           |
| ------------------ | ---------- | ---------- | ---------- | ---------- | ---------- | --------------------- |
| `vp pm fund`       | `npm fund` | `npm fund` | N/A (warn) | N/A (warn) | N/A (warn) | Show funding info     |
| `vp pm fund <pkg>` | `npm fund` | `npm fund` | N/A (warn) | N/A (warn) | N/A (warn) | Fund for specific pkg |
| `--json`           | `--json`   | `--json`   | N/A        | N/A        | N/A        | JSON output           |
| `--depth <n>`      | `--depth`  | `--depth`  | N/A        | N/A        | N/A        | Limit depth           |

**Note:**

- All package managers delegate to `npm fund` since funding is an npm-specific feature
- yarn@1 does not have a `fund` command (prints warning and ignores)
- yarn@2+ does not have a `fund` command (prints warning and ignores)

#### Ping Command

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-ping

| Vite+ Flag         | pnpm         | npm          | yarn@1       | yarn@2+      | bun          | Description   |
| ------------------ | ------------ | ------------ | ------------ | ------------ | ------------ | ------------- |
| `vp pm ping`       | `npm ping`   | `npm ping`   | `npm ping`   | `npm ping`   | `npm ping`   | Ping registry |
| `--registry <url>` | `--registry` | `--registry` | `--registry` | `--registry` | `--registry` | Registry URL  |

**Note:**

- All package managers delegate to `npm ping` since registry ping is an npm-specific feature

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

    /// View package information from the registry
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

                // Map vp pm commands to pnpm commands
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
                        eprintln!("Warning: npm removed 'prune' command in v6. Use 'vp install --prod' instead.");
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

- Keeps Vite+ CLI namespace clean
- Clear categorization (pm utilities vs task running)
- Matches Bun's design pattern
- Extensible for future utilities

## Error Handling

### No Package Manager Detected

```bash
$ vp pm list
Error: No package manager detected
Please run one of:
  - vp install (to set up package manager)
  - Add packageManager field to package.json
```

### Unsupported Command

```bash
$ vp pm prune
Detected package manager: yarn@4.0.0
Warning: yarn does not have 'prune' command. yarn install will prune extraneous packages automatically.
$ echo $?
0
```

### Command Failed

```bash
$ vp pm publish
Detected package manager: pnpm@10.15.0
Running: pnpm publish
Error: You must be logged in to publish packages
Exit code: 1
```

## User Experience

### Prune Packages

```bash
$ vp pm prune
Detected package manager: pnpm@10.15.0
Running: pnpm prune
Packages: -12

$ vp pm prune --prod
Detected package manager: npm@11.0.0
Running: npm prune --omit=dev
removed 45 packages
```

### Cache Management

```bash
$ vp pm cache dir
Detected package manager: pnpm@10.15.0
Running: pnpm store path
/Users/user/Library/pnpm/store

$ vp pm cache clean
Detected package manager: pnpm@10.15.0
Running: pnpm store prune
Removed 145 packages
```

### List Packages

```bash
$ vp pm list --depth 0
Detected package manager: pnpm@10.15.0
Running: pnpm list --depth 0

my-app@1.0.0
├── react@18.3.1
├── react-dom@18.3.1
└── lodash@4.17.21
```

### View Package

```bash
$ vp pm view react version
Detected package manager: npm@11.0.0
Running: npm view react version
18.3.1
```

### Publish Package

```bash
$ vp pm publish --dry-run
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
$ vp pm config get registry
Detected package manager: pnpm@10.15.0
Running: pnpm config get registry
https://registry.npmjs.org

$ vp pm config set registry https://custom-registry.com
Detected package manager: pnpm@10.15.0
Running: pnpm config set registry https://custom-registry.com
```

## Alternative Designs Considered

### Alternative 1: Individual Top-Level Commands

```bash
vp cache dir
vp publish
vp pack
```

**Rejected because**:

- Clutters top-level namespace
- Mixes task running with PM utilities
- Less clear categorization
- Harder to discover related commands

### Alternative 2: Full Flag Mapping

```bash
# Try to map all package manager flags
vp pm list --production  # Map to --prod (pnpm), --production (npm)
```

**Rejected because**:

- Maintenance burden as PMs add/change flags
- Incomplete mapping would be confusing
- Pass-through is more flexible
- Users can refer to PM docs for advanced usage

### Alternative 3: Single Pass-Through Command

```bash
vp pm -- pnpm store path
vp pm -- npm cache dir
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
$ vp pm --help
Package manager utilities

Usage: vp pm <COMMAND>

Commands:
  prune      Remove unnecessary packages
  pack       Create a tarball of the package
  list       List installed packages (alias: ls)
  view       View package information from the registry
  publish    Publish package to registry
  owner      Manage package owners
  cache      Manage package cache
  config     Manage package manager configuration
  login      Log in to the registry
  logout     Log out from the registry
  whoami     Show the currently logged-in user
  token      Manage registry authentication tokens
  audit      Run a security audit on installed packages
  dist-tag   Manage distribution tags on packages
  deprecate  Deprecate a version of a package
  search     Search the registry for packages
  rebuild    Rebuild native addons
  fund       Show funding information for installed packages
  ping       Ping the registry
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help

$ vp pm cache --help
Manage package cache

Usage: vp pm cache [SUBCOMMAND] [OPTIONS]

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
  vp pm cache dir              # Show cache directory
  vp pm cache clean            # Clean cache
  vp pm cache clean --force    # Force clean (npm)
  vp pm cache verify           # Verify cache (npm)
  vp pm cache list             # List cached packages (pnpm)
```

## Package Manager Compatibility

| Subcommand | pnpm       | npm     | yarn@1     | yarn@2+          | bun                | Notes                                   |
| ---------- | ---------- | ------- | ---------- | ---------------- | ------------------ | --------------------------------------- |
| prune      | ✅ Full    | ✅ Full | ❌ N/A     | ❌ N/A           | ❌ N/A             | npm uses --omit flags, yarn auto-prunes |
| pack       | ✅ Full    | ✅ Full | ✅ Full    | ✅ Full          | ✅ `bun pm pack`   | bun uses `bun pm pack`                  |
| list/ls    | ✅ Full    | ✅ Full | ⚠️ Limited | ❌ N/A           | ⚠️ `bun pm ls`     | bun has basic list support              |
| view       | ✅ Full    | ✅ Full | ⚠️ `info`  | ⚠️ `info`        | ✅ `npm view`      | delegates to npm                        |
| publish    | ✅ Full    | ✅ Full | ✅ Full    | ⚠️ `npm publish` | ✅ `bun publish`   | bun has native publish                  |
| owner      | ✅ Full    | ✅ Full | ✅ Full    | ⚠️ `npm owner`   | ✅ `npm owner`     | delegates to npm                        |
| cache      | ⚠️ `store` | ✅ Full | ✅ Full    | ✅ Full          | ⚠️ `bun pm cache`  | bun uses `bun pm cache`                 |
| config     | ✅ Full    | ✅ Full | ✅ Full    | ⚠️ Different     | ❌ N/A             | bun has no config command               |
| login      | ✅ `npm`   | ✅ Full | ✅ Full    | ⚠️ `npm login`   | ✅ `npm login`     | delegates to npm                        |
| logout     | ✅ `npm`   | ✅ Full | ✅ Full    | ⚠️ `npm logout`  | ✅ `npm logout`    | delegates to npm                        |
| whoami     | ✅ `npm`   | ✅ Full | ❌ N/A     | ⚠️ `npm whoami`  | ✅ `bun pm whoami` | bun has native whoami                   |
| token      | ✅ `npm`   | ✅ Full | ❌ N/A     | ❌ N/A           | ❌ N/A             | Always delegates to npm                 |
| audit      | ✅ Full    | ✅ Full | ✅ Full    | ⚠️ `npm audit`   | ✅ `bun audit`     | bun has native audit support            |
| dist-tag   | ✅ `npm`   | ✅ Full | ⚠️ `tag`   | ⚠️ `npm tag`     | ✅ `npm dist-tag`  | delegates to npm                        |
| deprecate  | ✅ `npm`   | ✅ Full | ✅ `npm`   | ✅ `npm`         | ✅ `npm deprecate` | Always delegates to npm                 |
| search     | ✅ `npm`   | ✅ Full | ✅ `npm`   | ✅ `npm`         | ✅ `npm search`    | Always delegates to npm                 |
| rebuild    | ✅ Full    | ✅ Full | ❌ N/A     | ❌ N/A           | ❌ N/A             | bun has no rebuild command              |
| fund       | ✅ `npm`   | ✅ Full | ❌ N/A     | ❌ N/A           | ❌ N/A             | Always delegates to npm                 |
| ping       | ✅ `npm`   | ✅ Full | ✅ `npm`   | ✅ `npm`         | ✅ `npm ping`      | Always delegates to npm                 |

## Future Enhancements

### 1. Interactive Cache Management

```bash
vp pm cache --interactive
# Shows cache size, allows selective cleaning
```

### 2. Publish Dry-Run Summary

```bash
vp pm publish --dry-run --summary
# Shows what would be published with sizes
```

### 3. Config Validation

```bash
vp pm config validate
# Checks configuration for issues
```

### 4. Owner Management UI

```bash
vp pm owner --interactive my-package
# Interactive UI for adding/removing owners
```

### 5. Cache Analytics

```bash
vp pm cache stats
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
- run: vp pm cache clean --force

# Show cache location for debugging
- run: vp pm cache dir
```

### Publishing Workflow

```bash
# Build packages
vp build -r

# Dry run to verify
vp pm publish --dry-run -r

# Publish with beta tag
vp pm publish --tag beta -r

# Publish only specific packages
vp pm publish --filter app
```

### Configuration Management

```bash
# Set custom registry
vp pm config set registry https://custom-registry.com

# Verify configuration
vp pm config get registry

# List all configuration
vp pm config list
```

### Dependency Auditing

```bash
# List dependencies to JSON file
vp pm list --json > deps.json

# List production dependencies
vp pm list --prod

# List specific workspace
vp pm list --filter app
```

## Conclusion

This RFC proposes adding `vp pm` command group to provide unified access to package manager utilities across pnpm/npm/yarn/bun. The design:

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
