# Command Line Interface

## `vite` CLI

The `vite` command is the main entry point for Vite+ (vite-plus), a monorepo task runner with intelligent caching and dependency resolution.

**Type:** `vite <COMMAND> [ARGS] [OPTIONS]`

## Dev Server

### `vite dev`

Start Vite dev server in the current directory. `vite serve` is an alias for `vite dev`.

#### Usage

```bash
vite dev [root] [OPTIONS]
```

#### Arguments

| Arguments | Description                                                                       |
| --------- | --------------------------------------------------------------------------------- |
| `root`    | The root directory to start the dev server from, default is the current directory |

#### Options

| Options                   |                                                                                                                                                                                      |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `--host [host]`           | Specify hostname (`string`)                                                                                                                                                          |
| `--port <port>`           | Specify port (`number`)                                                                                                                                                              |
| `--open [path]`           | Open browser on startup (`boolean \| string`)                                                                                                                                        |
| `--cors`                  | Enable CORS (`boolean`)                                                                                                                                                              |
| `--strictPort`            | Exit if specified port is already in use (`boolean`)                                                                                                                                 |
| `--force`                 | Force the optimizer to ignore the cache and re-bundle (`boolean`)                                                                                                                    |
| `-c, --config <file>`     | Use specified config file (`string`)                                                                                                                                                 |
| `--base <path>`           | Public base path (default: `/`) (`string`)                                                                                                                                           |
| `-l, --logLevel <level>`  | info \| warn \| error \| silent (`string`)                                                                                                                                           |
| `--clearScreen`           | Allow/disable clear screen when logging (`boolean`)                                                                                                                                  |
| `--configLoader <loader>` | Use `bundle` to bundle the config with esbuild, or `runner` (experimental) to process it on the fly, or `native` (experimental) to load using the native runtime (default: `bundle`) |
| `--profile`               | Start built-in Node.js inspector (check [Performance bottlenecks](./troubleshooting.md#performance-bottlenecks))                                                                     |
| `-d, --debug [feat]`      | Show debug logs (`string \| boolean`)                                                                                                                                                |
| `-f, --filter <filter>`   | Filter debug logs (`string`)                                                                                                                                                         |
| `-m, --mode <mode>`       | Set env mode (`string`)                                                                                                                                                              |
| `-h, --help`              | Display available CLI options                                                                                                                                                        |

#### Examples

```bash
vite dev
vite dev ./apps/website
vite dev --port 3000
```

## Build Application

### `vite build`

Build for production.

#### Usage

```bash
vite build [root] [OPTIONS]
```

#### Arguments

| Arguments | Description                                                        |
| --------- | ------------------------------------------------------------------ |
| `root`    | The root directory to build from, default is the current directory |

#### Options

| Options                        |                                                                                                                        |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------- |
| `--target <target>`            | Transpile target (default: `"modules"`) (`string`)                                                                     |
| `--outDir <dir>`               | Output directory (default: `dist`) (`string`)                                                                          |
| `--assetsDir <dir>`            | Directory under outDir to place assets in (default: `"assets"`) (`string`)                                             |
| `--assetsInlineLimit <number>` | Static asset base64 inline threshold in bytes (default: `4096`) (`number`)                                             |
| `--ssr [entry]`                | Build specified entry for server-side rendering (`string`)                                                             |
| `--sourcemap [output]`         | Output source maps for build (default: `false`) (`boolean \| "inline" \| "hidden"`)                                    |
| `--minify [minifier]`          | Enable/disable minification, or specify minifier to use (default: `"esbuild"`) (`boolean \| "terser" \| "esbuild"`)    |
| `--manifest [name]`            | Emit build manifest json (`boolean \| string`)                                                                         |
| `--ssrManifest [name]`         | Emit ssr manifest json (`boolean \| string`)                                                                           |
| `--emptyOutDir`                | Force empty outDir when it's outside of root (`boolean`)                                                               |
| `-w, --watch`                  | Rebuilds when modules have changed on disk (`boolean`)                                                                 |
| `-c, --config <file>`          | Use specified config file (`string`)                                                                                   |
| `--base <path>`                | Public base path (default: `/`) (`string`)                                                                             |
| `-l, --logLevel <level>`       | Info \| warn \| error \| silent (`string`)                                                                             |
| `--clearScreen`                | Allow/disable clear screen when logging (`boolean`)                                                                    |
| `--configLoader <loader>`      | Use `bundle` to bundle the config with esbuild or `runner` (experimental) to process it on the fly (default: `bundle`) |
| `--profile`                    | Start built-in Node.js inspector (check [Performance bottlenecks](./troubleshooting.md#performance-bottlenecks))       |
| `-d, --debug [feat]`           | Show debug logs (`string \| boolean`)                                                                                  |
| `-f, --filter <filter>`        | Filter debug logs (`string`)                                                                                           |
| `-m, --mode <mode>`            | Set env mode (`string`)                                                                                                |
| `-h, --help`                   | Display available CLI options                                                                                          |
| `--app`                        | Build all environments, same as `builder: {}` (`boolean`, experimental)                                                |

## Build Library

### `vite lib`

Build a library using tsdown.

#### Usage

```bash
vite lib [<ARGS>...]
```

#### Examples

```bash
vite lib
vite lib --watch
vite lib --outdir dist
```

## Build Documentation

### `vite doc`

Build documentation using VitePress.

#### Usage

```bash
vite doc [<ARGS>...]
```

#### Examples

```bash
vite doc build
vite doc dev
vite doc dev --host 0.0.0.0
```

## Lint

### `vite lint`

Lint code using oxlint.

#### Usage

```bash
vite lint [<ARGS>...]
```

#### Examples

```bash
vite lint
vite lint --fix
vite lint --quiet
```

## Format

### `vite fmt`

Format code using oxfmt.

#### Usage

```bash
vite fmt [<ARGS>...]
```

#### Examples

```bash
vite fmt
vite fmt --check
vite fmt --ignore-path .gitignore
```

## Testing

### `vite test`

Run tests using Vitest.

#### Usage

```bash
vite test [<ARGS>...]
```

#### Examples

```bash
vite test
vite test --watch
vite test run --coverage
```

## Task Runner

### `vite run`

Run tasks across monorepo packages with automatic dependency ordering.

#### Usage

```bash
vite run <TASKS>... [OPTIONS] [-- <TASK_ARGS>...]
```

#### Examples

```bash
# Run build in specific packages
vite run app#build web#build

# Run build recursively across all packages
vite run build --recursive

# Run without topological ordering
vite run build --recursive --no-topological

# Pass arguments to tasks
vite run test -- --watch --coverage
```

#### Options

| Option             | Alias | Description                                               |
| ------------------ | ----- | --------------------------------------------------------- |
| `--recursive`      | `-r`  | Run task in all packages that have it                     |
| `--no-recursive`   |       | Explicitly disable recursive mode                         |
| `--sequential`     | `-s`  | Run tasks sequentially (future)                           |
| `--no-sequential`  |       | Explicitly disable sequential mode                        |
| `--parallel`       | `-p`  | Run tasks in parallel (future)                            |
| `--no-parallel`    |       | Explicitly disable parallel mode                          |
| `--topological`    | `-t`  | Enable topological ordering based on package dependencies |
| `--no-topological` |       | Disable topological ordering                              |

#### Task Naming

- **Scoped**: `<package>#<task>` (e.g., `app#build`)
- **Unscoped**: `<task>` (runs in current package or all packages with `-r`)

#### Dependency Modes

1. **Explicit Dependencies** (always applied):
   - Defined in `vite-task.json` `dependsOn` field
   - Example: `"test": { "dependsOn": ["build", "lint"] }`

2. **Implicit Dependencies** (when `--topological`):
   - Based on package.json dependencies
   - If package A depends on B, then `A#build` depends on `B#build`
   - Default: ON for `--recursive`, OFF otherwise

### Global Options

| Option       | Alias | Description                             |
| ------------ | ----- | --------------------------------------- |
| `--debug`    | `-d`  | Display cache information for debugging |
| `--no-debug` |       | Explicitly disable debug mode           |
| `--help`     | `-h`  | Display help information                |
| `--version`  | `-V`  | Show version number                     |

### Task Configuration

Tasks are configured in `vite-task.json` files:

```json
{
  "tasks": {
    "build": {
      "command": "tsc && rollup",
      "cacheable": true,
      "dependsOn": ["lint"],
      "envs": ["NODE_ENV"],
      "fingerprintIgnores": [
        "node_modules/**/*",
        "!node_modules/**/package.json"
      ]
    }
  }
}
```

#### Configuration Fields

| Field                | Type       | Description                                         |
| -------------------- | ---------- | --------------------------------------------------- |
| `command`            | `string`   | Shell command to execute                            |
| `cacheable`          | `boolean`  | Whether to cache task results (default: `true`)     |
| `dependsOn`          | `string[]` | Explicit task dependencies                          |
| `cwd`                | `string`   | Working directory for the command                   |
| `envs`               | `string[]` | Environment variables that affect cache             |
| `passThroughEnvs`    | `string[]` | Environment variables passed but don't affect cache |
| `inputs`             | `string[]` | Input file patterns (future)                        |
| `fingerprintIgnores` | `string[]` | Glob patterns to exclude from cache fingerprint     |

### Fingerprint Ignores

Control which files trigger cache invalidation:

```json
{
  "tasks": {
    "install": {
      "command": "pnpm install",
      "cacheable": true,
      "fingerprintIgnores": [
        "node_modules/**/*",
        "!node_modules/**/package.json"
      ]
    }
  }
}
```

#### Pattern Syntax

- **Basic**: `node_modules/**/*` - ignore all files under node_modules
- **Negation**: `!node_modules/**/package.json` - include package.json files
- **Order matters**: Last matching pattern wins (gitignore-style)

### Caching

Vite+ uses intelligent caching to speed up task execution:

#### Cache Key Components

- Command fingerprint (command + cwd + envs)
- Post-run fingerprint (input file hashes)
- Task arguments

#### Cache Behavior

```bash
# First run - executes task
vite run build
# → Cache miss: no previous cache entry found

# Second run - replays from cache
vite run build
# → Cache hit - output replayed

# Modify source file
echo "new content" > src/index.ts
# → Cache miss: content of input 'src/index.ts' changed

# Modify ignored file (with fingerprintIgnores)
echo "modified" > node_modules/pkg/index.js
# → Cache hit - ignored file doesn't invalidate cache
```

#### Cache Location

- Default: `.vite-task-cache/` in workspace root
- Contains SQLite database with task results

#### Debug Cache

```bash
# View cache entries
vite cache view

# Enable cache debug output
vite run build --debug

# Clean cache
vite cache clean
```

### Environment Variables

| Variable                    | Description                                             |
| --------------------------- | ------------------------------------------------------- |
| `VITE_LOG`                  | Set logging level (e.g., `VITE_LOG=debug`)              |
| `VITE_DISABLE_AUTO_INSTALL` | Set to `1` to disable automatic dependency installation |
| `VITE_TASK_EXECUTION_ENV`   | Internal: indicates running inside vite task            |
| `VITE_TASK_EXECUTION_ID`    | Internal: unique ID for task execution                  |

### Execution Modes

#### Explicit Mode (default)

Run specific tasks:

```bash
vite run app#build web#build
```

#### Recursive Mode

Run task in all packages:

```bash
vite run build --recursive
vite run build -r
```

**Behavior:**

- Finds all packages with the specified task
- Runs tasks in dependency order (when `--topological`)
- Default topological ordering for recursive mode

#### Topological Ordering

Controls implicit dependencies based on package relationships:

```bash
# With topological (default for recursive)
vite run build -r
# → If A depends on B, A#build waits for B#build

# Without topological
vite run build -r --no-topological
# → Only explicit dependencies, no implicit ordering
```

### Task Arguments

Pass arguments to tasks using `--`:

```bash
# Arguments go to all tasks
vite run build test -- --watch

# For built-in commands
vite test -- --coverage --run
vite lint -- --fix
```

### Examples

#### Common Workflows

```bash
# Install dependencies
vite install

# Lint and fix issues
vite lint -- --fix

# Format code
vite fmt

# Run build recursively
vite run build -r

# Run tests in watch mode
vite test -- --watch

# Build for production
vite build

# Start dev server
vite dev

# Build library
vite lib

# Build docs
vite doc build

# Preview docs
vite doc dev
```

#### Monorepo Workflows

```bash
# Build all packages in dependency order
vite run build --recursive

# Build specific packages
vite run app#build utils#build

# Run tests without topological ordering
vite run test -r --no-topological

# Clean cache and rebuild
vite cache clean
vite run build -r
```

### Exit Codes

- `0` - Success (all tasks completed successfully)
- `Non-zero` - Failure (one or more tasks failed), or system error (workspace not found, cache error, etc.)

**Note:** When running multiple tasks, the first non-zero exit code is returned.

### Debugging

Enable verbose logging:

```bash
# Debug mode - shows cache operations
vite run build --debug

# Trace logging
VITE_LOG=debug vite run build

# View cache contents
vite cache view
```

## Package Management

### `vite install`

Aliases: `vite i`

Install dependencies using the detected package manager.

#### Usage

```bash
vite install [ARGS] [OPTIONS]
```

#### Examples

```bash
vite install
vite install --loglevel debug
```

#### Note

- Auto-detects package manager (pnpm/yarn/npm)
- Prompts for selection if none package manager detected

### `vite update`

Aliases: `vite up`

Updates packages to their latest version based on the specified range.

#### Usage

```bash
vite update [-g] [<pkg>...]
```

#### Examples

```bash
vite update
vite update @types/node
```

### `vite add`

Installs packages.

#### Usage

```bash
vite add [OPTIONS] <package>[@version]...
```

#### Examples

```bash
vite add -D @types/node
```

### `vite remove`

Aliases: `vite rm`, `vite uninstall`, `vite un`

Removes packages.

#### Usage

```bash
vite remove <package>[@version]...
```

```bash
vite remove @types/node
```

### `vite link`

Aliases: `vite ln`

Makes the current local package accessible system-wide, or in another location.

### `vite unlink`

Unlinks a system-wide package (inverse of `vite link`).

If called without arguments, all linked dependencies will be unlinked inside the current project.

### `vite dedupe`

Perform an install removing older dependencies in the lockfile if a newer version can be used.

### `vite outdated`

Shows outdated packages.

### `vite why`

Aliases: `vite explain`

Shows all packages that depend on the specified package.

### `vite pm <subcommand>`

The `vite pm` command group provides a set of utilities for working with package manager.

> package manager commands with low usage frequency will under this command group.

#### `vite pm prune`

Removes unnecessary packages.

#### `vite pm pack`

Pack the current package into a tarball.

#### `vite pm list`

Aliases: `vite pm ls`

List installed packages.

#### `vite pm view`

View a package info from the registry.

#### `vite pm publish`

Publishes a package to the registry.

#### `vite pm owner`

Manage package owners.

#### `vite pm cache`

Manage the packages metadata cache.

## Others

### `vite optimize`

Pre-bundle dependencies.

**Deprecated**: the pre-bundle process runs automatically and does not need to be called.

#### Usage

```bash
vite optimize [root]
```

#### Options

| Options                   |                                                                                                                        |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `--force`                 | Force the optimizer to ignore the cache and re-bundle (`boolean`)                                                      |
| `-c, --config <file>`     | Use specified config file (`string`)                                                                                   |
| `--base <path>`           | Public base path (default: `/`) (`string`)                                                                             |
| `-l, --logLevel <level>`  | Info \| warn \| error \| silent (`string`)                                                                             |
| `--clearScreen`           | Allow/disable clear screen when logging (`boolean`)                                                                    |
| `--configLoader <loader>` | Use `bundle` to bundle the config with esbuild or `runner` (experimental) to process it on the fly (default: `bundle`) |
| `-d, --debug [feat]`      | Show debug logs (`string \| boolean`)                                                                                  |
| `-f, --filter <filter>`   | Filter debug logs (`string`)                                                                                           |
| `-m, --mode <mode>`       | Set env mode (`string`)                                                                                                |
| `-h, --help`              | Display available CLI options                                                                                          |

### `vite preview`

Locally preview the production build. Do not use this as a production server as it's not designed for it.

This command starts a server in the build directory (by default `dist`). Run `vite build` beforehand to ensure that the build directory is up-to-date. Depending on the project's configured [`appType`](../config/shared-options.md#apptype), it makes use of certain middleware.

#### Usage

```bash
vite preview [root]
```

#### Options

| Options                   |                                                                                                                        |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `--host [host]`           | Specify hostname (`string`)                                                                                            |
| `--port <port>`           | Specify port (`number`)                                                                                                |
| `--strictPort`            | Exit if specified port is already in use (`boolean`)                                                                   |
| `--open [path]`           | Open browser on startup (`boolean \| string`)                                                                          |
| `--outDir <dir>`          | Output directory (default: `dist`)(`string`)                                                                           |
| `-c, --config <file>`     | Use specified config file (`string`)                                                                                   |
| `--base <path>`           | Public base path (default: `/`) (`string`)                                                                             |
| `-l, --logLevel <level>`  | Info \| warn \| error \| silent (`string`)                                                                             |
| `--clearScreen`           | Allow/disable clear screen when logging (`boolean`)                                                                    |
| `--configLoader <loader>` | Use `bundle` to bundle the config with esbuild or `runner` (experimental) to process it on the fly (default: `bundle`) |
| `-d, --debug [feat]`      | Show debug logs (`string \| boolean`)                                                                                  |
| `-f, --filter <filter>`   | Filter debug logs (`string`)                                                                                           |
| `-m, --mode <mode>`       | Set env mode (`string`)                                                                                                |
| `-h, --help`              | Display available CLI options                                                                                          |

### `vite cache`

Manage the task cache.

#### Usage

```bash
vite cache <clean|view>
```

#### `vite cache clean`

Clean up all cached task results.

```bash
vite cache clean
```

#### `vite cache view`

View cache entries in JSON format for debugging.

```bash
vite cache view
```

## See Also

- [Task Configuration](./tasks.md#configuration)
- [Caching Strategy](./caching.md)
- [Monorepo Guide](./monorepo.md)
