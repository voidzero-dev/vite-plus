# `vite-plus run`

Vite-plus run has two modes:

1. Implicit mode is running `vite-plus` without `run` command, it will run the task in the current package, it supposed to replace the `pnpm/yarn run` command.
2. Explicit mode is running `vite-plus run` with run command, like `vite-plus run vite-plus#build`.

## Implicit mode

With implicit mode, `vite` will run the task in the current package. It can't accept more than on task. The first argument will be treated as the task name; the args following the command will be treated as the task args and bypass to the task.

Given the following `package.json` file:

```json
{
  "scripts": {
    "build": "vite build",
    "lint": "oxlint"
  }
}
```

This command equivalent to `vite build`:

```bash
vite-plus build
```

This command equivalent to `vite build --mode production`:

```bash
vite-plus build --mode production
```

This command equivalent to `oxlint packages/cli/binding/index.js`:

```bash
vite-plus lint packages/cli/binding/index.js
```

If the command contains `#`, the command will be treated as a task in the workspace subpackage.

This command equivalent to `vite-plus run cli#build`:

```bash
vite-plus cli#build
```

## Explicit mode

With explicit mode, `vite-plus run` will run the task in the workspace subpackage. It can accept more than one task. The arguments will be treated as the task names; the args following the `--` will be treated as the task args and bypass to the task.
Unlike implicit mode, explicit mode can't accept task name without `#`, unless `--recursive` flag is used.

### Default behavior

The `vite-plus run` command will run the scoped tasks in dependency order. If there is a task name without `#`, it would cause an `Task not found` error.

### `--recursive,-r`

With the `--recursive,-r` flag, the `vite-plus run` command will run the tasks in all monorepo packages.

The task name should't contain `#` with the `--recursive,-r` flag. If any task name contains `#`, it would cause an `RecursiveRunWithScope` error.

### `--topological,-t` / `--no-topological`

The `--topological` flag controls whether implicit dependencies based on package dependencies are included in the task graph. This flag affects how tasks are ordered and executed:

- **With `--topological` (or `-t`)**: When package A depends on package B, and both have a task with the same name (e.g., "build"), then A's task will automatically depend on B's task. This ensures that dependencies are built before dependents.
- **With `--no-topological`**: Explicitly disables topological ordering. Only explicit task dependencies defined in `vite-task.json` files are honored. Package dependencies do not create implicit task dependencies.

Default behavior:

- When used with `--recursive`, topological is **enabled by default** (can be disabled with `--no-topological`)
- When used without `--recursive`, topological is **disabled by default** (can be enabled with `--topological` or `-t`)

Examples:

```bash
# Recursive build with topological ordering (default)
vite-plus run build -r

# Recursive build WITHOUT topological ordering
vite-plus run build -r --no-topological

# Single package with topological ordering enabled
vite-plus run app#build -t

# Multiple packages without topological ordering (default)
vite-plus run app#build web#build
```

Note: `--topological` and `--no-topological` are mutually exclusive and cannot be used together. See [boolean-flags.md](./boolean-flags.md) for more information about boolean flag patterns.

## Task Dependencies

Vite-plus supports two types of task dependencies:

### 1. Explicit Dependencies (Always Applied)

Tasks can declare explicit dependencies in `vite-task.json` files using the `dependsOn` field:

```json
{
  "tasks": {
    "lint": {
      "command": "eslint src",
      "cacheable": true,
      "dependsOn": ["build", "core#build"]
    },
    "deploy": {
      "command": "deploy-script --prod",
      "cacheable": false,
      "dependsOn": ["test", "build", "utils#lint"]
    }
  }
}
```

These explicit dependencies are always honored regardless of the `--topological` flag.

### 2. Implicit Dependencies (Topological Mode Only)

When `--topological` is enabled, vite-plus automatically creates dependencies between tasks with the same name based on package dependencies:

- If package `app` depends on package `utils` in `package.json`
- And both packages have a `build` script
- Then `app#build` will automatically depend on `utils#build`

This works transitively - if `app` depends on `utils`, and `utils` depends on `core`, then `app#build` will depend on both `utils#build` and `core#build`.

## Execution Order

Tasks are executed in topological order based on their dependencies:

1. Tasks with no dependencies run first
2. Tasks only run after all their dependencies have completed successfully
3. Independent tasks may run in parallel when `--parallel` is used

## Compound Commands

When a script contains `&&` operators, it's split into subtasks that execute sequentially:

```json
{
  "scripts": {
    "build": "tsc && rollup -c && echo 'Build complete'"
  }
}
```

This creates three subtasks:

- `package#build` (subcommand 0): `tsc`
- `package#build` (subcommand 1): `rollup -c`
- `package#build`: `echo 'Build complete'`

Cross-package dependencies connect to the first subtask, and the last subtask is considered the completion of the task.
