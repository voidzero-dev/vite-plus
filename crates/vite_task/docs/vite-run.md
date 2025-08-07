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

### Example Execution Flow

Given the following monorepo structure with topological ordering enabled:

```
Package Dependencies:
  app    → utils, ui
  ui     → utils
  utils  → (none)

Task Dependencies (explicit):
  app#build    → app#lint
  utils#build  → utils#test
```

The execution flow for `vite-plus run build -r --topological`:

```
┌─────────────────────────────────────────────────────────────┐
│                    Task Resolution                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Step 1: Collect all build tasks                            │
│  ────────────────────────────────────────                   │
│    • app#build                                              │
│    • ui#build                                               │
│    • utils#build                                            │
│                                                             │
│  Step 2: Add explicit dependencies                          │
│  ─────────────────────────────────────                      │
│    • app#build depends on app#lint                          │
│    • utils#build depends on utils#test                      │
│                                                             │
│  Step 3: Add implicit dependencies (--topological)          │
│  ──────────────────────────────────────────────────         │
│    • app#build depends on utils#build (app→utils)           │
│    • app#build depends on ui#build (app→ui)                 │
│    • ui#build depends on utils#build (ui→utils)             │
│                                                             │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Execution Order                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Wave 1: No dependencies                                    │
│  ┌──────────────┐                                           │
│  │ utils#test   │                                           │
│  │ app#lint     │ (can run in parallel with --parallel)     │
│  └──────────────┘                                           │
│         │                                                   │
│         ▼                                                   │
│  Wave 2: Dependencies from Wave 1                           │
│  ┌──────────────┐                                           │
│  │ utils#build  │                                           │
│  └──────────────┘                                           │
│         │                                                   │
│         ▼                                                   │
│  Wave 3: Dependencies from Wave 2                           │
│  ┌──────────────┐                                           │
│  │ ui#build     │                                           │
│  └──────────────┘                                           │
│         │                                                   │
│         ▼                                                   │
│  Wave 4: Final dependencies                                 │
│  ┌──────────────┐                                           │
│  │ app#build    │                                           │
│  └──────────────┘                                           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

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

## Package Graph Construction

Vite-plus builds a package graph to understand the relationships between packages in your monorepo:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     Workspace Root                                      │
│                  (pnpm-workspace.yaml)                                  │
└─────────────────┬───────────────────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│              1. Package Discovery                                       │
│                                                                         │
│  packages/            packages/          packages/                      │
│  ├── app/             ├── utils/         └── nameless/                  │
│  │   └── package.json │   └── package.json       └── package.json       │
│  │       name:        │       name:                  (no name)          │
│  │       "app"        │       "utils"                                   │
└─────────────────┬───────────────────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────┐
│              2. Dependency Resolution                       │
│                                                             │
│    app ─────depends─on────▶ utils                           │
│     ↓                         ↑                             │
│     └──────depends─on────▶ nameless ✗ (not allowed)         │
│                                                             │
│  Note: Nameless packages cannot be referenced               │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────┐
│              3. Task Graph Construction                     │
│                                                             │
│  app#build ──────────▶ utils#build                          │
│      ↓                      ↓                               │
│  app#test               utils#test                          │
│                                                             │
│  build ◀──── test  (nameless package internal deps)         │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────┐
│              4. Execution Planning                          │
│                                                             │
│  Execution Order: utils#build → app#build → parallel(tests) │
└─────────────────────────────────────────────────────────────┘
```

### 1. Package Discovery

The package graph builder starts by discovering all packages in the workspace:

- Reads the workspace configuration (`pnpm-workspace.yaml`, `yarn workspaces`, or `npm workspaces`)
- Resolves glob patterns to find all package directories
- Loads `package.json` from each package directory
- Creates a node in the graph for each package

### 2. Dependency Resolution

For each package, the builder analyzes its dependencies:

- Examines `dependencies`, `devDependencies`, and `peerDependencies` in `package.json`
- Identifies workspace dependencies (marked with `workspace:*` protocol)
- Creates edges in the graph between packages based on these dependencies
- Validates that all referenced workspace packages exist

