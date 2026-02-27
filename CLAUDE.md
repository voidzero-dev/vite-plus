# Vite-Plus

A monorepo task runner (like nx/turbo) with intelligent caching and dependency resolution.

## Core Concept

**Task Execution**: Run tasks across monorepo packages with automatic dependency ordering.

```bash
# Built-in commands
vp build                           # Run Vite build (dedicated command)
vp test                            # Run Vitest (dedicated command)
vp lint                            # Run oxlint (dedicated command)

# Run tasks across packages (explicit mode)
vp run build -r                    # recursive with topological ordering
vp run app#build web#build         # specific packages
vp run build -r --no-topological   # recursive without implicit deps

# Run task in current package (implicit mode - for non-built-in tasks)
vp run dev                         # runs dev script from package.json
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

- **Converting from std paths** (e.g., `TempDir::path()`):

  ```rust
  let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
  ```

- **Function signatures**: Prefer `&AbsolutePath` over `&std::path::Path`

- **Passing to std functions**: `AbsolutePath` implements `AsRef<Path>`, use `.as_path()` when explicit `&Path` is required

## Clippy Rules

All **new** Rust code must follow the custom clippy rules defined in `.clippy.toml` (disallowed types, macros, and methods). Existing code may not fully comply due to historical reasons.

## CLI Output

All user-facing output must go through shared output modules instead of raw print calls.

- **Rust**: Use `vite_shared::output` functions (`info`, `warn`, `error`, `note`, `success`) — never raw `println!`/`eprintln!` (enforced by clippy `disallowed-macros`)
- **TypeScript**: Use `packages/cli/src/utils/terminal.ts` functions (`infoMsg`, `warnMsg`, `errorMsg`, `noteMsg`, `log`) — never raw `console.log`/`console.error`

## Git Workflow

- Run `vp check --fix` before committing to format and lint code

## Quick Reference

- **Compound Commands**: `"build": "tsc && rollup"` splits into subtasks
- **Task Format**: `package#task` (e.g., `app#build`)
- **Path Types**: Use `vite_path` types instead of `std::path` types for type safety
- **Tests**: Run `cargo test -p vite_task` to verify changes
- **Debug**: Use `--debug` to see cache operations

## Tests

- Run `cargo test` to execute all tests
- You never need to run `pnpm install` in the test fixtures dir, vite-plus should able to load and parse the workspace without `pnpm install`.

## Build

- Run `pnpm bootstrap-cli` from the project root to build all packages and install the global CLI
  - This builds all `@voidzero-dev/*` and `vite-plus` packages
  - Compiles the Rust NAPI bindings and the `vp` Rust binary
  - Installs the CLI globally to `~/.vite-plus/`

## Snap Tests

Snap tests are located in `packages/cli/snap-tests/` (local CLI) and `packages/cli/snap-tests-global/` (global CLI). Each test case is a directory containing:

- `package.json` - Package configuration for the test
- `steps.json` - Commands to run and environment variables
- `src/` - Source files for the test
- `snap.txt` - Expected output (generated/updated by running the test)

```bash
# Run all snap tests (local + global)
pnpm -F vite-plus snap-test

# Run only local CLI snap tests
pnpm -F vite-plus snap-test-local
pnpm -F vite-plus snap-test-local <name-filter>

# Run only global CLI snap tests
pnpm -F vite-plus snap-test-global
pnpm -F vite-plus snap-test-global <name-filter>
```

The snap test will automatically generate/update the `snap.txt` file with the command outputs. It exits with zero status even if there are output differences; you need to manually check the diffs(`git diff`) to verify correctness.
