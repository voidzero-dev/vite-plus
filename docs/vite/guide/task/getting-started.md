# Getting Started

## Overview

The Vite+ Task Runner is a high-performance task execution system built into Vite+. It caches task results so repeated runs complete instantly, orders tasks by their dependencies, and scales across monorepo packages — all with minimal configuration.

It works with the scripts you already have in `package.json`. You can add task definitions in `vite.config.ts` when you need more control over dependencies and caching.

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

The `--cache` flag is a quick way to try caching, but the default behavior may not suit every task — you may need to control which files or environment variables affect the cache. To configure caching properly and enable it permanently, define the task in `vite.config.ts`.

## Task Definitions {#task-definitions}

Task definitions in `vite.config.ts` enable caching by default and give you more control — dependencies, environment variables, and custom inputs:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus'

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
})
```

A task definition can either reference a `package.json` script (by omitting `command`) or specify its own command. You cannot define a command in both places.

::: info
Tasks defined in `vite.config.ts` are cached by default. Plain `package.json` scripts (without a matching task entry) are **not** cached by default. See [Cache Configuration](./config#run-cache) for details.
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
dependsOn: ['@my/core#build', '@my/utils#lint']
```

## Monorepo Setup

In a monorepo, each package can define its own tasks in its `vite.config.ts`:

```
my-project/
├── pnpm-workspace.yaml
├── packages/
│   ├── core/
│   │   ├── package.json          # @my/core
│   │   └── vite.config.ts
│   ├── utils/
│   │   ├── package.json          # @my/utils → depends on @my/core
│   │   └── vite.config.ts
│   └── app/
│       ├── package.json          # @my/app → depends on @my/utils
│       └── vite.config.ts
```

Use `-r` (recursive) to run a task across all packages:

```bash
vp run -r build
```

```
~/packages/core$ vp build
✓ built in 28ms

~/packages/utils$ vp build
✓ built in 21ms

~/packages/app$ vp build
✓ built in 26ms

---
[vp run] 0/3 cache hit (0%).
```

The task runner uses your `package.json` dependency graph to determine the order — `@my/core` builds before `@my/utils`, which builds before `@my/app`. This is called **topological ordering** and is applied automatically.

When combined with `dependsOn`, both types of ordering work together. For example, with `-r` and `dependsOn: ['lint']` on every package's `build` task:

```
core#lint → core#build → utils#lint → utils#build → app#lint → app#build
```

Each package's `lint` runs before its `build` (explicit dependency), and packages are ordered by their dependency graph (topological).

Run it again — each package's task is cached independently:

```
~/packages/core$ vp build ✓ cache hit, replaying
~/packages/utils$ vp build ✓ cache hit, replaying
~/packages/app$ vp build ✓ cache hit, replaying

---
[vp run] 3/3 cache hit (100%), 468ms saved.
```

## Interactive Task Selector

Run `vp run` without a task name to browse available tasks:

```bash
vp run
```

```
Select a task (↑/↓, Enter to run, Esc to clear):

  › build: vp build
    lint: vp lint
```

## What's Next?

- [Running Tasks](./running-tasks) — package selection, compound commands, and concurrency
- [Caching](./caching) — how caching works, file tracking, and cache sharing
- [CLI Reference](./cli) — all flags and options
- [Config Reference](./config) — all task configuration options