### 3. Task Graph Construction

Once the package graph is built, vite-plus constructs a task graph:

- Loads tasks from `vite-task.json` files in each package
- Loads scripts from `package.json` files
- Resolves explicit task dependencies (from `dependsOn` fields)
- When `--topological` is enabled, adds implicit dependencies based on package relationships
- Validates that all task dependencies can be resolved

### 4. Execution Planning

The final step creates an execution plan:

- Performs topological sorting of the task graph
- Identifies tasks that can run in parallel
- Detects circular dependencies and reports errors
- Determines the optimal execution order

## Packages Without Names

Vite-plus supports packages that have no `name` field in their `package.json`. These anonymous packages have special constraints and behaviors:

```
┌────────────────────────────────────────────────────────────┐
│                 Multiple Nameless Packages                 │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  packages/                                                 │
│  ├── frontend/        (no name field)                      │
│  │   ├── package.json                                      │
│  │   └── vite-task.json                                    │
│  │                                                         │
│  ├── backend/         (no name field)                      │
│  │   ├── package.json                                      │
│  │   └── vite-task.json                                    │
│  │                                                         │
│  └── shared/          name: "@company/shared"              │
│      ├── package.json                                      │
│      └── vite-task.json                                    │
│                                                            │
├────────────────────────────────────────────────────────────┤
│                     Task Resolution                        │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  Recursive mode (-r):                                      │
│  ┌───────────────────────────────────────────┐             │
│  │ vite-plus run build -r                    │             │
│  └─────────────┬─────────────────────────────┘             │
│                ▼                                           │
│    ✓ build (frontend)                                      │
│    ✓ build (backend)                                       │
│    ✓ @company/shared#build                                 │
│                                                            │
│  Explicit mode:                                            │
│  ┌───────────────────────────────────────────┐             │
│  │ vite-plus run #build                      │             │
│  └─────────────┬─────────────────────────────┘             │
│                ▼                                           │
│    ✗ Error: Cannot reference nameless package              │
│                                                            │
│  Implicit mode (from package dir):                         │
│  ┌───────────────────────────────────────────┐             │
│  │ cd packages/frontend && vite-plus build   │             │
│  └─────────────┬─────────────────────────────┘             │
│                ▼                                           │
│    ✓ Runs build in current nameless package                │
│                                                            │
├────────────────────────────────────────────────────────────┤
│                   Dependency Rules                         │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  Allowed:                                                  │
│  ┌──────────────────────────────────────────┐              │
│  │ nameless ──depends─on──▶ @company/shared │              │
│  └──────────────────────────────────────────┘              │
│                                                            │
│  Not Allowed:                                              │
│  ┌──────────────────────────────────────────┐              │
│  │ @company/shared ──depends─on──▶ nameless │ ✗            │
│  └──────────────────────────────────────────┘              │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

### Constraints

1. **Tasks cannot be depended upon**: Other packages cannot declare dependencies on tasks from packages without names. This prevents ambiguity since there's no way to uniquely reference these tasks from external packages.

2. **Cannot be specified with `vite run` command**: You cannot directly target tasks from nameless packages using explicit mode (e.g., `vite-plus run #build` won't work). The lack of a package name makes it impossible to construct the `package#task` identifier.

3. **Can run via implicit mode**: When you're in the directory of a nameless package, you can use implicit mode to run its tasks:

   ```bash
   cd packages/anonymous-package
   vite-plus build  # Runs the build task in the current nameless package
   ```

4. **Included in recursive runs**: Tasks from nameless packages are included when using the `-r` flag:

   ```bash
   vite-plus run build -r  # Includes build tasks from all packages, including nameless ones
   ```

5. **Can depend on other packages**: Tasks within nameless packages can declare dependencies on tasks from named packages:

   ```json
   {
     "tasks": {
       "build": {
         "command": "tsc",
         "dependsOn": ["core#build", "utils#build"]
       }
     }
   }
   ```

6. **Multiple nameless packages allowed**: A monorepo can contain multiple packages without names. Each operates independently with the same constraints.
