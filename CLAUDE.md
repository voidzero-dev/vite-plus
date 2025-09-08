# Vite-Plus

A monorepo task runner (like nx/turbo) with intelligent caching and dependency resolution.

## Core Concept

**Task Execution**: Run tasks across monorepo packages with automatic dependency ordering.

```bash
# Built-in commands
vite-plus build                           # Run Vite build (dedicated command)
vite-plus test                            # Run Vite test (dedicated command)
vite-plus lint                            # Run oxlint (dedicated command)

# Run tasks across packages (explicit mode)
vite-plus run build -r                    # recursive with topological ordering
vite-plus run app#build web#build         # specific packages
vite-plus run build -r --no-topological   # recursive without implicit deps

# Run task in current package (implicit mode - for non-built-in tasks)
vite-plus dev                             # runs dev script from package.json
```

## Key Architecture

- **Entry**: `crates/vite_task/src/lib.rs` - CLI parsing and main logic
- **Workspace**: `crates/vite_task/src/config/workspace.rs` - Loads packages, creates task graph
- **Task Graph**: `crates/vite_task/src/config/task_graph_builder.rs` - Builds dependency graph
- **Execution**: `crates/vite_task/src/schedule.rs` - Executes tasks in dependency order

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

## Path Type System

- **Type Safety**: All paths use typed `vite_path` instead of `std::path` for better safety
  - **Absolute Paths**: `vite_path::AbsolutePath` / `AbsolutePathBuf`
  - **Relative Paths**: `vite_path::RelativePath` / `RelativePathBuf`

- **Usage Guidelines**:
  - Use methods such as `strip_prefix`/`join` provided in `vite_path` for path operations instead of converting to std paths
  - Only convert to std paths when interfacing with std library functions, and this should be implicit in most cases thanks to `AsRef<Path>` implementations
  - Add necessary methods in `vite_path` instead of falling back to std path types

## Quick Reference

- **Compound Commands**: `"build": "tsc && rollup"` splits into subtasks
- **Task Format**: `package#task` (e.g., `app#build`)
- **Path Types**: Use `vite_path` types instead of `std::path` types for type safety
- **Tests**: Run `cargo test -p vite_task` to verify changes
- **Debug**: Use `--debug` to see cache operations

## Tests

- Run `cargo test` to execute all tests
- You never need to run `pnpm install` in the test fixtures dir, vite-plus should able to load and parse the workspace without `pnpm install`.
