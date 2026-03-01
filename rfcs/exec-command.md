# RFC: `vp exec` Command

## Summary

Add `vp exec` as a subcommand that prepends `./node_modules/.bin` to PATH and executes a command. This is the equivalent of `pnpm exec`.

The command completes the execution story alongside existing commands:

| Command       | Behavior                                                       | Analogy         |
| ------------- | -------------------------------------------------------------- | --------------- |
| `vp dlx`      | Always downloads from remote                                   | `pnpm dlx`      |
| `vpx`         | Local → global → PATH → remote fallback                        | `npx`           |
| **`vp exec`** | **Prepend `node_modules/.bin` to PATH, then execute normally** | **`pnpm exec`** |

## Motivation

Currently, to run a command with `node_modules/.bin` on PATH, developers must use `vpx` (which has global/remote fallback) or call `./node_modules/.bin/<cmd>` directly. There is no simple way to prepend the local bin directory to PATH and execute — the behavior that `pnpm exec` provides.

### Why `vp exec` Is Needed

1. **No remote fallback**: Unlike `vpx`, `vp exec` never downloads from the registry — commands resolve via `node_modules/.bin` + existing PATH only
2. **Workspace iteration**: `pnpm exec --recursive` runs a command in every workspace package — `vpx` doesn't support this
3. **pnpm exec parity**: Projects migrating from pnpm expect `exec` to exist as a subcommand
4. **Explicit intent**: `vp exec` means "run with local bins on PATH" vs `vpx` which means "find it anywhere, download if needed"

### Current Pain Points

```bash
# Developer wants to run with node_modules/.bin on PATH, no remote fallback
vpx eslint .                           # Has remote fallback — may download unexpectedly
./node_modules/.bin/eslint .           # Verbose, not portable

# Developer wants to run a command in every workspace package
pnpm exec --recursive -- eslint .      # Works with pnpm
# No vp equivalent exists today
```

### Proposed Solution

```bash
# Run with node_modules/.bin on PATH (no remote fallback)
vp exec eslint .

# Run in every workspace package
vp exec --recursive -- eslint .

# Shell mode
vp exec -c 'echo $PATH'
```

## Command Syntax

```bash
vp exec [OPTIONS] [--] <command> [args...]
```

The leading `--` is optional and stripped for backward compatibility (matching pnpm exec behavior).

**Options:**

- `--shell-mode, -c` — Execute within a shell environment (`/bin/sh` on UNIX, `cmd.exe` on Windows)
- `--recursive, -r` — Run in every workspace package (local CLI only)
- `--workspace-root, -w` — Run on the workspace root package only (local CLI only)
- `--filter, -F <selector>` — Filter packages by name pattern or relative path (local CLI only); also accepts `--filter=<selector>` form
- `--parallel` — Run concurrently without topological sort (local CLI only)
- `--reverse` — Reverse topological order (local CLI only)
- `--resume-from <pkg>` — Resume from a specific package (local CLI only); also accepts `--resume-from=<pkg>` form
- `--report-summary` — Save results to `vp-exec-summary.json` (local CLI only)

### Usage Examples

```bash
# Basic: run locally installed binary
vp exec eslint .

# With arguments
vp exec tsc --noEmit

# Shell mode (pipe commands, expand variables)
vp exec -c 'echo $PATH'
vp exec -c 'eslint . && prettier --check .'

# Run in every workspace package
vp exec -r -- eslint .

# Filter to specific packages
vp exec --filter 'app...' -- tsc --noEmit

# Filter by relative path
vp exec --filter ./packages/app-a -- tsc --noEmit

# Braced path filter with dependency traversal
vp exec --filter '{./packages/app-a}...' -- tsc --noEmit

# Run in parallel (no topological ordering)
vp exec -r --parallel -- eslint .

# Resume from a specific package (after failure)
vp exec -r --resume-from @my/app -- tsc --noEmit

# Run on workspace root only
vp exec -w -- node -e "console.log(process.env.VITE_PLUS_PACKAGE_NAME)"

# Save execution summary
vp exec -r --report-summary -- vitest run
```

