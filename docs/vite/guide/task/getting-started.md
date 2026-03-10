# Getting Started

## Overview

`vp run` works like `pnpm run` — it runs the scripts in your `package.json` — but adds caching, dependency ordering, and monorepo-aware execution on top. It works in both single-package projects and monorepos.

You can add task definitions in `vite.config.ts` when you need more control over dependencies and caching.

## Running Scripts

If your project already has scripts in `package.json`, you can run them with `vp run` right away — no configuration needed:

```json [package.json]
{
  "scripts": {
    "build": "vp build",
    "test": "vp test"
  }
}
```

```bash
vp run build
```

```
$ vp build
vite v8.0.0 building for production...
✓ 4 modules transformed.
dist/index.html                0.12 kB │ gzip: 0.12 kB
dist/assets/index-FvSqcG4U.js  0.69 kB │ gzip: 0.39 kB

✓ built in 28ms
```

## Caching

Plain `package.json` scripts are not cached by default. Pass `--cache` to try it out:

```bash
vp run --cache build
```

```
$ vp build
✓ built in 28ms
```

Run it again — the output is replayed instantly from cache:

```
$ vp build ✓ cache hit, replaying
✓ built in 28ms

---
[vp run] cache hit, 468ms saved.
```

Edit a source file and run again — the task runner detects the change and re-runs:

```
$ vp build ✗ cache miss: 'src/index.ts' modified, executing
```

The task runner automatically tracks which files your command reads. No configuration needed.

The `--cache` flag is a quick way to try caching, but the default behavior may not suit every task — you may need to control which files or environment variables affect the cache. To configure caching properly and enable it permanently, define the task in `vite.config.ts`. See [Caching](./caching) for how it works and [Config Reference](./config) for all task options.

## Task Definitions {#task-definitions}

Task definitions in `vite.config.ts` enable caching by default and give you more control — dependencies, environment variables, and custom inputs:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  run: {
    tasks: {
      build: {
        // Uses the "build" script from package.json
        dependsOn: ['lint'],
        envs: ['NODE_ENV'],
      },
      deploy: {
        command: 'deploy-script --prod',
        cache: false,
        dependsOn: ['build', 'test'],
      },
    },
  },
});
```

A task definition can either reference a `package.json` script (by omitting `command`) or specify its own command. You cannot define a command in both places.

::: info
Tasks defined in `vite.config.ts` are cached by default. Plain `package.json` scripts (without a matching task entry) are **not** cached by default. See [When Is Caching Enabled?](./caching#when-is-caching-enabled) for details.
:::

See [Config Reference](./config) for all available task options.

## Task Dependencies

Use [`dependsOn`](./config#depends-on) to ensure tasks run in the right order. Running `vp run deploy` with the config above runs `build` and `test` first:

```
                ┌─────────┐   ┌────────┐
                │  build  │   │  test  │
                └────┬────┘   └───┬────┘
                     │            │
                     └─────┬──────┘
                           │
                     ┌─────▼─────┐
                     │  deploy   │
                     └───────────┘
```

Dependencies can reference tasks in other packages using the `package#task` format:

```ts
dependsOn: ['@my/core#build', '@my/utils#lint'];
```

## Running Across Packages

In a monorepo, use `-r` (recursive) to run a task across all packages:

```bash
vp run -r build
```

The task runner automatically orders packages by their `package.json` dependency graph — if `@my/app` depends on `@my/utils` which depends on `@my/core`, they build in that order. Each package's result is cached independently.

See [Running Tasks](./running-tasks) for package selection with `--filter`, `--transitive`, and other options.

## What's Next?

- [Running Tasks](./running-tasks) — package selection, compound commands, and concurrency
- [Caching](./caching) — how caching works, file tracking, and cache sharing
- [CLI Reference](./cli) — all flags and options
- [Config Reference](./config) — all task configuration options
