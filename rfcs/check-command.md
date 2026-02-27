# RFC: `vp check` Command

## Summary

Add `vp check` as a built-in command that runs format verification, linting, and type checking in a single invocation. This provides a single "fast check" command for CI and local development, distinct from "slow checks" like test suites.

## Motivation

Currently, running a full code quality check requires chaining multiple commands:

```bash
# From the monorepo template's "ready" script:
vp fmt && vp lint --type-aware && vp run test -r && vp run build -r
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
vp check --fix --no-lint    # Only fix formatting

# Disable specific checks
vp check --no-fmt
vp check --no-lint
vp check --no-type-aware
vp check --no-type-check
```

### Options

| Flag                               | Default | Description                                             |
| ---------------------------------- | ------- | ------------------------------------------------------- |
| `--fix`                            | OFF     | Auto-fix format and lint issues                         |
| `--fmt` / `--no-fmt`               | ON      | Run format check (`vp fmt --check`)                     |
| `--lint` / `--no-lint`             | ON      | Run lint check (`vp lint`)                              |
| `--type-aware` / `--no-type-aware` | ON      | Enable type-aware lint rules (oxlint `--type-aware`)    |
| `--type-check` / `--no-type-check` | ON      | Enable TypeScript type checking (oxlint `--type-check`) |

**Flag dependency:** `--type-check` requires `--type-aware` as a prerequisite.

- `--type-aware` enables lint rules that use type information (e.g., `no-floating-promises`)
- `--type-check` enables experimental TypeScript compiler-level type checking (requires type-aware)
- If `--no-type-aware` is set, `--type-check` is also implicitly disabled

Both are enabled by default in `vp check` to provide comprehensive static analysis.

### File Path Arguments

`vp check` accepts optional trailing file paths, which are passed through to `fmt` and `lint`:

```bash
# Check only specific files
vp check --fix src/index.ts src/utils.ts
```

When file paths are provided:

- `--no-error-on-unmatched-pattern` is automatically added to `fmt` args (prevents errors when paths don't match fmt patterns)
- Paths are appended to both `fmt` and `lint` sub-commands

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
2. vp lint --type-aware --type-check       (lint + type checking)
```

If any step fails, `vp check` exits immediately with a non-zero exit code.

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

Add `Check` variant to `Commands` enum in `crates/vite_global_cli/src/cli.rs`:

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

Add `Check` to `SynthesizableSubcommand` in `packages/cli/binding/src/cli.rs`. The check command internally resolves and runs fmt + lint sequentially, reusing existing resolvers.

### TypeScript Side

No new resolver needed — `vp check` reuses existing `resolve-lint.ts` and `resolve-fmt.ts`.

### Key Files to Modify

1. `crates/vite_global_cli/src/cli.rs` — Add `Check` command variant and routing
2. `packages/cli/binding/src/cli.rs` — Add check subcommand handling (sequential fmt + lint)
3. `packages/cli/src/bin.ts` — (if needed for routing)

## CLI Help Output

```
Run format, lint, and type checks

Usage: vp check [OPTIONS]

Options:
      --fmt              Run format check [default: true]
      --lint             Run lint check [default: true]
      --type-aware       Enable type-aware linting [default: true]
      --type-check       Enable TypeScript type checking [default: true]
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
"ready": "vp check && vp run test -r && vp run build -r"
```

## Comparison with Other Tools

| Tool              | Scope                              |
| ----------------- | ---------------------------------- |
| `cargo check`     | Type checking only                 |
| `cargo clippy`    | Lint only                          |
| **`biome check`** | **Format + lint (closest analog)** |
| `deno check`      | Type checking only                 |

## Snap Tests

```
packages/cli/snap-tests/check-basic/
  package.json
  steps.json     # { "steps": [{ "command": "vp check" }] }
  src/index.ts   # Clean file that passes all checks
  snap.txt

packages/cli/snap-tests/check-fmt-fail/
  package.json
  steps.json     # { "steps": [{ "command": "vp check" }] }
  src/index.ts   # Badly formatted file
  snap.txt       # Shows fmt --check failure, lint doesn't run (fail-fast)

packages/cli/snap-tests/check-no-fmt/
  package.json
  steps.json     # { "steps": [{ "command": "vp check --no-fmt" }] }
  snap.txt       # Only lint runs
```
