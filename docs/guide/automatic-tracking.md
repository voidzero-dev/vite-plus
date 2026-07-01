# Automatic Data Tracking

Automatic data tracking is how Vite Task learns what inputs a task needs for caching outputs without explicit config.

When you run a cache-enabled task, Vite Task observes the task's execution and records what files were read and written, as well as any metadata reported by the task. On the next run, Vite Task uses the recorded fingerprint to decide whether to replay the cache or run the task.

Use this page when you need to understand why a task hits or misses the cache, or when you need to decide whether to add `input`, `output`, `env`, or `untrackedEnv` config.

## Tracking Tiers

Automatic data tracking has two tiers:

| Tier                 | Applies to                               | Records                                                                                                                                                         |
| -------------------- | ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| File system tracking | All tasks with cache enabled             | <ul><li>Files read by the command</li><li>Missing-file probes</li><li>Directory listings</li><li>Written output files</li></ul>                                 |
| Cooperative tracking | Cache-reporting tools (`vp build` today) | <ul><li>Environment variables reported by the tool</li><li>Tool-managed paths that should not be inputs or outputs, such as `node_modules/.vite-temp`</li></ul> |

Vite Task starts with file system tracking for any command. A cache-reporting tool can add information that only the tool knows while it runs.

## File System Tracking

File system tracking applies to every cache-enabled task. If you omit [`input`](/config/run#input), Vite Task tracks the files a command reads while it runs:

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

For this task, Vite Task records source files, config files, missing files the command checked, and directories the command scanned. Subsequent runs re-run the task when one of those tracked inputs changes.

File system tracking also tracks outputs. If you omit [`output`](/config/run#output), Vite Task archives files the command writes after a successful run and restores them on a cache hit.

### Limitations

Vite Task cannot track environment variable reads, and it cannot always tell which tracked paths are stable inputs, generated outputs, or tool-managed cache paths that should not become inputs or outputs.

Use [Override Inputs And Outputs](#override-inputs-and-outputs) when file system tracking includes files that should not affect the cache, misses files that should, or restores the wrong outputs.

Use [`env`](/config/run#env) when a command needs an environment variable and the value should affect the cache, or [`untrackedEnv`](/config/run#untrackedenv) when the value should not affect the cache.

These limitations do not apply to `vp build`: Vite reports [Cooperative Tracking](#cooperative-tracking) metadata automatically, including `VITE_*`, `NODE_ENV`, and Vite-managed cache paths that should not become inputs or outputs. A standard `vp build` task does not need manual `input`, `output`, or `env`.

### Override Inputs And Outputs

[`input`](/config/run#input) controls what invalidates the cache. [`output`](/config/run#output) controls which files Vite Task restores on a cache hit.

Both options use the same syntax and can be configured separately.

- Omit the option to keep automatic tracking.
- Add `{ auto: true }` to keep automatic tracking while adding glob rules.
- Use string globs to include paths.
- Use `!` globs to exclude paths.
- Use `[]` to replace automatic tracking with an empty list.

```ts [vite.config.ts]
tasks: {
  build: {
    command: 'node build.mjs',

    // Keep automatic input tracking, but exclude `dist` from inputs.
    input: [{ auto: true }, '!dist/**'],

    // Disable automatic output tracking and restore only `dist/**` on a cache hit.
    output: ['dist/**'],
  },
}
```

Use explicit `input` globs only when you know the command's full input set. This lint task overrides inputs only, so output tracking stays automatic:

```ts [vite.config.ts]
tasks: {
  lint: {
    command: 'vp lint',
    // Disable automatic input tracking and fingerprint only these files.
    input: ['src/**', 'vite.config.ts'],
  },
}
```

Set `input: []` when no files should affect the cache fingerprint. This is rarely useful. For example, a download task can be cached when the same URL always serves the same file. No input files should be fingerprinted for this task, but changing the URL still invalidates the cache:

```ts [vite.config.ts]
tasks: {
  downloadSchema: {
    command: 'curl -O https://example.com/schema.json',
    input: [],
  },
}
```

Set `output: []` when no files should be restored on a cache hit.

## Cooperative Tracking

File system tracking records access. It cannot know why a tool used each path.

`vp build` knows more about a Vite build than Vite Task can infer from file access. When `vp build` runs with cache enabled, Vite reports that metadata to Vite Task. Vite Task merges the report with file system tracking to build a more accurate cache fingerprint.

For a standard Vite build, you do not need to add these entries yourself because Vite reports them automatically at runtime:

- `env: ['VITE_*']` or `env: ['NODE_ENV']`
- `output: ['dist/**']`
- input or output exclusions for temporary paths like `node_modules/.vite-temp`

You only need to define the task with `vp build`:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  run: {
    tasks: {
      frontendBuild: 'vp build',
    },
  },
});
```

Run this task with `vpr frontendBuild` or `vp run frontendBuild`.

Manual config overrides reported metadata. Add `input`, `output`, `env`, or `untrackedEnv` when your project has behavior that Vite cannot report.

Vite+ supports cooperative tracking for `vp build` today. It will extend this support to more first-party tools in the future. Third-party tools can report cache metadata with [`@voidzero-dev/vite-task-client`](https://npmx.dev/package/@voidzero-dev/vite-task-client).

## When To Add Manual Config

Add config when your project has behavior the command or tool cannot know.

| Case                                                              | Example                                                                                         |
| ----------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Exclude an output directory from inputs                           | `input: [{ auto: true }, '!dist/**']`                                                           |
| Exclude a temporary generated file from input and output tracking | `input: [{ auto: true }, '!.tmp/config.mjs']`<br>`output: [{ auto: true }, '!.tmp/config.mjs']` |
| Avoid automatic file tracking for a task                          | `input: ['src/**']`<br>`output: ['dist/**']`                                                    |
| Track and pass an env var                                         | `env: ['NODE_ENV']`                                                                             |
| Pass an env var without fingerprinting it                         | `untrackedEnv: ['GITHUB_ACTIONS']`                                                              |
