# Command Line Interface

## `vp` CLI

The `vp` command is the main entry point for Vite+ (vite-plus), a monorepo task runner with intelligent caching and dependency resolution.

**Type:** `vp <COMMAND> [ARGS] [OPTIONS]`

## Dev Server

### `vp dev`

Start Vite dev server in the current directory. `vp serve` is an alias for `vp dev`.

#### Usage

```bash
vp dev [root] [OPTIONS]
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
vp dev
vp dev ./apps/website
vp dev --port 3000
```

## Build Application

### `vp build`

Build for production.

#### Usage

```bash
vp build [root] [OPTIONS]
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

### `vp pack`

Build a library using tsdown.

#### Usage

```bash
vp pack [<ARGS>...]
```

#### Examples

```bash
vp pack
vp pack --watch
vp pack --outdir dist
```

## Build Documentation

### `vp doc`

Build documentation using VitePress.

#### Usage

```bash
vp doc [<ARGS>...]
```

#### Examples

```bash
vp doc build
vp doc dev
vp doc dev --host 0.0.0.0
```

## Lint

### `vp lint`

Lint code using oxlint.

#### Usage

```bash
vp lint [<ARGS>...]
```

#### Examples

```bash
vp lint
vp lint --fix
vp lint --quiet
```

## Format

### `vp fmt`

Format code using oxfmt.

#### Usage

```bash
vp fmt [<ARGS>...]
```

#### Examples

```bash
vp fmt
vp fmt --check
vp fmt --ignore-path .gitignore
```

## Testing

### `vp test`

Run tests using Vitest.
By default, `vp test` runs once and exits (equivalent to `vitest run`).
Use watch mode explicitly with `vp test watch`.

#### Usage

```bash
vp test [<ARGS>...]
```

#### Examples

```bash
vp test
vp test watch
vp test run --coverage
```

## Task Runner

### `vp run`

Run tasks across monorepo packages with automatic dependency ordering.

#### Usage

```bash
vp run <TASKS>... [OPTIONS] [-- <TASK_ARGS>...]
```

#### Examples

```bash
# Run build in specific packages
vp run app#build web#build

# Run build recursively across all packages
vp run build --recursive

# Run without topological ordering
vp run build --recursive --no-topological

# Pass arguments to tasks
vp run test -- --watch --coverage
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
      "fingerprintIgnores": ["node_modules/**/*", "!node_modules/**/package.json"]
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
      "fingerprintIgnores": ["node_modules/**/*", "!node_modules/**/package.json"]
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
vp run build
# → Cache miss: no previous cache entry found

# Second run - replays from cache
vp run build
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
vp cache view

# Enable cache debug output
vp run build --debug

# Clean cache
vp cache clean
```

### Environment Variables

| Variable                  | Description                                  |
| ------------------------- | -------------------------------------------- |
| `VITE_LOG`                | Set logging level (e.g., `VITE_LOG=debug`)   |
| `VITE_TASK_EXECUTION_ENV` | Internal: indicates running inside vite task |
| `VITE_TASK_EXECUTION_ID`  | Internal: unique ID for task execution       |

### Execution Modes

#### Explicit Mode (default)

Run specific tasks:

```bash
vp run app#build web#build
```

#### Recursive Mode

Run task in all packages:

```bash
vp run build --recursive
vp run build -r
```

**Behavior:**

- Finds all packages with the specified task
- Runs tasks in dependency order (when `--topological`)
- Default topological ordering for recursive mode

#### Topological Ordering

Controls implicit dependencies based on package relationships:

```bash
# With topological (default for recursive)
vp run build -r
# → If A depends on B, A#build waits for B#build

# Without topological
vp run build -r --no-topological
# → Only explicit dependencies, no implicit ordering
```

### Task Arguments

Pass arguments to tasks using `--`:

```bash
# Arguments go to all tasks
vp run build test -- --watch

# For built-in commands
vp test -- --coverage --run
vp lint -- --fix
```

### Examples

#### Common Workflows

```bash
# Install dependencies
vp install

