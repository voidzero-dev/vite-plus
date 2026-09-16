# command_no_color

## `vp check --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp check [OPTIONS] [PATHS]...

Run format, lint, and type checks.

Arguments:
  [PATHS]...  File paths to pass to fmt and lint

Options:
  --fix                            Auto-fix format and lint issues
  --quiet                          Disable reporting on warnings, only errors are reported
  --no-fmt                         Skip format check
  --no-lint                        Skip lint rules; type-check still runs when `lint.options.typeCheck` is true
  --no-error-on-unmatched-pattern  Do not exit with error when pattern is unmatched
  -h, --help                       Print help

Examples:
  vp check
  vp check --fix
  vp check --no-lint src/index.ts

Documentation: https://viteplus.dev/guide/check
```

## `vp --definitely-invalid`

Argument errors and highlighted arguments honor NO_COLOR

**Exit code:** 2

```
VITE+ - The Unified Toolchain for the Web

error: Unexpected argument \'--definitely-invalid\'
```

## `vp check --no-fmt --no-lint`

Shared errors and command-specific summaries honor NO_COLOR

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: No checks enabled

Enable `lint.options.typeCheck` in vite.config.ts for type-check only, drop a `--no-fmt`/`--no-lint` flag, or re-enable `check.fmt`/`check.lint` in vite.config.ts.
```
