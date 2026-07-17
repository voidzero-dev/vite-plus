# command_outdated_bun

## `vp outdated --help`

should show help

```
VITE+ - The Unified Toolchain for the Web

Usage: vp outdated [OPTIONS] [PACKAGES]... [-- <PASS_THROUGH_ARGS>...]

Check for outdated packages

Arguments:
  [PACKAGES]...           Package name(s) to check
  [PASS_THROUGH_ARGS]...  Additional arguments to pass through to the package manager

Options:
  --long                       Show extended information
  --format <FORMAT>            Output format: table (default), list, or json
  -r, --recursive              Check recursively across all workspaces
  --filter <PATTERN>           Filter packages in monorepo
  -w, --workspace-root         Include workspace root
  -P, --prod                   Only production and optional dependencies
  -D, --dev                    Only dev dependencies
  --no-optional                Exclude optional dependencies
  --compatible                 Only show compatible versions
  --sort-by <FIELD>            Sort results by field
  -g, --global                 Check globally installed packages
  --concurrency <CONCURRENCY>  Number of global package checks to run in parallel (only with -g)
  -h, --help                   Print help

Documentation: https://viteplus.dev/guide/install
```

## `vp install`

should install packages first

```
VITE+ - The Unified Toolchain for the Web

bun install <version> (af24e281)

 test-vite-plus-top-package@1.0.0 (<version> available)
 test-vite-plus-other-optional@1.0.0 (<version> available)
 testnpm2@1.0.0 (<version> available)

4 packages installed [<duration>]
```

## `vp outdated testnpm2`

should show outdated package

```
bun outdated <version> (af24e281)
┌──────────┬─────────┬────────┬────────┐
│ Package  │ Current │ Update │ Latest │
├──────────┼─────────┼────────┼────────┤
│ testnpm2 │ 1.0.0   │ 1.0.0  │ 1.0.1  │
└──────────┴─────────┴────────┴────────┘
```

## `vp outdated -r`

should support recursive output

```
bun outdated <version> (af24e281)
┌──────────────────────────────────────────┬─────────┬────────┬────────┬──────────────────────┐
│ Package                                  │ Current │ Update │ Latest │ Workspace            │
├──────────────────────────────────────────┼─────────┼────────┼────────┼──────────────────────┤
│ testnpm2                                 │ 1.0.0   │ 1.0.0  │ 1.0.1  │ command-outdated-bun │
├──────────────────────────────────────────┼─────────┼────────┼────────┼──────────────────────┤
│ test-vite-plus-top-package (dev)         │ 1.0.0   │ 1.0.0  │ 1.1.0  │ command-outdated-bun │
├──────────────────────────────────────────┼─────────┼────────┼────────┼──────────────────────┤
│ test-vite-plus-other-optional (optional) │ 1.0.0   │ 1.0.0  │ 1.1.0  │ command-outdated-bun │
└──────────────────────────────────────────┴─────────┴────────┴────────┴──────────────────────┘
```
