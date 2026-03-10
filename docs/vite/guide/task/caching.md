# Caching

## How It Works {#how-caching-works}

When a task runs successfully (exit code 0), its terminal output (stdout/stderr) is saved. On the next run, the task runner checks if anything changed:

1. **Arguments** — did the [additional arguments](./cli#additional-arguments) passed to the task change?
2. **Environment variables** — did any [fingerprinted env vars](./config#envs) change?
3. **Input files** — did any file that the command reads change?

If everything matches, the cached output is replayed instantly — the command never actually runs.

::: info
Currently, only terminal output is cached and replayed. Output files (e.g., `dist/`) are not cached — if you delete them, use `--no-cache` to force a re-run. Output file caching is planned for a future release.
:::

When a cache miss occurs, the task runner tells you exactly why:

```
$ vp lint ✗ cache miss: 'src/utils.ts' modified, executing
$ vp build ✗ cache miss: envs changed, executing
$ vp test ✗ cache miss: args changed, executing
```

## When Is Caching Enabled? {#when-is-caching-enabled}

A command run by `vp run` is either a **task** (has an entry in `vite.config.ts`) or a **script** (only exists in `package.json` with no corresponding task entry). By default, **tasks are cached and scripts are not.** Three layers control whether caching is on:

### 1. Per-task `cache: false` (highest priority, tasks only)

A task can set [`cache: false`](./config#cache) to opt out. This cannot be overridden by `--cache` or `run.cache` — if a task says no caching, it means no caching.

### 2. CLI flags

`--no-cache` disables caching for everything. `--cache` enables caching for both tasks and scripts — equivalent to setting [`run.cache: true`](./config#run-cache) for that invocation.

### 3. Workspace config

The [`run.cache`](./config#run-cache) option in your root `vite.config.ts` controls the default for each category:

| Setting         | Default | Effect                                |
| --------------- | ------- | ------------------------------------- |
| `cache.tasks`   | `true`  | Cache commands that have a task entry |
| `cache.scripts` | `false` | Cache plain `package.json` scripts    |

Use `--cache` to quickly enable script caching, or set `run.cache.scripts: true` in config to enable it permanently.

## Automatic File Tracking {#automatic-file-tracking}

By default, the task runner tracks which files each command reads during execution. When `vp build` runs, it records which files the process opens — your `.ts` source files, `vite.config.ts`, `package.json`, etc. — and records their content hashes. On the next run, it re-checks those hashes to determine if anything changed.

This means caching works out of the box for most commands without any configuration. The tracker also records:

- **File non-existence** — if a command probes for a file that doesn't exist (e.g., `utils.ts` during module resolution), creating that file later correctly invalidates the cache.
- **Directory listings** — if a command scans a directory (e.g., a test runner looking for `*.test.ts`), adding or removing files in that directory invalidates the cache.

### Over-fingerprinting {#over-fingerprinting}

Automatic tracking can sometimes include more files than necessary, causing unnecessary cache misses:

- **Tool cache files** — some tools maintain their own cache (e.g., TypeScript's `.tsbuildinfo`, Cargo's `target/`). These files may change between runs even when your source code hasn't, causing unnecessary cache invalidation.
- **Directory listings** — when a command scans a directory (e.g., globbing for `**/*.js`), the task runner sees the directory read but not the glob pattern. Any file added or removed in that directory — even unrelated ones — invalidates the cache.

Use the [`inputs`](./config#inputs) option to exclude noisy files or replace automatic tracking with explicit file patterns:

```ts
tasks: {
  build: {
    command: 'tsc',
    inputs: [{ auto: true }, '!**/*.tsbuildinfo'],
  },
}
```

## Environment Variables {#environment-variables}

By default, tasks run in a clean environment — only a small set of common variables (like `PATH`, `HOME`, `CI`) are passed through. Other environment variables are neither visible to the task nor included in the cache fingerprint.

To make a variable affect caching, add it to [`envs`](./config#envs). Changing its value invalidates the cache:

```ts
tasks: {
  build: {
    command: 'webpack --mode production',
    envs: ['NODE_ENV'],
  },
}
```

To pass a variable to the task **without** affecting the cache, use [`passThroughEnvs`](./config#pass-through-envs). This is useful for variables like `CI` or `GITHUB_ACTIONS` that should be available but shouldn't trigger a rebuild when they change.

See the [config reference](./config#envs) for details on wildcard patterns and the full list of automatically passed-through variables.

## Cache Sharing {#cache-sharing}

The cache is content-based — if two tasks run the same command with the same inputs, they share the cache entry. This happens naturally when multiple tasks include a common step, either as standalone tasks or as parts of [compound commands](./running-tasks#compound-commands):

```json [package.json]
{
  "scripts": {
    "check": "vp lint && vp build",
    "release": "vp lint && deploy-script"
  }
}
```

With caching enabled (e.g. `--cache` or [`run.cache.scripts: true`](./config#run-cache)), running `check` first means the `vp lint` step in `release` is an instant cache hit, since both run the same command against the same files.

## Clearing the Cache {#clearing-the-cache}

```bash
vp cache clean
```

This deletes the entire cache directory. Tasks will run fresh on the next invocation.
