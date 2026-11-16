# Automatic Data Tracking

Automatic data tracking is how Vite Task learns what inputs a task needs for caching outputs without explicit config.

When you run a cache-enabled task, Vite Task observes the task's execution and records what files were read and written, as well as any metadata reported by the task. On the next run, Vite Task uses the recorded fingerprint to decide whether to replay the cache or run the task.

Use this page when you need to understand why a task hits or misses the cache, or when you need to decide whether to add `cache.input`, `cache.output`, `cache.env`, or `cache.untrackedEnv` config.

## Tracking Tiers

Automatic data tracking has two tiers:

| Tier                 | Applies to                                    | Records                                                                                                                                                         |
| -------------------- | --------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| File system tracking | All tasks with cache enabled                  | <ul><li>Files read by the command</li><li>Missing-file probes</li><li>Directory listings</li><li>Written output files</li></ul>                                 |
| Cooperative tracking | Cache-reporting tools (`vp build`, `vp test`) | <ul><li>Environment variables reported by the tool</li><li>Tool-managed paths that should not be inputs or outputs, such as `node_modules/.vite-temp`</li></ul> |

Vite Task starts with file system tracking for any command. A cache-reporting tool can add information that only the tool knows while it runs.

## File System Tracking

File system tracking applies to every cache-enabled task. If you omit [`cache.input`](/config/run#cache-input), Vite Task tracks the files a command reads while it runs:

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

File system tracking also tracks outputs. If you omit [`cache.output`](/config/run#cache-output), Vite Task archives files the command writes after a successful run and restores them on a cache hit.

### Rebuildable Inputs

Some tools read files they also write:

- With [`exports`](https://tsdown.dev/options/package-exports) enabled, tsdown reads `package.json` and writes the generated `exports` field back into it.
- Many build tools list `dist` to delete the old build before they write a new one there.
- Tools with their own cache read it and write it back, as `tsc --incremental` does with `tsconfig.tsbuildinfo`.

This section explains how Vite Task handles files and folders that a task both reads and writes.

Vite Task calls these files and folders **rebuildable**: the task produces them itself, so they don't have to exist before it runs. A task can rebuild:

- Files it writes, such as `dist/index.mjs` or the `package.json` that tsdown updates.
- Folders where the task wrote everything inside, such as `dist`.

::: warning
Rebuildable inputs assume that a task doesn't keep changing its outputs when its inputs stay the same.

For example, a script that appends data to a file on every run breaks this assumption, and Vite Task can't detect it. Vite Task caches the file as the script left it, so the next run is a cache hit that restores the file without appending anything.
:::

After the task finishes, Vite Task fingerprints its inputs as the task left them. On the next run:

- A rebuildable input may be missing, as on a fresh clone or after you delete it. The cache still hits and restores it with the task's outputs.
- A rebuildable file that exists must match what the task left. If something else changed it, the cache misses.
- Inputs that aren't rebuildable, such as your source files, are still checked strictly. Deleting or editing one causes a cache miss.

#### Example: A Task Rewriting Sources

```ts [vite.config.ts]
tasks: {
  build: {
    // `--exports` makes tsdown write the generated `exports` field into package.json.
    command: 'tsdown --exports',
  },
}
```

```
my-lib/
├── src/
│   └── index.ts             # input: tsdown reads it
└── package.json             # rebuildable: tsdown reads it, then writes "exports" into it
```

On the next run, Vite Task compares `package.json` with the file tsdown wrote, so the `exports` field that tsdown added doesn't cause a cache miss:

```bash
$ vp run build               # first run: tsdown adds "exports" to package.json
$ vp run build               # cache hit
```

Editing `package.json` yourself still causes a cache miss, as it should, because tsdown has to run again with your changes:

```bash
$ vp pm pkg set version=1.1.0
$ vp run build               # cache miss: 'package.json' modified
```

#### Example: A Task Listing Its Output Folder For Deletion

```ts [vite.config.ts]
tasks: {
  build: {
    // `--clean` deletes the old build in `dist` before tsdown writes a new one.
    command: 'tsdown --clean',
  },
}
```

```
my-lib/
├── src/
│   └── index.ts             # input
├── package.json             # input
└── dist/                    # rebuildable: the task wrote everything inside
    └── index.mjs            # rebuildable: the task writes it
```

Listing `dist` makes it an input. Since the task wrote everything inside `dist`, the whole folder is rebuildable:

```bash
$ vp run build               # first run: tsdown writes dist/
$ rm -rf dist                # as on a fresh clone
$ vp run build               # cache hit: dist/ is restored
```

You don't need `'!dist/**'` in `cache.input` to keep `dist` out of the task's inputs, even though the task lists `dist` to delete the old build and then writes a new one.

#### Example: A Task Reading And Writing Its Own Cache

```ts [vite.config.ts]
tasks: {
  typecheck: {
    command: 'tsc --incremental --noEmit',
  },
}
```

```
my-app/
├── src/
│   └── main.ts              # input: tsc reads it
├── tsconfig.json            # input: tsc reads it
└── tsconfig.tsbuildinfo     # rebuildable: tsc reads its cache from it, then writes it back
```

Vite Task compares `tsconfig.tsbuildinfo` with the file tsc wrote, so tsc's updates to it don't cause a cache miss. If the file is missing, as on a fresh clone, a cache hit restores it:

```bash
$ vp run typecheck           # first run: tsc writes tsconfig.tsbuildinfo
$ vp run typecheck           # cache hit
$ rm tsconfig.tsbuildinfo    # as on a fresh clone
$ vp run typecheck           # cache hit: tsconfig.tsbuildinfo is restored
```

#### Avoid Caching Tasks That Modify Files In Place

A task that rewrites your source files makes them rebuildable. If a cached `vp fmt` rewrites every file in `src/utils`, the whole folder becomes rebuildable:

```
src/
└── utils/                   # rebuildable: vp fmt rewrote everything inside
    ├── a.ts                 # rebuildable: vp fmt rewrote it
    └── b.ts                 # rebuildable: vp fmt rewrote it
```

Deleting `src/utils` is then a cache hit that writes the folder back.

Turn off auto-fix in the lint and format tasks you cache:

```ts [vite.config.ts]
tasks: {
  lint: 'vp lint', // instead of `vp lint --fix`
  format: 'vp fmt --check', // instead of `vp fmt`
}
```

### Limitations

Vite Task cannot track environment variable reads, and it cannot always tell which tracked paths are stable inputs, generated outputs, or tool-managed cache paths that should not become inputs or outputs.

Use [Override Inputs And Outputs](#override-inputs-and-outputs) when file system tracking includes files that should not affect the cache, misses files that should, or restores the wrong outputs.

Use [`cache.env`](/config/run#cache-env) when a command needs an environment variable and the value should affect the cache, or [`cache.untrackedEnv`](/config/run#cache-untrackedenv) when the value should not affect the cache.

These limitations do not apply to `vp build`: Vite reports [Cooperative Tracking](#cooperative-tracking) metadata automatically, including `VITE_*`, `NODE_ENV`, and Vite-managed cache paths that should not become inputs or outputs. A standard `vp build` task does not need manual `input`, `output`, or `env`.

### Override Inputs And Outputs

[`cache.input`](/config/run#cache-input) controls what invalidates the cache. [`cache.output`](/config/run#cache-output) controls which files Vite Task restores on a cache hit.

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
    cache: {
      // Keep automatic input tracking, but exclude `dist` from inputs.
      input: [{ auto: true }, '!dist/**'],

      // Disable automatic output tracking and restore only `dist/**` on a cache hit.
      output: ['dist/**'],
    },
  },
}
```

Use explicit `input` globs only when you know the command's full input set. This lint task overrides inputs only, so output tracking stays automatic:

```ts [vite.config.ts]
tasks: {
  lint: {
    command: 'vp lint',
    cache: {
      // Disable automatic input tracking and fingerprint only these files.
      input: ['src/**', 'vite.config.ts'],
    },
  },
}
```

Set `input: []` when no files should affect the cache fingerprint. This is rarely useful. For example, a download task can be cached when the same URL always serves the same file. No input files should be fingerprinted for this task, but changing the URL still invalidates the cache:

```ts [vite.config.ts]
tasks: {
  downloadSchema: {
    command: 'curl -O https://example.com/schema.json',
    cache: {
      input: [],
    },
  },
}
```

Set `output: []` when no files should be restored on a cache hit.

## Cooperative Tracking

File system tracking records access. It cannot know why a tool used each path.

`vp build` knows more about a Vite build than Vite Task can infer from file access. When `vp build` runs with cache enabled, Vite reports that metadata to Vite Task. Vite Task merges the report with file system tracking to build a more accurate cache fingerprint.

For a standard Vite build, you do not need to add these entries yourself because Vite reports them automatically at runtime:

- `env: ['VITE_*']` or `env: ['NODE_ENV']`
- input or output exclusions for temporary paths like `node_modules/.vite-temp`

Build outputs such as `dist` are captured by automatic file-write tracking and restored on cache hits.

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

Manual config overrides reported metadata. Add `cache.input`, `cache.output`, `cache.env`, or `cache.untrackedEnv` when your project has behavior that Vite cannot report.

Vite+ supports cooperative tracking for `vp build` and `vp test`. `vp test` reports Vitest's on-disk cache paths (`node_modules/.vitest-cache` and the results cache under the Vite cache directory) so they neither invalidate the task nor get restored into it. It will extend this support to more first-party tools in the future. Third-party tools can report cache metadata with [`@voidzero-dev/vite-task-client`](https://npmx.dev/package/@voidzero-dev/vite-task-client).

## When To Add Manual Config

Add config when your project has behavior the command or tool cannot know. These options go under the task's [`cache`](/config/run#cache) object.

| Case                                                              | Example                                                                                         |
| ----------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Exclude an output directory from inputs                           | `input: [{ auto: true }, '!dist/**']`                                                           |
| Exclude a temporary generated file from input and output tracking | `input: [{ auto: true }, '!.tmp/config.mjs']`<br>`output: [{ auto: true }, '!.tmp/config.mjs']` |
| Avoid automatic file tracking for a task                          | `input: ['src/**']`<br>`output: ['dist/**']`                                                    |
| Track and pass an env var                                         | `env: ['NODE_ENV']`                                                                             |
| Pass an env var without fingerprinting it                         | `untrackedEnv: ['GITHUB_ACTIONS']`                                                              |
