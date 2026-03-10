# Getting Started

The Task Runner lets you define, run, and cache tasks in any project — whether it's a single-package project or a large monorepo.

## Why Use a Task Runner?

When you work on a project, you frequently run the same commands — build, lint, test. Each time, these commands do the same work if nothing changed. The task runner solves this by:

- **Caching results** — when nothing changed, tasks complete instantly by replaying previous output
- **Tracking dependencies** — tasks run in the correct order, so `build` always runs before `deploy`
- **Running across packages** — in a monorepo, run tasks across all packages with a single command

## Defining Tasks

Tasks are defined in your `vite.config.ts` under the `run` section:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus'

export default defineConfig({
  run: {
    tasks: {
      build: {
        command: 'vp build',
      },
      lint: {
        command: 'vp lint',
      },
    },
  },
})
```

## Running a Task

Use `vp run` followed by the task name:

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

Run the same task again — the output is replayed instantly from cache:

```bash
vp run build
```

```
$ vp build ✓ cache hit, replaying
vite v8.0.0 building for production...
✓ 4 modules transformed.
dist/index.html                0.12 kB │ gzip: 0.12 kB
dist/assets/index-FvSqcG4U.js  0.69 kB │ gzip: 0.39 kB

✓ built in 28ms

---
[vp run] cache hit, 468ms saved.
```

The task runner automatically tracks which files your command reads. If you edit a source file and run again, it detects the change and re-runs the task:

```
$ vp build ✗ cache miss: 'src/index.ts' modified, executing
```

No configuration needed — it just works.

::: tip
Tasks are cached by default. You can [disable caching](/config/task#cache) for tasks that shouldn't be cached, like dev servers.
:::

## Scripts vs. Tasks {#scripts-vs-tasks}

The task runner recognizes two sources of runnable commands:

1. **Package.json scripts** — any entry in `"scripts"` can be run with `vp run`:

   ```json [package.json]
   {
     "scripts": {
       "build": "vp build",
       "test": "vp test"
     }
   }
   ```

   ```bash
   vp run test
   ```

2. **Task definitions** — entries in `vite.config.ts` that add configuration like dependencies or cache settings:

   ```ts [vite.config.ts]
   export default defineConfig({
     run: {
       tasks: {
         build: {
           // No command — uses the "build" script from package.json
           dependsOn: ['lint'],
           envs: ['NODE_ENV'],
         },
       },
     },
   })
   ```

A task definition can either reference a `package.json` script (by omitting `command`) or specify its own command. You cannot define a command in both places — it's one or the other.

::: tip When to Use Which?
Use **`package.json` scripts** for simple commands you want to run standalone (e.g., `"dev": "vp dev"`). Use **task definitions** in `vite.config.ts` when you need caching, dependencies, or other task runner features. You can combine both — define the command in `package.json` and add configuration in `vite.config.ts`.
:::

::: info
Tasks defined in `vite.config.ts` are cached by default. Plain `package.json` scripts (without a matching task entry) are **not** cached by default. See [Cache Configuration](/config/task#run-cache) for details.
:::

## Task Dependencies

Use [`dependsOn`](/config/task#depends-on) to ensure tasks run in the right order:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus'

export default defineConfig({
  run: {
    tasks: {
      deploy: {
        command: 'deploy-script --prod',
        cache: false,
        dependsOn: ['build', 'test'],
      },
      build: {
        command: 'vp build',
      },
      test: {
        command: 'vp test',
      },
    },
  },
})
```

Running `vp run deploy` runs `build` and `test` first:

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

Run `vp run` without a task name to browse all available tasks:

```bash
vp run
```

```
Select a task (↑/↓, Enter to run, Esc to clear):

  › build: vp build
    lint: vp lint
```

Use the arrow keys to navigate, type to search, and press Enter to run. If you mistype a task name, the selector opens with suggestions:

```bash
vp run buid
```

```
Task "buid" not found.
Select a task (↑/↓, Enter to run, Esc to clear): buid

  › build: vp build
```

## What's Next?

- [Running Tasks](./running-tasks) — package selection, compound commands, and concurrency
- [Caching](./caching) — how caching works, file tracking, and cache sharing
- [CLI Reference](./cli) — all flags and options
- [Config Reference](/config/task) — all task configuration options
