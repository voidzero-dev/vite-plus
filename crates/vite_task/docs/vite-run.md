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

The `vite-plus run` command will run the scoped tasks in topological order. If there is a task name without `#`, it would cause an `Task not found` error.

### `--recursive,-r`

With the `--recursive,-r` flag, the `vite-plus run` command will run the tasks in all monorepo packages.

The task name should't contain `#` with the `--recursive,-r` flag. If any task name contains `#`, it would cause an `RecursiveRunWithScope` error.
