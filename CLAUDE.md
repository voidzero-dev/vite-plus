# Vite-Plus

A monorepo task runner (like nx/turbo) with intelligent caching and dependency resolution.

## Core Concept

**Task Execution**: Run tasks across monorepo packages with automatic dependency ordering.

```bash
# Run task in current package (implicit mode)
vite-plus build

# Run tasks across packages (explicit mode)
vite-plus run build -r                    # recursive with topological ordering
vite-plus run app#build web#build         # specific packages
vite-plus run build -r --no-topological   # recursive without implicit deps
```

## Key Architecture

- **Entry**: `crates/vite_task/src/lib.rs` - CLI parsing and main logic
- **Workspace**: `src/config/workspace.rs` - Loads packages, creates task graph
- **Task Graph**: `src/config/task_graph_builder.rs` - Builds dependency graph
- **Execution**: `src/schedule.rs` - Executes tasks in dependency order

## Task Dependencies

1. **Explicit** (always applied): Defined in `vite-task.json`
   ```json
   {
     "tasks": {
       "test": {
         "command": "jest",
         "dependsOn": ["build", "lint"]
       }
     }
   }
   ```

2. **Implicit** (when `--topological`): Based on package.json dependencies
   - If A depends on B, then A#build depends on B#build automatically

## Key Features

- **Topological Flag**: Controls implicit dependencies from package relationships
  - Default: ON for `--recursive`, OFF otherwise
  - Toggle with `--no-topological` to disable

- **Boolean Flags**: All support `--no-*` pattern for explicit disable
  - Example: `--recursive` vs `--no-recursive`
  - Conflicts handled by clap
  - If you want to add a new boolean flag, follow this pattern

## Quick Reference

- **Compound Commands**: `"build": "tsc && rollup"` splits into subtasks
- **Task Format**: `package#task` (e.g., `app#build`)
- **Tests**: Run `cargo test -p vite_task` to verify changes
- **Debug**: Use `--debug` to see cache operations

## Tests

- Run `cargo test` to execute all tests
- You never need to run `pnpm install` in the test fixtures dir, vite-plus should able to load and parse the workspace without `pnpm install`.