## Filter Selector Syntax

The `--filter` flag supports pnpm-compatible selectors:

**Name patterns:**

- `app-a` — exact package name
- `app-*` — glob pattern matching package names
- `@myorg/*` — scoped package glob

**Path selectors** (detected by leading `.` or `..`):

- `./packages/app-a` — match packages whose directory is at or under this path
- `../other-pkg` — relative path from cwd

**Braced path selectors** (pnpm-compatible syntax):

- `{./packages/app-a}` — equivalent to `./packages/app-a`
- `{./packages/app-a}...` — path with dependency traversal
- `...{./packages/app-a}` — path with dependent traversal
- `app-*{./packages}` — combined name pattern + path filter (match by path first, then filter by name)

**Modifiers:**

- `<selector>...` — include the package and all its transitive dependencies
- `...<selector>` — include the package and all packages that depend on it
- `<selector>^...` — only dependencies, exclude the matched package itself
- `...^<selector>` — only dependents, exclude the matched package itself
- `!<selector>` — exclude matched packages from the result set

Modifiers work with name patterns (e.g., `app-a...`) and braced path selectors (e.g., `{./packages/app-a}...`). Unbraced path selectors (e.g., `./packages/app-a`) do not support traversal modifiers.

**Whitespace splitting**: `--filter "a b"` is equivalent to `--filter a --filter b` (pnpm compatibility). Each `--filter` value is split by whitespace into individual filter tokens.

**Unmatched filter warning**: When an inclusion filter matches no packages, a warning is emitted to stderr (e.g., `WARN No packages matched the filter 'nonexistent'`).

**Exclusion-only filters**: When all selectors are exclusion-only (e.g., `--filter '!app-b'`), the result is all non-root workspace packages minus the excluded ones. This matches pnpm behavior — exclusion without an explicit inclusion implies "start with everything".

**`-w --filter` interaction**: `-w` (workspace root) combined with `--filter` is additive — the workspace root is included alongside the filtered packages. This matches pnpm behavior.

**Workspace root inclusion rules**:

- `-r` (recursive) includes the workspace root along with all workspace packages
- `-w` (workspace root) runs on the workspace root package only
- `--filter '*'` includes the workspace root because `*` name-matches all packages including root

## Core Behavior

Based on pnpm exec behavior (reference: `exec/plugin-commands-script-runners/src/exec.ts`):

