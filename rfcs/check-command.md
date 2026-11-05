# RFC: `vp check` Command

## Summary

Add `vp check` as a built-in command that runs format verification, linting, and type checking in a single invocation. This provides a single "fast check" command for CI and local development, distinct from "slow checks" like test suites.

## Motivation

Currently, running a full code quality check requires chaining multiple commands:

```bash
# From the monorepo template's "ready" script:
vp fmt && vp lint --type-aware && vp run -r test && vp run -r build
```

Pain points:

- **No single command** for the most common pre-commit/CI check: "is my code correct?"
- Users must remember to pass `--type-aware` and `--type-check` to lint
- The `&&` chaining pattern is fragile and verbose
- No standardized "check" workflow across projects

### Fast vs Slow Checks

- **Fast checks** (seconds): type checking + linting + formatting — static analysis, no code execution
- **Slow checks** (minutes): test suites (Vitest) — code execution

`vp check` targets the **fast checks** category. Tests are explicitly excluded — use `vp test` for that.

## Command Syntax

```bash
# Run all fast checks (fmt --check + lint --type-aware --type-check)
vp check

# Auto-fix format and lint issues
vp check --fix
vp check --fix --no-lint    # Skip lint rules; configured type checks still run

# Disable specific checks
vp check --no-fmt
vp check --no-lint
```

### Options

| Flag                              | Default | Description                                               |
| --------------------------------- | ------- | --------------------------------------------------------- |
| `--fix`                           | OFF     | Auto-fix formatting and lint issues                       |
| `--no-fmt`                        | OFF     | Skip formatting                                           |
| `--no-lint`                       | OFF     | Skip lint rules; retain configured TypeScript diagnostics |
| `--quiet`                         | OFF     | Report errors without warnings                            |
| `--deny-warnings`                 | OFF     | Treat lint warnings as errors                             |
| `--no-error-on-unmatched-pattern` | OFF     | Do not fail when no files match                           |

Type-aware rules and TypeScript diagnostics follow `lint.options` in
`vite.config.ts`. `vp create` and `vp migrate` enable both settings by default:

```ts
export default {
  lint: {
    options: {
      typeAware: true,
      typeCheck: true,
    },
  },
};
```

`typeAware` enables rules that need type information, such as
`typescript/no-floating-promises`. `typeCheck` also enables TypeScript compiler
diagnostics. Set either option to `false` in the config to disable it.
`vp check` does not expose `--no-type-aware` or `--no-type-check` flags.

### File Path Arguments

`vp check` accepts optional trailing file paths, which are passed through to `fmt` and `lint`:

```bash
# Check only specific files
vp check --fix src/index.ts src/utils.ts
```

When file paths are provided:

- Paths are appended to both `fmt` and `lint` sub-commands
- In `--fix` mode, `--no-error-on-unmatched-pattern` is implicitly enabled for both `fmt` and `lint`, preventing errors when all provided paths are excluded by ignorePatterns. This is the common lint-staged use case where staged files may not match tool-specific patterns.
- Without `--fix`, unmatched patterns are reported as errors unless `--no-error-on-unmatched-pattern` is explicitly passed. Both oxfmt and oxlint support this flag natively.

This enables lint-staged integration:

```json
"lint-staged": {
  "*.@(js|ts|tsx)": "vp check --fix"
}
```

lint-staged appends staged file paths automatically, so `vp check --fix` becomes e.g. `vp check --fix src/a.ts src/b.ts`.

## Behavior

Commands run **sequentially** with fail-fast semantics:

```
1. vp fmt --check                          (verify formatting, don't auto-fix)
2. vp lint                               (lint + configured type checking)
```

If any step fails, `vp check` exits immediately with a non-zero exit code.

## CLI Output

`vp check` should print **completion summaries only** for successful phases:

```text
pass: All 989 files are correctly formatted (423ms, 16 threads)
pass: Found no warnings, lint errors, or type errors in 150 files (452ms, 16 threads)
```

Output rules:

- Do not print delegated commands such as `vp fmt --check` or `vp lint --type-aware --type-check`
- Print one `pass:` line only after a phase completes successfully
- Mention type checks in the lint success line only when `lint.options.typeCheck` is enabled
- On failure, print a human-readable `error:` line, then raw diagnostics, then a blank line and a final summary sentence
- With `--no-fmt --no-lint`, run only configured TypeScript diagnostics. Report an error if no checks remain enabled.

