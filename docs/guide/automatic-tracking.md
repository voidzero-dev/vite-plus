# Automatic Tracking

Automatic tracking is how Vite Task learns what to cache for a task without explicit configurations. When you run a cache-enabled task, Vite Task observes the task's execution and records what files were read and written, as well as any metadata reported by the task. On the next run, Vite Task decides whether the cache misses or hits based on the recorded fingerprint.

Use this page when you need to understand why a task hits or misses the cache, or when you need to decide whether to add `input`, `output`, `env`, or `untrackedEnv` config.

## Tracking Tiers

Automatic tracking has two tiers:

| Tier                 | Applies to                   | Records                                                                                                                                                                       |
| -------------------- | ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| File system tracking | All tasks with cache enabled | Files read by the command, missing-file probes, directory listings, and written output files                                                                                  |
| Cooperative tracking | Cache-reporting tools        | Metadata reported by the tool, such as environment variables and tool-managed cache paths that should not be considered as inputs or outputs (e.g. `node_modules/.vite-temp`) |

Vite Task starts with file system tracking for any command. A cache-reporting tool can add information that only the tool knows while it runs.

## File System Tracking

File system tracking applies to every task to be cached. If you omit [`input`](/config/run#input), Vite Task tracks the files a command reads while it runs:

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

### Limitations

File system tracking records file access, not every value a process reads. It cannot correctly track environment variable reads, and it cannot always tell which paths are stable inputs, generated outputs, or tool-managed state.

Use [Override Inputs And Outputs](#override-inputs-and-outputs) when automatic file tracking includes files that should not affect the cache, misses files that should, or restores the wrong outputs. Common cases include generated files and directory scans with unrelated files.

Use [`env`](/config/run#env) when an environment variable changes a command's result.

These limitations do not apply to `vp build`: Vite reports [Cooperative Tracking](#cooperative-tracking) metadata automatically, including `VITE_*`, `NODE_ENV`, and tool-managed cache paths that should not become inputs or outputs. A standard `vp build` task does not need manual `input`, `output`, or `env`:

```ts [vite.config.ts]
tasks: {
  build: 'vp build',
}
```

### Override Inputs And Outputs

- [`input`](/config/run#input) controls what invalidates the cache.
- [`output`](/config/run#output) controls which files Vite Task restores on a cache hit.

Both options accept the same kinds of entries and can be configured separately. Omit an option to keep automatic tracking for it. Add `{ auto: true }` to keep automatic tracking while adding glob rules. Use string globs to include paths and `!` globs to exclude paths. Use `[]` to replace automatic tracking with an empty list.

```ts [vite.config.ts]
tasks: {
  build: {
    command: 'node build.mjs',
    input: [{ auto: true }, '!dist', '!dist/**'],
    output: ['dist/**'],
  },
}
```

Use explicit `input` globs only when you can name the command's full, stable input set. This lint task overrides inputs only, so output tracking stays automatic:

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
    input: [{ auto: true }, '!dist', '!dist/**'],
    output: ['dist/**'],
  },
}
```

Set `input: []` when no files should affect the cache key. Set `output: []` when no files should be restored on a cache hit.

## Cooperative Tracking

File system tracking sees access. It cannot always see intent.

`vp build` knows more about a Vite build than Vite Task can infer from file access. When `vp build` runs with cache enabled, Vite reports that metadata to Vite Task. Vite Task merges the report with file system tracking to build a more accurate cache fingerprint.

For a standard Vite build, you do not need to add these entries yourself:

- `env: ['VITE_*']` or `env: ['NODE_ENV']`
- `output: ['dist/**']`
- input or output rules for temporary paths like `node_modules/.vite-temp`

You only need to define the task with `vp build`:

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

Manual config still wins. Add `input`, `output`, or `env` when your project has behavior that Vite cannot know.

Vite+ supports cooperative tracking for `vp build` today. It will extend this support to more first-party tools in the future. Third-party tools can report cache metadata with [`@voidzero-dev/vite-task-client`](https://npmx.dev/package/@voidzero-dev/vite-task-client).

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