# Lint and fix issues
vp lint -- --fix

# Format code
vp fmt

# Run build recursively
vp run build -r

# Run tests in watch mode
vp test -- --watch

# Build for production
vp build

# Start dev server
vp dev

# Build library
vp pack

# Build docs
vp doc build

# Preview docs
vp doc dev
```

#### Monorepo Workflows

```bash
# Build all packages in dependency order
vp run build --recursive

# Build specific packages
vp run app#build utils#build

# Run tests without topological ordering
vp run test -r --no-topological

# Clean cache and rebuild
vp cache clean
vp run build -r
```

### Exit Codes

- `0` - Success (all tasks completed successfully)
- `Non-zero` - Failure (one or more tasks failed), or system error (workspace not found, cache error, etc.)

**Note:** When running multiple tasks, the first non-zero exit code is returned.

### Debugging

Enable verbose logging:

```bash
# Debug mode - shows cache operations
vp run build --debug

# Trace logging
VITE_LOG=debug vp run build

# View cache contents
vp cache view
```

## Package Management

### `vp install`

Aliases: `vp i`

Install dependencies using the detected package manager.

#### Usage

```bash
vp install [ARGS] [OPTIONS]
```

#### Examples

```bash
vp install
vp install --loglevel debug
```

#### Note

- Auto-detects package manager (pnpm/yarn/npm)
- Prompts for selection if none package manager detected

### `vp update`

Aliases: `vp up`

Updates packages to their latest version based on the specified range.

#### Usage

```bash
vp update [-g] [<pkg>...]
```

#### Examples

```bash
vp update
vp update @types/node
```

### `vp add`

Installs packages.

#### Usage

```bash
vp add [OPTIONS] <package>[@version]...
```

#### Examples

```bash
vp add -D @types/node
```

### `vp remove`

Aliases: `vp rm`, `vp uninstall`, `vp un`

Removes packages.

#### Usage

```bash
vp remove <package>[@version]...
```

```bash
vp remove @types/node
```

### `vp link`

Aliases: `vp ln`

Makes the current local package accessible system-wide, or in another location.

### `vp unlink`

Unlinks a system-wide package (inverse of `vp link`).

If called without arguments, all linked dependencies will be unlinked inside the current project.

### `vp dedupe`

Perform an install removing older dependencies in the lockfile if a newer version can be used.

### `vp outdated`

Shows outdated packages.

### `vp why`

Aliases: `vp explain`

Shows all packages that depend on the specified package.

### `vp pm <subcommand>`

The `vp pm` command group provides a set of utilities for working with package manager.

> package manager commands with low usage frequency will under this command group.

#### `vp pm prune`

Removes unnecessary packages.

#### `vp pm pack`

Pack the current package into a tarball.

#### `vp pm list`

Aliases: `vp pm ls`

List installed packages.

#### `vp pm view`

View a package info from the registry.

#### `vp pm publish`

Publishes a package to the registry.

#### `vp pm owner`

Manage package owners.

#### `vp pm cache`

Manage the packages metadata cache.

## Others

### `vp optimize`

Pre-bundle dependencies.

**Deprecated**: the pre-bundle process runs automatically and does not need to be called.

#### Usage

```bash
vp optimize [root]
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

### `vp preview`

Locally preview the production build. Do not use this as a production server as it's not designed for it.

This command starts a server in the build directory (by default `dist`). Run `vp build` beforehand to ensure that the build directory is up-to-date. Depending on the project's configured [`appType`](../../config/shared-options.md#apptype), it makes use of certain middleware.

#### Usage

```bash
vp preview [root]
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

### `vp cache`

Manage the task cache.

#### Usage

```bash
vp cache <clean|view>
```

#### `vp cache clean`

Clean up all cached task results.

```bash
vp cache clean
```

#### `vp cache view`

View cache entries in JSON format for debugging.

```bash
vp cache view
```

## See Also

- [Task Configuration](./tasks.md#configuration)
- [Caching Strategy](./caching.md)
- [Monorepo Guide](./monorepo.md)
