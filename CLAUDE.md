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

<!--VITE PLUS START-->

# Using Vite+, the Unified Toolchain for the Web

This project is using Vite+, a unified toolchain built on top of Vite, Rolldown, Vitest, tsdown, Oxlint, Oxfmt, and Vite Task. Vite+ wraps runtime management, package management, and frontend tooling in a single global CLI called `vp`. Vite+ is distinct from Vite, but it invokes Vite through `vp dev` and `vp build`.

## Vite+ Workflow

`vp` is a global binary that handles the full development lifecycle. Run `vp help` to print a list of commands and `vp <command> --help` for information about a specific command.

### Start

- create - Create a new project from a template
- migrate - Migrate an existing project to Vite+
- config - Configure hooks and agent integration
- staged - Run linters on staged files
- install (`i`) - Install dependencies
- env - Manage Node.js versions

### Develop

- dev - Run the development server
- check - Run format, lint, and TypeScript type checks
- lint - Lint code
- fmt - Format code
- test - Run tests

### Execute

- run - Run monorepo tasks
- exec - Execute a command from local `node_modules/.bin`
- dlx - Execute a package binary without installing it as a dependency
- cache - Manage the task cache

### Build

- build - Build for production
- pack - Build libraries
- preview - Preview production build

### Manage Dependencies

Vite+ automatically detects and wraps the underlying package manager such as pnpm, npm, or Yarn through the `packageManager` field in `package.json` or package manager-specific lockfiles.

- add - Add packages to dependencies
- remove (`rm`, `un`, `uninstall`) - Remove packages from dependencies
- update (`up`) - Update packages to latest versions
- dedupe - Deduplicate dependencies
- outdated - Check for outdated packages
- list (`ls`) - List installed packages
- why (`explain`) - Show why a package is installed
- info (`view`, `show`) - View package information from the registry
- link (`ln`) / unlink - Manage local package links
- pm - Forward a command to the package manager

### Maintain

- upgrade - Update `vp` itself to the latest version

These commands map to their corresponding tools. For example, `vp dev --port 3000` runs Vite's dev server and works the same as Vite. `vp test` runs JavaScript tests through the bundled Vitest. The version of all tools can be checked using `vp --version`. This is useful when researching documentation, features, and bugs.

## Common Pitfalls

- **Using the package manager directly:** Do not use pnpm, npm, or Yarn directly. Vite+ can handle all package manager operations.
- **Always use Vite commands to run tools:** Don't attempt to run `vp vitest` or `vp oxlint`. They do not exist. Use `vp test` and `vp lint` instead.
- **Running scripts:** Vite+ built-in commands (`vp dev`, `vp build`, `vp test`, etc.) always run the Vite+ built-in tool, not any `package.json` script of the same name. To run a custom script that shares a name with a built-in command, use `vp run <script>`. For example, if you have a custom `dev` script that runs multiple services concurrently, run it with `vp run dev`, not `vp dev` (which always starts Vite's dev server).
- **Do not install Vitest, Oxlint, Oxfmt, or tsdown directly:** Vite+ wraps these tools. They must not be installed directly. You cannot upgrade these tools by installing their latest versions. Always use Vite+ commands.
- **Use Vite+ wrappers for one-off binaries:** Use `vp dlx` instead of package-manager-specific `dlx`/`npx` commands.
- **Import JavaScript modules from `vite-plus`:** Instead of importing from `vite` or `vitest`, all modules should be imported from the project's `vite-plus` dependency. For example, `import { defineConfig } from 'vite-plus';` or `import { expect, test, vi } from 'vite-plus/test';`. You must not install `vitest` to import test utilities.
- **Type-Aware Linting:** There is no need to install `oxlint-tsgolint`, `vp lint --type-aware` works out of the box.

## Review Checklist for Agents

- [ ] Run `vp install` after pulling remote changes and before getting started.
- [ ] Run `vp check` and `vp test` to validate changes.
<!--VITE PLUS END-->
