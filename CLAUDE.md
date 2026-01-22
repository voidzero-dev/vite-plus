# Vite-Plus

A monorepo task runner (like nx/turbo) with intelligent caching and dependency resolution.

## Core Concept

**Task Execution**: Run tasks across monorepo packages with automatic dependency ordering.

```bash
# Built-in commands
vite build                           # Run Vite build (dedicated command)
vite test                            # Run Vite test (dedicated command)
vite lint                            # Run oxlint (dedicated command)

# Run tasks across packages (explicit mode)
vite run build -r                    # recursive with topological ordering
vite run app#build web#build         # specific packages
vite run build -r --no-topological   # recursive without implicit deps

# Run task in current package (implicit mode - for non-built-in tasks)
vite dev                             # runs dev script from package.json
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

## Build

- Run `pnpm bootstrap-cli` from the project root to build all packages and install the global CLI
  - This builds all `@voidzero-dev/*`, `vite-plus` and `vite-plus-cli` packages
  - Compiles the Rust NAPI bindings
  - Installs the CLI globally via npm

## Snap Tests

Snap tests are located in `packages/global/snap-tests/` and `packages/cli/snap-tests/`. Each test case is a directory containing:

- `package.json` - Package configuration for the test
- `steps.json` - Commands to run and environment variables
- `src/` - Source files for the test
- `snap.txt` - Expected output (generated/updated by running the test)

```bash
# Run all global snap tests (for local cli, the package name would be vite-plus)
pnpm -F vite-plus-cli snap-test

# Run a specific snap test by name filter
pnpm -F vite-plus-cli snap-test migration-skip-vite-peer-dependency
```

The snap test will automatically generate/update the `snap.txt` file with the command outputs. It exits with zero status even if there are output differences; you need to manually check the diffs(`git diff`) to verify correctness.

## Ecosystem-CI Tests

Ecosystem-CI tests real-world projects against vite-plus. Located in `ecosystem-ci/`.

### Adding a New Test Case

1. **Get repository info** using GitHub CLI:
   ```bash
   gh api repos/OWNER/REPO --jq '.default_branch'
   gh api repos/OWNER/REPO/commits/BRANCH --jq '.sha'
   ```

2. **Add entry to `ecosystem-ci/repo.json`**:
   ```json
   {
     "project-name": {
       "repository": "https://github.com/owner/repo.git",
       "branch": "main",
       "hash": "full-commit-sha",
       "directory": "web"  // optional: subdirectory containing package.json
     }
   }
   ```

3. **Add test matrix entry to `.github/workflows/e2e-test.yml`**:
   ```yaml
   - name: project-name
     node-version: 24
     directory: web  # optional: must match repo.json if specified
     command: |
       vite run lint
       vite run build
   ```

4. **Test locally**:
   ```bash
   pnpm dlx tsx ecosystem-ci/clone.ts project-name
   ```

### Key Files

- `ecosystem-ci/repo.json` - Project definitions (repository, branch, hash, optional directory)
- `ecosystem-ci/clone.ts` - Clones projects to temp directory
- `ecosystem-ci/patch-project.ts` - Runs `vite migrate` on cloned projects
- `.github/workflows/e2e-test.yml` - CI workflow with test matrix

### Subdirectory Support

For projects where the web app is in a subdirectory (e.g., `dify/web`):
- Add `"directory": "subdir"` to repo.json entry
- Add `directory: subdir` to workflow matrix entry
- `patch-project.ts` and workflow steps automatically use the subdirectory
