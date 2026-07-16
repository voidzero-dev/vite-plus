# test_panicked_fix

## `vp lint --help`

print help message and no panicked

```
VITE+ - The Unified Toolchain for the Web

Usage: vp lint [PATH]... [OPTIONS]

Lint code.
Options are forwarded to Oxlint.

Options:
  --tsconfig <PATH>                Override the TypeScript config
  --fix                            Fix issues when possible
  --fix-suggestions                Apply auto-fixable suggestions
  --fix-dangerously                Apply dangerous fixes and suggestions
  --type-aware                     Enable rules requiring type information
  --type-check                     Enable experimental type checking
  --import-plugin                  Enable the import plugin
  --no-error-on-unmatched-pattern  Do not exit with error when no files are selected
  --rules                          List registered rules
  -h, --help                       Print help

Examples:
  vp lint
  vp lint src --fix
  vp lint --type-aware --tsconfig ./tsconfig.json

Documentation: https://viteplus.dev/guide/lint
```