Representative failure output:

```text
error: Formatting issues found
src/index.js
steps.json

Found formatting issues in 2 files (105ms, 16 threads). Run `vp check --fix` to fix them.
```

```text
error: Lint or type issues found
...diagnostics...

Found 3 errors and 1 warning in 2 files (452ms, 16 threads)
```

## Decisions

### Dual mode: verify and fix

By default, `vp check` is a **read-only verification** command. It never modifies files:

- `vp fmt --check` reports unformatted files (doesn't auto-format)
- `vp lint --type-aware --type-check` reports issues (doesn't auto-fix)

This keeps `vp check` safe for CI and predictable for local dev.

With `--fix`, `vp check` switches to **auto-fix** mode:

- `vp fmt` auto-formats files
- `vp lint --fix --type-aware --type-check` auto-fixes lint issues

This replaces the manual `vp fmt && vp lint --fix` workflow with a single command.

### No tests

`vp check` does **not** run Vitest. The distinction is intentional:

- `vp check` = fast static analysis (seconds)
- `vp test` = test execution (minutes)

## Implementation Architecture

### Rust Global CLI

Add `Check` variant to `Commands` enum in `crates/vp_global_cli/src/cli.rs`:

```rust
#[command(disable_help_flag = true)]
Check {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
},
```

Route via delegation:

```rust
Commands::Check { args } => commands::delegate::execute(cwd, "check", &args).await,
```

### NAPI Binding

The `Check` variant is defined in `SynthesizableSubcommand` in `packages/cli/binding/src/cli/types.rs`. The check command's orchestration logic lives in its own module at `packages/cli/binding/src/check/`, following the same directory-per-command pattern as `exec/`:

- `check/mod.rs` — `execute_check()` orchestration (runs fmt + lint sequentially, handles `--fix` re-formatting)
- `check/analysis.rs` — Output analysis types (`CheckSummary`, `LintMessageKind`, etc.), parsers, and formatting helpers

The check module reuses `SubcommandResolver` and `resolve_and_capture_output` from `cli/resolver.rs` and `cli/execution.rs` to resolve and run the underlying fmt/lint commands.

### TypeScript Side

`vp check` reuses `resolve-lint.ts` and `resolve-fmt.ts`.

### Experimental type-aware backend

This draft replaces the bundled Go backend with `tsrs@0.22.0-ts7.1.0-beta`,
a Rust port of the TypeScript 7 compiler. `resolve-lint.ts` selects the bundled
`tsrs` package and passes its `bin/tsrs-lint` launcher to Oxlint through
`OXLINT_TSGOLINT_PATH`. The same resolver serves `vp lint` and `vp check`.
Users do not need to set an environment variable for the default backend.

An explicit `OXLINT_TSGOLINT_PATH` takes precedence over the resolver default.
The environment variable keeps its existing name because Oxlint uses it to
select the type-aware backend. A project can still select Go explicitly:

```sh
vp add -D oxlint-tsgolint
OXLINT_TSGOLINT_PATH="$PWD/node_modules/.bin/tsgolint" vp check
```

`tsrs` is a production dependency. `oxlint-tsgolint` remains a development
dependency for comparisons. The benchmark selects the Go executable explicitly,
so a change to the default cannot make it compare tsrs against itself.
See the [performance report](../bench/tsrs-lint.md) for elapsed time, peak memory,
and measurement conditions.

The pinned tsrs release has no Windows ARM64 binary. This draft adds no automatic
Go fallback. A release for all supported Vite+ platforms needs an upstream
Windows ARM64 binary or an explicit fallback. Runtime smoke tests cover macOS
ARM64. The Codespaces comparison also validates both backends on Linux x64
with Vite+ and vinext. Other platforms still need runtime validation.

The backend switch preserves the existing formatting phase, lint configuration,
and failure handling. Smoke fixtures compare diagnostics, suppression comments,
and automatic fixes against Go. These checks do not establish complete
compiler parity.

### Key Files

1. `crates/vp_global_cli/src/cli.rs` — `Check` command variant and routing
2. `packages/cli/binding/src/cli/types.rs` — `SynthesizableSubcommand::Check` definition, delegates to `check` module
3. `packages/cli/binding/src/check/mod.rs` — Check command orchestration (`execute_check`)
4. `packages/cli/binding/src/check/analysis.rs` — Output parsing and analysis types

## CLI Help Output

```
Run format, lint, and type checks

Usage: vp check [OPTIONS] [PATHS]...

Options:
      --fix              Auto-fix format and lint issues
      --quiet            Report errors without warnings
      --deny-warnings    Treat lint warnings as errors
      --no-fmt           Skip format check
      --no-lint          Skip lint rules; retain configured type checks
      --no-error-on-unmatched-pattern  Do not exit with error when no files match
  -h, --help             Print help
```

## Relationship to Existing Commands

| Command                             | Purpose                                          | Speed    |
| ----------------------------------- | ------------------------------------------------ | -------- |
| `vp fmt`                            | Format code (auto-fix)                           | Fast     |
| `vp fmt --check`                    | Verify formatting                                | Fast     |
| `vp lint`                           | Lint code                                        | Fast     |
| `vp lint --type-aware --type-check` | Lint + full type checking                        | Fast     |
| `vp test`                           | Run test suite                                   | Slow     |
| `vp build`                          | Build project                                    | Slow     |
| **`vp check`**                      | **fmt --check + lint --type-aware --type-check** | **Fast** |
| **`vp check --fix`**                | **fmt + lint --fix --type-aware --type-check**   | **Fast** |

With `vp check`, the monorepo template's "ready" script simplifies to:

```json
"ready": "vp check && vp run -r test && vp run -r build"
```

## Caching

When `vp check` is used as a package.json script (e.g., `"check": "vp check"`) and executed via `vp run check`, it supports task runner caching like other synthesized commands (`vp build`, `vp lint`, `vp fmt`).

### Configuration

Enable caching in `vite.config.ts`:

```ts
export default {
  run: {
    cache: true,
  },
};
```

With caching enabled, the second `vp run check` replays cached output when inputs haven't changed:

```
$ vp check ◉ cache hit, replaying
pass: All 4 files are correctly formatted (105ms, 16 threads)
pass: Found no warnings or lint errors in 2 files (452ms, 16 threads)
```

### Cache key

The check command's cache fingerprint includes:

- **Environment variable:** `OXLINT_TSGOLINT_PATH` (an explicit Go or tsrs backend override affects lint behavior)
- **Input files:** Auto-tracked via fspy, excluding:
  - `node_modules/.vite/task-cache/**` — task runner state files that change after each run

These exclusions are defined by `check_cache_inputs()` in `packages/cli/binding/src/cli/resolver.rs`.

### How it differs from `vp fmt` / `vp lint`

When `vp fmt` or `vp lint` appear in task scripts, the command handler resolves them to their underlying binaries (e.g., `node path/to/oxfmt.mjs`). The `vp check` command is different — it runs as a full `vp check` subprocess because it's a composite command that orchestrates both fmt and lint internally. This means the `vp` CLI process itself is tracked by fspy, which is why the `.vite/task-cache` exclusions are necessary.

## Comparison with Other Tools

| Tool              | Scope                              |
| ----------------- | ---------------------------------- |
| `cargo check`     | Type checking only                 |
| `cargo clippy`    | Lint only                          |
| **`biome check`** | **Format + lint (closest analog)** |
| `deno check`      | Type checking only                 |

## Snapshot Tests

CLI cases live in `crates/vp_cli_snapshots/tests/cli_snapshots/fixtures/`.
Each case uses `snapshots.toml` and a recorded Markdown snapshot.

| Fixture                   | Coverage                                               |
| ------------------------- | ------------------------------------------------------ |
| `check_pass_typecheck`    | Formatting, linting, and configured type checks        |
| `check_fmt_fail`          | Formatting failure stops the command                   |
| `check_no_fmt`            | Linting without formatting                             |
| `check_no_lint_typecheck` | Type checks without lint rules                         |
| `check_oxlint_env`        | An explicit backend path overrides the bundled default |
| `check_cache_enabled`     | A second `vp run check` replays cached output          |
| `oxlint_typeaware`        | Type-aware task caching tracks imported type files     |

Use the [snapshot runner instructions](../crates/vp_cli_snapshots/tests/cli_snapshots/README.md)
to run or update these cases.
