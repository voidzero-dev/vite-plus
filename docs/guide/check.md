# Check

`vp check` runs format, lint, and type checks together.

## Overview

`vp check` is the default command for fast static checks in Vite+. It brings together formatting through [Oxfmt](https://oxc.rs/docs/guide/usage/formatter.html), linting through [Oxlint](https://oxc.rs/docs/guide/usage/linter.html), and TypeScript type checks through [tsgolint](https://github.com/oxc-project/tsgolint). By merging all of these tasks into a single command, `vp check` is faster than running formatting, linting, and type checking as separate tools in separate commands.

When both `typeAware` and `typeCheck` are enabled in the `lint.options` block in `vite.config.ts`, `vp check` also runs TypeScript type checks through the Oxlint type-aware path powered by the TypeScript Go toolchain and [tsgolint](https://github.com/oxc-project/tsgolint). `vp create` and `vp migrate` enable both `typeAware` and `typeCheck` by default.

We recommend turning both `typeAware` and `typeCheck` on so `vp check` becomes the single command for static checks during development.

## Usage

```bash
vp check
vp check --fix              # Format and run autofixers.
vp check --fix-dangerously  # Format and apply lint fixes, including dangerous fixes and suggestions.
vp check --continue-on-error # Report all enabled checks, even if formatting fails.
vp check --quiet            # Hide lint warnings; still report and fail on errors.
vp check --deny-warnings    # Treat lint warnings as errors.
vp check --format=agent     # Use compact lint and type-check diagnostics for agents.
vp check --no-fmt           # Skip format; run lint (and type-check if enabled).
vp check --no-lint          # Skip lint rules; keep type-check when enabled.
vp check --no-fmt --no-lint # Type-check only (requires both `typeAware` and `typeCheck` enabled).
```

If formatting fails, `vp check` stops before lint and type checks run. To fix formatting and apply available lint fixes, run `vp check --fix`. This command changes files. To skip formatting and run the remaining enabled checks, run `vp check --no-fmt`.

Use `vp check --fix-dangerously` to also apply dangerous lint fixes and suggestions.
This option enables formatting fixes without requiring `--fix`. Dangerous lint fixes
may change program behavior, so review the changes before committing them. When both
flags are passed, `--fix-dangerously` takes precedence for lint fixes.
Like `--fix`, it supports `--no-fmt` and `--continue-on-error`; lint fixes are skipped
when lint rules are disabled with `--no-lint` or `check.lint: false`. Type diagnostics
are not automatically fixed.

Use `vp check --continue-on-error` to run all enabled checks and report formatting, lint,
and type errors in one invocation without modifying files. Type checks still
require the existing `lint.options.typeAware` and `lint.options.typeCheck` settings.
The command exits with a non-zero status if any check fails, even when later
checks pass. Without `--continue-on-error`, the existing fail-fast behavior is preserved.

`--continue-on-error` also works with `--fix`: checks continue after a tool reports a
failure, and formatting runs again after lint fixes. The first failing exit status
is preserved. Options such as `--no-fmt`, `--no-lint`, `--quiet`, and
`--deny-warnings` retain their usual behavior. Configuration loading or command
startup errors still abort the command.

Put `vp check` flags before file paths. For example, use `vp check --fix src/index.ts`. Arguments after the first path go to the format and lint tools instead of `vp check`.

Use `--format <FORMAT>` to choose the output format for lint and type-check diagnostics. Supported formats are `default`, `agent`, `github`, `gitlab`, `json`, `junit`, `sarif`, `stylish`, `unix`, and `checkstyle`. The default keeps the combined check summary, including in CI. Other formats print the selected reporter's output instead of the lint summary; a clean `agent` run produces no lint output. Formatting output is unchanged, so the combined output is not a single machine-readable report. Use `vp lint --format <FORMAT>` when you need a standalone lint report.

## Configuration

`vp check` uses the same configuration you already define for linting and formatting:

- [`lint`](/guide/lint#configuration) block in `vite.config.ts`
- [`fmt`](/guide/fmt#configuration) block in `vite.config.ts`
- TypeScript project structure and tsconfig files for type-aware linting

When run from a workspace package, `vp check` uses the workspace-root `lint` and `fmt` blocks. Package configs cannot replace root format settings, lint rules, or type-check options. File paths and TypeScript projects are still resolved from the package working directory.

Recommended base `lint` config:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: {
    options: {
      typeAware: true,
      typeCheck: true,
    },
  },
});
```

### Disabling a step by default

To make `vp check` skip formatting or linting without passing a flag every time, set the [`check`](/config/check) block in `vite.config.ts`. This is handy when a project wants the rest of the toolchain but not, say, formatting:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  check: {
    fmt: false, // `vp check` lints (and type-checks) but does not format
  },
});
```

These options only affect `vp check`; standalone `vp fmt` and `vp lint` still run normally. A step is skipped if it is disabled in config or the matching `--no-fmt` / `--no-lint` flag is passed. Because the defaults apply to every `vp check` run, a pre-commit hook that calls `vp check` will skip the disabled step too. See [Check Config](/config/check) for the full reference.