1. **Prepend `./node_modules/.bin`** (and extra bin paths from the package manager) to `PATH`
2. **Strip leading `--`** from the command for backward compatibility
3. **Execute command** via process spawn with `stdio: inherit` — the command resolves through the modified PATH (local bins first, then system PATH)
4. **Shell mode**: When `-c` is specified, pass `shell: true` to the child process
5. **Set `VITE_PLUS_PACKAGE_NAME`** env var with the current package name (analogous to pnpm's `PNPM_PACKAGE_NAME`)
6. **Error if no command**: `'vp exec' requires a command to run`

## Relationship Between Commands

| Behavior             | `vp exec`                        | `vpx`                       | `vp dlx`       |
| -------------------- | -------------------------------- | --------------------------- | -------------- |
| Prepend to PATH      | `./node_modules/.bin` (cwd only) | Walk up `node_modules/.bin` | No             |
| Global vp pkg lookup | No                               | Yes                         | No             |
| System PATH          | Yes (after `node_modules/.bin`)  | Yes                         | No             |
| Remote download      | No                               | Yes (fallback)              | Always         |
| Workspace iteration  | Yes (`-r`, `--filter`)           | No                          | No             |
| Shell mode           | Yes (`-c`)                       | Yes (`-c`)                  | Yes (`-c`)     |
| Use case             | Run with local bins on PATH      | Run any tool, find it       | Download & run |

### Key Differences from vpx

- `vp exec` prepends only `./node_modules/.bin` from the current directory — it does **not** walk up parent directories. Use `vpx` if you want monorepo root binaries.
- `vp exec` never falls back to global vp packages or remote download — commands resolve through `node_modules/.bin` + system PATH only.

## Implementation Architecture

### Global CLI

**File**: `crates/vite_global_cli/src/cli.rs`

The `Exec` variant in `Commands` enum (Category C) unconditionally delegates to the local CLI:

```rust
// Category C: Local CLI Delegation
/// Execute a command from local node_modules/.bin
#[command(disable_help_flag = true)]
Exec {
    /// Additional arguments
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
},
```

Route in `execute_command()`:

```rust
Commands::Exec { args } => commands::delegate::execute(cwd, "exec", &args).await,
```

The global CLI always delegates `exec` to the local CLI — there is no fallback path or direct execution in the global CLI. This follows the same unconditional delegation pattern as other Category C commands.

### Local CLI

**Module**: `packages/cli/binding/src/exec/`

The local CLI receives the `exec` command via delegation from the global CLI (same mechanism as `run`, `build`, etc.). The exec logic is organized into a dedicated module with submodules:

```
packages/cli/binding/src/exec/
├── mod.rs       — entry point (execute), delegates to workspace.rs
├── args.rs      — ExecArgs (clap-derived struct with #[clap(flatten)] PackageQueryArgs)
└── workspace.rs — execute_exec_workspace(), topological_sort_packages()
```

There is a single code path for both single-package and multi-package execution. `mod.rs` validates the command is non-empty and delegates to `execute_exec_workspace()`. When no workspace flags (`--recursive`, `--filter`, etc.) are given, `PackageQueryArgs::into_package_query()` returns a `ContainingPackage(cwd)` selector that resolves to just the current package — so the workspace path naturally handles the single-package case.

Package filtering is delegated to `vite_workspace`'s reusable API: `PackageQueryArgs` (CLI args struct, embedded via `#[clap(flatten)]`) → `PackageQuery` (via `into_package_query()`) → `IndexedPackageGraph::resolve_query()` → `FilterResolution` (with `package_subgraph` and `unmatched_selectors`). This follows the same pattern used by `vp run` via `RunFlags`.

The local CLI has full workspace awareness and can handle:

- `--recursive` — iterate workspace packages with topological sort
- `--filter, -F` — filter packages by selector
- `--parallel` — run concurrently
- `--reverse` — reverse topological order
- `--resume-from` — resume from specific package
- `--report-summary` — save results JSON

For the local CLI, exec uses the workspace package graph to iterate packages, prepending each package's `node_modules/.bin` to PATH before spawning the command in that package's directory.

When only a single package is selected (whether by default or via `--filter`), the `pkg_name$ cmd` prefix is suppressed from output and command-not-found errors produce a user-friendly message with a hint to run `vp install` or use `vpx`.

### Reusable Code

The following existing code is reused:

| Module           | Function                           | Purpose                                           |
| ---------------- | ---------------------------------- | ------------------------------------------------- |
| `vite_command`   | `resolve_bin()`                    | Resolve binary path via PATH lookup               |
| `vite_command`   | `build_command()`                  | Build a `tokio::process::Command` for a binary    |
| `vite_command`   | `build_shell_command()`            | Build a shell command for `-c` mode               |
| `vite_install`   | `PackageManager::get_bin_prefix()` | Get package manager bin directory for PATH        |
| `vite_workspace` | `find_workspace_root()`            | Locate workspace root from cwd                    |
| `vite_workspace` | `load_package_graph()`             | Load workspace packages and dependency graph      |
| `vite_workspace` | `PackageQueryArgs`                 | CLI args struct for package selection             |
| `vite_workspace` | `IndexedPackageGraph`              | Indexed graph with `resolve_query()`              |
| `vite_workspace` | `FilterResolution`                 | Resolution result: subgraph + unmatched selectors |

## Design Decisions

### 1. Unconditional Delegation (No Global CLI Fallback)

**Decision**: The global CLI always delegates `exec` to the local CLI. There is no fallback path for projects without vite-plus as a dependency.

**Rationale**:

- Simplifies the global CLI — no need for a direct-execution codepath
- Consistent with how all Category C commands are dispatched
- The local CLI has all the workspace awareness needed for `--recursive`, `--filter`, etc.
- Projects using `vp exec` are expected to have vite-plus installed

### 2. No Directory Walk-Up (Unlike vpx)

**Decision**: `vp exec` only checks `./node_modules/.bin` in the current directory, not parent directories.

**Rationale**:

- Matches `pnpm exec` behavior — strict local scope
- In workspace iteration (`-r`), each package should use its own `node_modules/.bin`
- Walking up would blur the boundary between package-level and workspace-level binaries
- Use `vpx` if you want walk-up behavior

### 3. Workspace Features Only via Local CLI

**Decision**: `--recursive`, `--workspace-root`, `--filter`, `--parallel`, `--reverse`, `--resume-from`, and `--report-summary` only work when vite-plus is a local dependency (local CLI handles them).

**Rationale**:

- These features require workspace awareness from vite-task infrastructure
- The global CLI fallback is for simple, single-directory exec
- This is consistent with how `vp run` handles workspace features

### 4. Same Env Var Convention

**Decision**: Set `VITE_PLUS_PACKAGE_NAME` env var when executing in a workspace package.

**Rationale**:

- Follows pnpm's `PNPM_PACKAGE_NAME` convention
- Allows scripts to know which package they're running in
- Consistent naming with vite-plus branding

### 5. Strip Leading `--`

**Decision**: Automatically strip a leading `--` from the command arguments.

**Rationale**:

- Matches pnpm exec backward compatibility behavior
- `vp exec -- eslint .` and `vp exec eslint .` should behave identically
- Reduces friction for users coming from pnpm

### 6. Execution Ordering

**Decision**: When `--recursive` or `--filter` is used, packages execute in topological order (dependencies first). The topological sort uses `petgraph::algo::toposort` on the `FilterResolution.package_subgraph` (not the original full graph), enabling future `--filter-prod` support where dev dependency edges are excluded at subgraph construction time.

**Rationale**:

- **Topological ordering by default**: Commands like `tsc --noEmit` or `build` need dependencies to complete before dependents. Running in dependency order ensures correctness without requiring users to specify `--topological` explicitly.
- **No alphabetical tie-breaking**: Packages with no ordering constraint between them (e.g., two unrelated leaf packages) are ordered by petgraph's internal traversal order. This matches pnpm's behavior.
- **`--parallel` skips ordering**: In parallel mode, all packages are spawned concurrently — topological order only affects the order of output collection.
- **`--reverse`**: Reverses the topological order (dependents first, then dependencies). Useful for cleanup operations.
- **Circular dependency handling**: When cycles exist, `toposort()` returns an error. The fallback uses `petgraph::algo::tarjan_scc`, which returns strongly connected components (SCCs) in reverse topological order of the condensed DAG. This preserves correct ordering for non-cyclic dependencies even when cycles are present — nodes outside a cycle are correctly placed before or after the cycle based on their dependency relationship.

  **Example — normal dependency chain (no cycle):**

  ```
  a → b → c → d → e    (a depends on b, b depends on c, ...)

  toposort produces dependencies-first order:
  Result: [e, d, c, b, a]
  ```

  **Example — simple cycle (2 nodes):**

  ```
  a ←→ b    (mutual dependency)

  toposort returns Err(Cycle).
  tarjan_scc returns [{a, b}] — one SCC containing both nodes.
  Result: [a, b] or [b, a]  (intra-SCC order is arbitrary)
  ```

  **Example — 3-node cycle:**

  ```
  a → b → c → a    (a depends on b, b depends on c, c depends on a)

  toposort returns Err(Cycle).
  tarjan_scc returns [{a, b, c}] — all three form one SCC.
  Result: [a, b, c] in any permutation  (intra-SCC order is arbitrary)
  ```

  **Example — cycle with a non-cyclic dependency:**

  ```
  a ←→ b, a → c    (a↔b cycle, a depends on non-cyclic c)

  toposort returns Err(Cycle).
  tarjan_scc returns [{c}, {a, b}] — c as its own SCC first, then
  the a↔b cycle.  Dependencies-first order is preserved.
  Result: [c, a, b] or [c, b, a]  (c always before the cycle)
  ```

  **Example — cycle with a non-cyclic dependent:**

  ```
  a ←→ b ← aa    (a↔b cycle, aa depends on b)

  toposort returns Err(Cycle).
  tarjan_scc returns [{a, b}, {aa}] — the cycle SCC first, then aa.
  Result: [a, b, aa] or [b, a, aa]  (cycle always before aa)
  ```

- **Platform-safe PATH construction**: PATH environment variable is constructed using `std::env::join_paths()` instead of hardcoded `:` separator, ensuring correct behavior on both Unix (`:`) and Windows (`;`).

## CLI Help Output

```bash
$ vp exec --help
Execute a command from local node_modules/.bin

Usage: vp exec [OPTIONS] [--] <command> [args...]

Arguments:
  <command>  Command to execute from node_modules/.bin
  [args...]  Arguments to pass to the command

Options:
  -c, --shell-mode              Execute the command within a shell environment
  -r, --recursive               Run in every workspace package
  -w, --workspace-root          Run on the workspace root package only
  -F, --filter <PATTERN>        Filter packages (can be used multiple times)
      --parallel                Run concurrently without topological ordering
      --reverse                 Reverse execution order
      --resume-from <PACKAGE>   Resume from a specific package
      --report-summary          Save results to vp-exec-summary.json
  -h, --help                    Print help

Examples:
  vp exec eslint .                            # Run local eslint
  vp exec tsc --noEmit                        # Run local TypeScript compiler
  vp exec -c 'eslint . && prettier --check .' # Shell mode
  vp exec -r -- eslint .                      # Run in all workspace packages
  vp exec --filter 'app...' -- tsc            # Run in filtered packages
```

## Error Handling

### Missing Command

```bash
$ vp exec
Error: 'vp exec' requires a command to run

Usage: vp exec [--] <command> [args...]

Examples:
  vp exec eslint .
  vp exec tsc --noEmit
```

### Command Not Found

```bash
$ vp exec nonexistent-cmd
Error: Command 'nonexistent-cmd' not found

Hint: Run 'vp install' to install dependencies, or use 'vpx' for remote fallback.
```

## Snap Tests

### Global CLI Test: `command-exec-pnpm10`

**Location**: `packages/cli/snap-tests-global/command-exec-pnpm10/`

```
command-exec-pnpm10/
├── package.json
├── steps.json
└── snap.txt          # auto-generated
```

**`package.json`**:

```json
{
  "name": "command-exec-pnpm10",
  "version": "1.0.0",
  "packageManager": "pnpm@10.19.0"
}
```

**`steps.json`**:

```json
{
  "env": {
    "VITE_DISABLE_AUTO_INSTALL": "1"
  },
  "commands": [
    "vp exec echo hello # basic exec, no vite-plus dep (global CLI handles directly)",
    "vp exec node -e \"console.log('hi')\" # exec with args passthrough",
    "vp exec nonexistent-cmd # command not found error",
    "vp exec -c 'echo hello from shell' # shell mode"
  ]
}
```

**Test cases**:

1. `vp exec echo hello` — basic execution with a command found on PATH after `node_modules/.bin` prepend
2. `vp exec node -e "console.log('hi')"` — argument passthrough to a multi-arg command
3. `vp exec nonexistent-cmd` — command-not-found error message
4. `vp exec -c 'echo hello from shell'` — shell mode execution

### Local CLI Test: `command-exec`

**Location**: `packages/cli/snap-tests/command-exec/`

```
command-exec/
├── package.json
├── steps.json
└── snap.txt          # auto-generated
```

**`package.json`**:

```json
{
  "name": "command-exec",
  "version": "1.0.0",
  "packageManager": "pnpm@10.19.0",
  "devDependencies": {
    "vite-plus": "workspace:*",
    "cowsay": "^1.6.0"
  }
}
```

**`steps.json`**:

```json
{
  "commands": [
    "vp exec cowsay hello # exec with installed binary",
    "vp exec -c 'echo $PATH' # verify PATH includes node_modules/.bin"
  ]
}
```

**Test cases**:

1. `vp exec cowsay hello` — execute locally installed binary via local CLI delegation
2. `vp exec -c 'echo $PATH'` — verify that `node_modules/.bin` is prepended to PATH

## Edge Cases

### Leading `--` Stripping

```bash
# Both are equivalent
vp exec -- eslint .
vp exec eslint .
```

### Shell Mode with Complex Commands

```bash
# Pipes and redirects require shell mode
vp exec -c 'eslint . 2>&1 | tee lint-output.txt'

# Environment variable expansion
vp exec -c 'echo $NODE_ENV'
```

### Recursive with Failures

When running recursively, a failure in one package stops execution (unless `--parallel` is used, in which case all packages run and failures are collected):

```bash
$ vp exec -r -- tsc --noEmit
@my/utils: tsc --noEmit ... ok
@my/app: tsc --noEmit ... FAILED (exit code 1)
Error: 1 of 5 packages failed
```

### Empty args After `--`

```bash
$ vp exec --
Error: 'vp exec' requires a command to run
```

## Security Considerations

1. **No remote fallback**: Unlike `vpx`, `vp exec` never downloads from the registry, eliminating supply chain risk from accidental remote execution
2. **PATH behavior**: Commands resolve through `./node_modules/.bin` (prepended) + system PATH. This matches `pnpm exec` behavior — system commands like `echo`, `node`, etc. are still reachable
3. **Shell mode risks**: Shell mode (`-c`) allows arbitrary shell commands — same considerations as pnpm exec

## Backward Compatibility

This is a new feature with no breaking changes:

- Existing `vp dlx` and `vpx` behavior unchanged
- New `exec` subcommand is purely additive
- No changes to configuration format
- Follows established delegation pattern (like `vp run`)

## Comparison with pnpm exec

| Behavior              | `pnpm exec`                              | `vp exec`                                |
| --------------------- | ---------------------------------------- | ---------------------------------------- |
| PATH modification     | Prepend `./node_modules/.bin`            | Prepend `./node_modules/.bin`            |
| Command resolution    | Modified PATH (local bins + system PATH) | Modified PATH (local bins + system PATH) |
| Walk-up               | No                                       | No                                       |
| Shell mode (`-c`)     | Yes                                      | Yes                                      |
| Recursive (`-r`)      | Yes (workspace iteration)                | Yes (via local CLI)                      |
| Workspace root (`-w`) | Yes (root only)                          | Yes (root only)                          |
| Filter                | `--filter`                               | `--filter`                               |
| Path-based filter     | `--filter ./packages/app`                | `--filter ./packages/app`                |
| Braced path filter    | `--filter {./packages/app}`              | `--filter {./packages/app}`              |
| Name + path filter    | `--filter 'app-*{./packages}'`           | `--filter 'app-*{./packages}'`           |
| Parallel              | `--parallel`                             | `--parallel`                             |
| Report summary        | `--report-summary`                       | `--report-summary`                       |
| Package name env var  | `PNPM_PACKAGE_NAME`                      | `VITE_PLUS_PACKAGE_NAME`                 |
| Strip leading `--`    | Yes                                      | Yes                                      |

## Future Enhancements

### 1. `--if-present` Flag

```bash
# Skip packages where the command doesn't exist (useful with -r)
vp exec -r --if-present -- eslint .
```

## Conclusion

This RFC proposes adding `vp exec` to complete the execution command trio in Vite+:

- `vp dlx` — always remote (like `pnpm dlx`)
- `vpx` — local-first with fallback chain (like `npx`)
- `vp exec` — prepend local bins to PATH, no remote fallback (like `pnpm exec`)

The design:

- Matches `pnpm exec` semantics for familiar developer experience
- Follows the established unconditional delegation pattern for global/local CLI routing
- Reuses existing infrastructure (`vpx.rs` helpers, delegation, PATH manipulation)
- Supports workspace features (recursive, filter, parallel) via local CLI
- Is purely additive with no breaking changes
