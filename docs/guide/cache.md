# Task Caching

Vite Task can automatically track dependencies and cache tasks run through `vp run`.

## Overview

When a task runs successfully (exit code 0), its terminal output (stdout/stderr) and all files it writes (output files) are saved. On the next run, Vite Task checks if anything changed:

1. **Arguments:** did the [additional arguments](/guide/run#additional-arguments) passed to the task change?
2. **Environment variables:** did any [fingerprinted env vars](/config/run#env) change?
3. **Inputs:** did any input file that the command reads change?

When all checks match, Vite Task replays the cached terminal output, restores saved output files, and skips the command.

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

`--no-cache` disables caching for every task and script in that run. `--cache` enables caching for both tasks and scripts, which is equivalent to setting [`run.cache: true`](/config/run#run-cache) for that invocation.

### 3. Workspace config

The [`run.cache`](/config/run#run-cache) option in your root `vite.config.ts` controls the default for each category:

| Setting         | Default | Effect                                  |
| --------------- | ------- | --------------------------------------- |
| `cache.tasks`   | `true`  | Cache tasks defined in `vite.config.ts` |
| `cache.scripts` | `false` | Cache `package.json` scripts            |

## Automatic Data Tracking

Vite Task uses [automatic data tracking](/guide/automatic-tracking) to learn what each task needs for caching so you don't have to configure it manually. Automatic data tracking has two tiers:

- **File system tracking:** Vite Task records file reads, missing-file probes, directory listings, and written output files for every task with cache enabled.
- **Cooperative tracking:** cache-reporting tools can report metadata that file system tracking cannot infer. Vite+ supports this for `vp build` today.

Use [`input`](/config/run#input) or [`output`](/config/run#output) when a task needs manual tracking rules. `input` controls what invalidates the cache. `output` controls which files Vite Task restores on a cache hit.

```ts [vite.config.ts]
tasks: {
  build: {
    command: 'node build.mjs',
    input: [{ auto: true }, '!dist/**'],
    output: ['dist/**'],
  },
}
```

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

To pass a variable to the task **without** affecting cache behavior, use [`untrackedEnv`](/config/run#untrackedenv). This is useful for variables like `CI` or `GITHUB_ACTIONS` that should be available in the task, but do not affect caching behavior.

See [Run Config](/config/run#env) for details on wildcard patterns and the full list of automatically passed-through variables.

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

The task cache is stored in `node_modules/.vite/task-cache` at the project root. `vp cache clean` deletes that cache directory.
