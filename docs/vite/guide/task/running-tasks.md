# Running Tasks

## Package Selection {#package-selection}

With no flags, `vp run` runs the task in the package containing your current directory:

```bash
cd packages/app
vp run build        # runs build in @my/app only
```

Use `package#task` to target a specific package from anywhere:

```bash
vp run @my/app#build
```

### Recursive (`-r`) {#recursive}

Run the task across **all** packages in the workspace:

```bash
vp run -r build     # builds every package, dependencies first
```

### Transitive (`-t`) {#transitive}

Run the task in a package **and all its dependencies**:

```bash
vp run -t @my/app#build
```

If `@my/app` depends on `@my/utils` which depends on `@my/core`, this builds all three in dependency order. You can also use `--filter "@my/app..."` for the same result.

### Filter (`--filter`) {#filter}

Select packages by name, directory, or glob pattern. The task name always comes **after** the flags:

```bash
# By name
vp run --filter @my/app build

# By glob
vp run --filter "@my/*" build

# By directory
vp run --filter ./packages/app build

# With dependencies
vp run --filter "@my/app..." build    # app + all its dependencies

# With dependents
vp run --filter "...@my/core" build   # core + everything that depends on it

# Exclude packages
vp run --filter "@my/*" --filter "!@my/utils" build
```

Multiple `--filter` flags produce a union of the selected packages.

::: tip
The `--filter` syntax is compatible with pnpm. Most pnpm filter expressions work identically.
:::

### Workspace Root (`-w`) {#workspace-root}

Explicitly select the workspace root package:

```bash
vp run -w build
```

## Compound Commands {#compound-commands}

Commands joined with `&&` are split into independently cached sub-tasks:

```ts [vite.config.ts]
export default defineConfig({
  run: {
    tasks: {
      check: {
        command: 'vp lint && vp build',
      },
    },
  },
})
```

```
$ vp lint
Found 0 warnings and 0 errors.

$ vp build
✓ built in 28ms

---
[vp run] 0/2 cache hit (0%).
```

Each sub-task has its own cache entry. On the next run, if only `.ts` files changed but lint passes, only `vp build` re-runs:

```
$ vp lint ✓ cache hit, replaying
$ vp build ✗ cache miss: 'src/index.ts' modified, executing
✓ built in 30ms

---
[vp run] 1/2 cache hit (50%), 120ms saved.
```

### Nested `vp run` {#nested-vp-run}

When a command contains `vp run`, it is **expanded at plan time** into the execution graph — no extra process is spawned. This means each sub-task is cached independently and output is shown flat:

```json [package.json]
{
  "scripts": {
    "ci": "vp run lint && vp run test && vp run build"
  }
}
```

Running `vp run ci` expands into three separate tasks, each with its own cache entry:

```
┌────────┐
│  lint  │
└───┬────┘
    │
┌───▼────┐
│  test  │
└───┬────┘
    │
┌───▼────┐
│ build  │
└────────┘
```

Flags work inside nested scripts too. For example, `vp run -r build` inside a script expands into individual build tasks for every package.

::: details Self-Referencing Scripts
A common monorepo pattern is a root script that runs a task recursively:

```json [package.json (root)]
{
  "scripts": {
    "build": "vp run -r build"
  }
}
```

This creates a potential recursion: root's `build` → `vp run -r build` → includes root's `build` → ... The task runner detects this and prunes the self-reference automatically, so other packages' builds run normally without infinite loops.
:::

## Concurrency {#concurrency}

When running multiple tasks (e.g., with `-r`), the task runner executes independent tasks in parallel. Tasks that have no dependency relationship run concurrently, while dependent tasks wait for their dependencies to complete.

For a single task, the process inherits the terminal directly — interactive programs, progress bars, and stdin all work normally. For multiple concurrent tasks, output is captured and displayed in order to avoid interleaving.

## Execution Summary {#execution-summary}

Use `-v` to see a detailed execution summary:

```bash
vp run -r -v build
```

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    Vite+ Task Runner • Execution Summary
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Statistics:   3 tasks • 3 cache hits • 0 cache misses
Performance:  100% cache hit rate, 468ms saved in total

Task Details:
────────────────────────────────────────────────
  [1] @my/core#build: ~/packages/core$ vp build ✓
      → Cache hit - output replayed - 200ms saved
  ·······················································
  [2] @my/utils#build: ~/packages/utils$ vp build ✓
      → Cache hit - output replayed - 150ms saved
  ·······················································
  [3] @my/app#build: ~/packages/app$ vp build ✓
      → Cache hit - output replayed - 118ms saved
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

Use `--last-details` to recall the summary from the last run without re-executing:

```bash
vp run --last-details
```
