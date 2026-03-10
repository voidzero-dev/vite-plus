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

Select packages by name, directory, or glob pattern. The syntax is compatible with pnpm's `--filter`:

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

### Workspace Root (`-w`) {#workspace-root}

Explicitly select the workspace root package:

```bash
vp run -w build
```

## Compound Commands {#compound-commands}

Commands joined with `&&` are split into independent sub-tasks, each cached separately when [caching is enabled](./caching#when-is-caching-enabled). This applies to both `vite.config.ts` tasks and `package.json` scripts, where compound commands are common:

```json [package.json]
{
  "scripts": {
    "check": "vp lint && vp build"
  }
}
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

When a command contains `vp run`, the task runner **inlines it as separate tasks** rather than spawning a nested process. Each sub-task is cached independently and output is shown flat:

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
