# Task Caching

Vite Task tracks dependencies, caches terminal output, and restores output files for cached tasks run through `vp run`.

## Overview

When a task exits with code 0, Vite Task saves its terminal output and any tracked output files. On the next run, Vite Task checks whether the cache entry still matches:

1. **Arguments:** did the [additional arguments](/guide/run#additional-arguments) passed to the task change?
2. **Environment variables:** did any [fingerprinted env vars](/config/run#env) change?
3. **Input files:** did any file that the command reads change?

If the cache entry matches, Vite Task replays the terminal output, restores tracked output files, and skips the command.

## Output Restoration

Vite Task tracks files that a task writes. On a cache hit, Vite Task restores those files from the cache.

For example, a cached `vp build` can restore `dist` after you delete it:

```bash
vp run build
rm -rf dist
vp run build # cache hit; Vite Task restores dist
```

Use [`output`](/config/run#output) when you need to customize file restoration:

- Omit `output` to restore files that the task writes.
- Add glob patterns to restore a specific set of files.
- Add `{ auto: true }` with globs to combine automatic write tracking with explicit patterns.
- Set `output: []` to disable output restoration.

When a cache miss occurs, Vite Task tells you exactly why:

```
$ vp lint ✗ cache miss: 'src/utils.ts' modified, executing
$ vp build ✗ cache miss: env 'VITE_GREETING' changed, executing
$ vp test ✗ cache miss: args changed, executing
```

## When Is Caching Enabled?

A command run by `vp run` is either a **task** defined in `vite.config.ts` or a **script** defined in `package.json`. Task names and script names cannot overlap. By default, **tasks are cached and scripts are not.**

There are three types of controls for task caching, in order:

### 1. Per-task `cache: false`

A task can set [`cache: false`](/config/run#cache) to opt out. This cannot be overridden by any other cache control flag.

### 2. CLI flags

`--no-cache` disables caching for everything. `--cache` enables caching for both tasks and scripts, which is equivalent to setting [`run.cache: true`](/config/run#run-cache) for that invocation.

### 3. Workspace config

The [`run.cache`](/config/run#run-cache) option in your root `vite.config.ts` controls the default for each category:

| Setting         | Default | Effect                                  |
| --------------- | ------- | --------------------------------------- |
| `cache.tasks`   | `true`  | Cache tasks defined in `vite.config.ts` |
| `cache.scripts` | `false` | Cache `package.json` scripts            |

## Automatic File Tracking

Vite Task tracks which files each command reads during execution. When a task runs, it records which files the process opens, such as your `.ts` source files, `vite.config.ts`, and `package.json`, and records their content hashes. On the next run, it re-checks those hashes to determine if anything changed.

This means caching works out of the box for most commands without any configuration. Vite Task also records:

- **Missing files:** if a command probes for a file that doesn't exist, such as `utils.ts` during module resolution, creating that file later correctly invalidates the cache.
- **Directory listings:** if a command scans a directory, such as a test runner looking for `*.test.ts`, adding or removing files in that directory invalidates the cache.

### Avoiding Overly Broad Input Tracking

Automatic tracking can sometimes include more files than necessary, causing unnecessary cache misses:

- **Tool cache files:** some tools maintain their own cache, such as TypeScript's `.tsbuildinfo` or Cargo's `target/`. These files may change between runs even when your source code has not, causing unnecessary cache invalidation.
- **Directory listings:** when a command scans a directory, such as when globbing for `**/*.js`, Vite Task sees the directory read but not the glob pattern. Any file added or removed in that directory, even unrelated ones, invalidates the cache.

Use the [`input`](/config/run#input) option to exclude files or to replace automatic tracking with explicit file patterns:

```ts [vite.config.ts]
tasks: {
  build: {
    command: 'tsc',
    input: [{ auto: true }, '!**/*.tsbuildinfo'],
  },
}
```

### Runner-Aware Tools

Tools can report cache information to Vite Task while they run. Vite uses this runner protocol during `vp build` to report inputs, outputs, and Vite environment variables. You do not need to declare `env: ['VITE_*']` or `output: ['dist/**']` for a standard Vite build.

## Environment Variables

By default, tasks run in a clean environment. Only a small set of common variables, such as `PATH`, `HOME`, and `CI`, are passed through. Other environment variables are neither visible to the task nor included in the cache fingerprint.

To add an environment variable to the cache key, add it to [`env`](/config/run#env). Changing its value then invalidates the cache:

```ts [vite.config.ts]
tasks: {
  build: {
    command: 'webpack --mode production',
    env: ['NODE_ENV'],
  },
}
```

To pass a variable to the task **without** affecting cache behavior, use [`untrackedEnv`](/config/run#untracked-env). This is useful for variables like `CI` or `GITHUB_ACTIONS` that should be available in the task, but do not generally affect caching behavior.

See [Run Config](/config/run#env) for details on wildcard patterns and the full list of default passed-through variables.

## Cache Sharing

Vite Task's cache is content-based. If two tasks run the same command with the same inputs, they share the cache entry. This happens naturally when multiple tasks include a common step, either as standalone tasks or as parts of [compound commands](/guide/run#compound-commands):

```json [package.json]
{
  "scripts": {
    "check": "vp lint && vp build",
    "release": "vp lint && deploy-script"
  }
}
```

With caching enabled, for example through `--cache` or [`run.cache.scripts: true`](/config/run#run-cache), running `check` first means the `vp lint` step in `release` is an instant cache hit, since both run the same command against the same files.

## Cache Commands

Use `vp cache clean` when you need to clear cached task results:

```bash
vp cache clean
```

Vite Task stores cache files under `node_modules/.vite/task-cache` at the project root. Vite Task keeps separate subdirectories for cache schema versions, and `vp cache clean` deletes the cache directory.

## Reusing the Cache Across CI Runs

::: warning Experimental
Reusing the task cache across CI runs is experimental. The on-disk cache format and the behaviors described here can change between Vite+ versions. A restored cache that no longer matches is always re-run, so a stale or incompatible cache costs time, never correctness.
:::

The cache normally lives in your working tree and disappears when CI checks out a fresh copy, so every run starts cold. Because Vite Task's cache is [content-based](#cache-sharing), you can persist the cache directory between runs and Vite Task replays any task whose inputs have not changed — turning a clean CI checkout into a warm one.

On GitHub Actions, [`actions/cache`](https://github.com/actions/cache) handles the persistence. Two layers cooperate, and they decide independently:

1. **`actions/cache`** restores and saves the `node_modules/.vite/task-cache` directory, keyed by a string you choose.
2. **Vite Task** then decides, per task, whether the restored cache matches the current inputs.

`actions/cache` only ferries the directory between runs; Vite Task does the real hit/miss logic. The cache is portable across machines and checkout paths because its keys are content hashes of workspace-relative paths, with no absolute paths, hostname, or OS baked in.

### Step by step

First, make sure the commands you want to cache run through [`vp run`](/guide/run) as tasks. Only `vp run` applies task caching — `vp build` on its own is not cached. Define the task in `vite.config.ts` so it is cached by default:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  run: {
    tasks: {
      build: {
        command: 'vp build',
        // See "Keep inputs stable across runs" below.
        input: [{ auto: true }, '!node_modules/**'],
      },
    },
  },
});
```

Then add an `actions/cache` step to your workflow. If you use [`voidzero-dev/setup-vp`](/guide/ci) to install Vite+, its `cache: true` only caches your **dependencies** — the task cache is separate and needs its own step:

```yaml [.github/workflows/ci.yml]
name: CI
on: [push]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      # Installs Vite+, Node, your package manager, and runs `vp install`.
      # `cache: true` caches dependencies, not task results.
      - uses: voidzero-dev/setup-vp@v1
        with:
          node-version: '24'
          cache: true

      # Persist the Vite Task cache so task results carry over between runs.
      # Runs after install because the cache lives inside node_modules.
      - name: Cache Vite Task results
        uses: actions/cache@v4
        with:
          path: node_modules/.vite/task-cache
          key: vite-task-${{ runner.os }}-${{ github.sha }}
          restore-keys: |
            vite-task-${{ runner.os }}-

      - run: vp run build
```

The first run finds no cache, executes `build`, and saves the directory:

```
Cache not found for input keys: vite-task-Linux-<sha>, vite-task-Linux-
$ vp build
✓ built in 180ms
Cache saved with key: vite-task-Linux-<sha>
```

A later run restores it through the `restore-keys` prefix and replays the task instead of rebuilding:

```
Cache restored from key: vite-task-Linux-<sha-of-previous-run>
$ vp build ◉ cache hit, replaying
vp run: cache hit, 466ms saved.
```

If you do not use `setup-vp`, the same `actions/cache` step works with any toolchain. Just place it after your dependency install, and invoke Vite+ through your package manager (for example `pnpm exec vp run build`).

### Choosing a cache key

A GitHub Actions cache entry is **immutable**: once a key is written it is never overwritten. A fixed key would freeze the cache after the first run, so it would never pick up new results. Use a **rolling key plus a prefix fallback** instead:

```yaml [.github/workflows/ci.yml]
key: vite-task-${{ runner.os }}-${{ github.sha }}
restore-keys: |
  vite-task-${{ runner.os }}-
```

- `key` is unique per commit (`github.sha`), so every run saves a fresh entry — the cache keeps moving forward.
- `restore-keys` is matched as a prefix. When the exact `key` is absent (the usual case on a new commit), GitHub restores the **most recent** entry beginning with `vite-task-<os>-`.

Include `runner.os` in the key so a `ubuntu` cache is never restored onto a `windows` job. Keep one cache per task graph; the directory is small, so there is no need to split it per task.

### Keep inputs stable across runs

Vite Task's [automatic input tracking](#automatic-file-tracking) records the files a command reads, the directories it scans, and the files it probes for. In CI some of those differ between runs without being real inputs, which produces spurious misses. Two patterns come up most often.

**Files whose content changes between runs.** The common case is pnpm's `node_modules/.modules.yaml`, which is rewritten whenever the dependency cache is restored, so a build that reads it misses on the next run with `cache miss: 'node_modules/.modules.yaml' modified`. Exclude `node_modules` from the task's inputs — dependency identity is still tracked through your committed lockfile:

```ts [vite.config.ts]
build: {
  command: 'vp build',
  input: [{ auto: true }, '!node_modules/**'],
}
```

**Tool-owned files written into the tracked tree.** Incremental caches such as TypeScript's `.tsbuildinfo` are written during the run but are usually gitignored, so they exist when the cache is saved and are absent on the next fresh checkout. Because the tool also scans the directory that holds them, the directory listing changes and the task misses with `'tsconfig.tsbuildinfo' removed from workspace root`. A file-glob exclusion like `!**/*.tsbuildinfo` does **not** fix this: it suppresses the file's content, but not its appearance in the scanned directory listing. Make the tool write the file somewhere already excluded instead, such as under `node_modules`:

```ts [vite.config.ts]
typecheck: {
  command: 'tsc --noEmit --tsBuildInfoFile node_modules/.cache/tsc/typecheck.tsbuildinfo',
  input: [{ auto: true }, '!node_modules/**'],
}
```

When a task still misses, the [cache miss reason](#output-restoration) names the path that changed. If its content is noisy, exclude it; if it is a tool-owned file being tracked through a directory listing, relocate it out of the tracked tree.

### When a task still misses

Some commands embed an absolute path in the executed command line. `vp build`, for example, runs the bundled Vite CLI by its absolute path, which becomes part of the task's cache key. This path is stable as long as the **checkout location is stable**, which is the normal case on GitHub-hosted runners: a given repository always checks out to `/home/runner/work/<repo>/<repo>`. The cache reuses cleanly across runs there.

It can miss when the absolute checkout path differs between runs — for example self-hosted runners with a rotating work directory, or jobs that run from different paths. Plain `node`/script tasks do not have this limitation; their keys are fully path-independent. To keep `vp build` cacheable, run it from a consistent working directory.

### Invalidate on toolchain changes

Vite Task stores its cache under a schema-versioned subdirectory and ignores entries written by an incompatible version, so upgrading Vite+ is safe — it just starts cold. To avoid carrying stale subdirectories around in the saved archive, fold your lockfile hash into the key so a dependency or toolchain change starts from a clean cache:

```yaml [.github/workflows/ci.yml]
key: vite-task-${{ runner.os }}-${{ hashFiles('pnpm-lock.yaml') }}-${{ github.sha }}
restore-keys: |
  vite-task-${{ runner.os }}-${{ hashFiles('pnpm-lock.yaml') }}-
```

### Limitations and workarounds

GitHub Actions caching has constraints that shape how much reuse you actually get:

- **Storage limit and eviction.** Each repository has a default **10 GB** cache budget (raisable on paid plans). When the budget is exceeded, GitHub evicts entries **least-recently-used first**. The task cache directory is small, so this is rarely the binding constraint unless you cache many other things alongside it.
- **7-day expiry.** Caches **not accessed for 7 days are removed**. On a quiet repository the cache can disappear between runs; the next run simply rebuilds and repopulates it.
- **Immutable keys.** A key is written once and never updated — handled by the [rolling key](#choosing-a-cache-key) pattern above.
- **Branch scoping.** A run can restore caches created by **its own branch** and by the repository's **default branch**, but not caches from sibling branches. To warm new branches and pull requests, run CI on your default branch so its cache becomes the shared baseline. Pull requests from forks have read-only, restricted cache access by design.

When the cache grows stale or you change task definitions, clear it locally with `vp cache clean`; in CI, the 7-day and LRU policies reclaim space automatically, or you can prune entries with a scheduled cleanup workflow.
