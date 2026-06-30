# Automatic Tracking

Automatic tracking is how Vite Task learns whether a cached task can be reused. When you run a cached command through `vp run`, Vite Task records the files, outputs, and cache-reporting metadata that affect the result. On the next run, Vite Task compares that record with the current command and replays the task only when the fingerprint still matches.

Use this page when you need to understand why a task hits or misses the cache, or when you need to decide whether to add `input`, `output`, `env`, or `untrackedEnv` config.

## Tracking Tiers

Automatic tracking has two tiers:

| Tier                 | Applies to                            | Records                                                                                                                  |
| -------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| File system tracking | All cached tasks run through `vp run` | Files read by the command, missing-file probes, directory listings, and written output files                             |
| Cooperative tracking | Cache-reporting tools                 | Metadata reported by the tool, such as environment variables and tool-owned paths that should not affect the task result |

Vite Task starts with file system tracking for any command. A cache-reporting tool can add information that only the tool knows while it runs.

## File System Tracking

File system tracking applies to every cached task run through `vp run`. If you omit [`input`](/config/run#input), Vite Task tracks the files a command reads while it runs:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  run: {
    tasks: {
      build: {
        command: 'tsc',
      },
    },
  },
});
```

For this task, Vite Task records source files, config files, missing files the command checked, and directories the command scanned. A later run misses the cache when one of those tracked inputs changes.

File system tracking also tracks outputs. If you omit [`output`](/config/run#output), Vite Task archives files the command writes after a successful run and restores them on a cache hit.

### Adjust File System Tracking

Keep file system tracking and add exclusions when a command reads generated files that should not invalidate the task:

```ts [vite.config.ts]
tasks: {
  build: {
    command: 'tsc',
    input: [{ auto: true }, '!**/*.tsbuildinfo', '!dist', '!dist/**'],
  },
}
```

Use explicit `input` globs only when the command has a small, stable input set you can name:

```ts [vite.config.ts]
tasks: {
  lint: {
    command: 'vp lint',
    input: ['package.json', 'pnpm-lock.yaml', 'src/**', 'tsconfig*.json'],
  },
}
```

Use explicit `output` globs when you want to narrow or extend the files Vite Task restores:

```ts [vite.config.ts]
tasks: {
  build: {
    command: 'node build.mjs',
    output: ['dist/**'],
  },
}
```

Set `input: []` to disable file tracking for a task. Set `output: []` to disable output restoration. The task can still use cache entries based on command arguments and tracked environment variables.

## Cooperative Tracking

Some tools know cache dependencies that file system tracking cannot infer from the outside. A cache-reporting tool can cooperate with Vite Task while it runs. Vite Task still observes file reads and writes. It uses the report to refine the cache fingerprint.

Vite+ supports cooperative tracking for `vp build` today. When a task runs `vp build`, Vite reports build cache metadata to Vite Task. For a standard Vite build, you do not need to add these entries yourself:

- `env: ['VITE_*']` or `env: ['NODE_ENV']`
- `output: ['dist/**']`
- explicit input globs that replace file system tracking

Define the build as a `vp run` task:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  run: {
    tasks: {
      build: 'vp build',
    },
  },
});
```

Then run it with:

```bash
vp run build
```

Direct `vp build` runs a build, but it does not use the Vite Task cache. Use `vp run build` when you want task caching.

Vite+ will extend cooperative tracking to more first-party tools in the future. Third-party tools can report cache metadata with [`@voidzero-dev/vite-task-client`](https://npmx.dev/package/@voidzero-dev/vite-task-client).

## When To Add Manual Config

Add config when your project has behavior the command or tool cannot know.

| Goal                                                     | Config                                         |
| -------------------------------------------------------- | ---------------------------------------------- |
| Keep file system tracking and add CI dependency identity | `input: [{ auto: true }, 'pnpm-lock.yaml']`    |
| Exclude generated output from the input fingerprint      | `input: [{ auto: true }, '!dist', '!dist/**']` |
| Replace file system tracking for a small input set       | `input: ['src/**', 'package.json']`            |
| Narrow output restoration                                | `output: ['dist/**']`                          |
| Track an env var used by a non-reporting command         | `env: ['NODE_ENV']`                            |
| Pass an env var without fingerprinting it                | `untrackedEnv: ['GITHUB_ACTIONS']`             |

For CI builds, keep automatic tracking for `vp build` and add only the extra project facts CI needs:

```ts [vite.config.ts]
tasks: {
  'ci-build': {
    command: 'vp build',
    input: [{ auto: true }, 'pnpm-lock.yaml', '!dist', '!dist/**'],
    output: ['dist/**'],
  },
}
```

This task keeps file system tracking, lets Vite report build metadata, adds the lockfile to the fingerprint, excludes restored output files from inputs, and restores only `dist/**` on a cache hit.

## Common Cache Miss Causes

Vite Task chooses a cache miss when tracked data changed between runs. In CI, these data sources often explain misses after a restored cache:

- **Dependency install metadata:** add committed package manifests and lockfiles as inputs. For `vp build`, keep automatic tracking and add the lockfile.
- **Tool-owned incremental state:** move incremental files into a generated cache directory or exclude them from file system-tracked inputs.
- **Generated outputs:** exclude output directories from `input`, then configure `output` so Vite Task can restore them.
- **Broad directory scans:** use explicit `input` globs when a command scans directories that contain unrelated files.
- **Environment variables:** add `env` for variables that change a non-reporting command's result. Leave standard Vite build env vars out of `env` because `vp build` reports them.

For GitHub Actions cache reuse, see [GitHub Actions Cache](/guide/github-actions-cache). That guide explains how to restore `node_modules/.vite/task-cache` between CI runs after your task hits locally.
