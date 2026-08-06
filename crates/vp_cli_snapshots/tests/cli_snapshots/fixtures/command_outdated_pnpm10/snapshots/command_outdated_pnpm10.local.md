# command_outdated_pnpm10

## `vp outdated --help`

should show help

```
Check for outdated packages

Usage: vp outdated [OPTIONS] [PACKAGES]... [-- <PASS_THROUGH_ARGS>...]

Arguments:
  [PACKAGES]...           Package name(s) to check
  [PASS_THROUGH_ARGS]...  Additional arguments to pass through to the package manager

Options:
      --long                       Show extended information
      --format <FORMAT>            Output format: table (default), list, or json
  -r, --recursive                  Check recursively across all workspaces
      --filter <PATTERN>           Filter packages in monorepo
  -w, --workspace-root             Include workspace root
  -P, --prod                       Only production and optional dependencies
  -D, --dev                        Only dev dependencies
      --no-optional                Exclude optional dependencies
      --compatible                 Only show compatible versions
      --sort-by <FIELD>            Sort results by field
  -g, --global                     Check globally installed packages
      --concurrency <CONCURRENCY>  Number of global package checks to run in parallel (only with -g)
  -h, --help                       Print help
```

## `vp install`

should install packages first

```

dependencies:
 testnpm2 1.0.0 (1.0.1 is available)

optionalDependencies:
 test-vite-plus-other-optional 1.0.0 (1.1.0 is available)

devDependencies:
 test-vite-plus-top-package 1.0.0 (1.1.0 is available)

Done in <duration> using pnpm <version>
```

## `vp outdated testnpm2`

should outdated package

**Exit code:** 1

```
┌──────────┬─────────┬────────┐
│ Package  │ Current │ Latest │
├──────────┼─────────┼────────┤
│ testnpm2 │ 1.0.0   │ 1.0.1  │
└──────────┴─────────┴────────┘
```

## `vp outdated test-vite*`

should outdated with one glob pattern

**Exit code:** 1

```
┌──────────────────────────────────────────┬─────────┬────────┐
│ Package                                  │ Current │ Latest │
├──────────────────────────────────────────┼─────────┼────────┤
│ test-vite-plus-other-optional (optional) │ 1.0.0   │ 1.1.0  │
├──────────────────────────────────────────┼─────────┼────────┤
│ test-vite-plus-top-package (dev)         │ 1.0.0   │ 1.1.0  │
└──────────────────────────────────────────┴─────────┴────────┘
```

## `vp outdated test-vite* *npm*`

should outdated with multiple glob patterns

**Exit code:** 1

```
┌──────────────────────────────────────────┬─────────┬────────┐
│ Package                                  │ Current │ Latest │
├──────────────────────────────────────────┼─────────┼────────┤
│ testnpm2                                 │ 1.0.0   │ 1.0.1  │
├──────────────────────────────────────────┼─────────┼────────┤
│ test-vite-plus-other-optional (optional) │ 1.0.0   │ 1.1.0  │
├──────────────────────────────────────────┼─────────┼────────┤
│ test-vite-plus-top-package (dev)         │ 1.0.0   │ 1.1.0  │
└──────────────────────────────────────────┴─────────┴────────┘
```

## `vp outdated --format json`

should support json output

**Exit code:** 1

```
{
  "testnpm2": {
    "current": "1.0.0",
    "latest": "1.0.1",
    "wanted": "1.0.0",
    "isDeprecated": false,
    "dependencyType": "dependencies"
  },
  "test-vite-plus-other-optional": {
    "current": "1.0.0",
    "latest": "1.1.0",
    "wanted": "1.0.0",
    "isDeprecated": false,
    "dependencyType": "optionalDependencies"
  },
  "test-vite-plus-top-package": {
    "current": "1.0.0",
    "latest": "1.1.0",
    "wanted": "1.0.0",
    "isDeprecated": false,
    "dependencyType": "devDependencies"
  }
}
```

## `vp outdated --format list`

should support list output

**Exit code:** 1

```
testnpm2
1.0.0 => 1.0.1

test-vite-plus-other-optional (optional)
1.0.0 => 1.1.0

test-vite-plus-top-package (dev)
1.0.0 => 1.1.0
```

## `vp outdated --format table`

should support table output

**Exit code:** 1

```
┌──────────────────────────────────────────┬─────────┬────────┐
│ Package                                  │ Current │ Latest │
├──────────────────────────────────────────┼─────────┼────────┤
│ testnpm2                                 │ 1.0.0   │ 1.0.1  │
├──────────────────────────────────────────┼─────────┼────────┤
│ test-vite-plus-other-optional (optional) │ 1.0.0   │ 1.1.0  │
├──────────────────────────────────────────┼─────────┼────────┤
│ test-vite-plus-top-package (dev)         │ 1.0.0   │ 1.1.0  │
└──────────────────────────────────────────┴─────────┴────────┘
```

## `vp outdated testnpm2 --long --format list`

should support --long

**Exit code:** 1

```
testnpm2
1.0.0 => 1.0.1
```

## `vp outdated -r`

should support recursive output

**Exit code:** 1

```
┌──────────────────────────────────────────┬─────────┬────────┬─────────────────────────┐
│ Package                                  │ Current │ Latest │ Dependents              │
├──────────────────────────────────────────┼─────────┼────────┼─────────────────────────┤
│ testnpm2                                 │ 1.0.0   │ 1.0.1  │ command-outdated-pnpm10 │
├──────────────────────────────────────────┼─────────┼────────┼─────────────────────────┤
│ test-vite-plus-other-optional (optional) │ 1.0.0   │ 1.1.0  │ command-outdated-pnpm10 │
├──────────────────────────────────────────┼─────────┼────────┼─────────────────────────┤
│ test-vite-plus-top-package (dev)         │ 1.0.0   │ 1.1.0  │ command-outdated-pnpm10 │
└──────────────────────────────────────────┴─────────┴────────┴─────────────────────────┘
```

## `vp outdated -P`

should support prod output

**Exit code:** 1

```
┌──────────────────────────────────────────┬─────────┬────────┐
│ Package                                  │ Current │ Latest │
├──────────────────────────────────────────┼─────────┼────────┤
│ testnpm2                                 │ 1.0.0   │ 1.0.1  │
├──────────────────────────────────────────┼─────────┼────────┤
│ test-vite-plus-other-optional (optional) │ 1.0.0   │ 1.1.0  │
└──────────────────────────────────────────┴─────────┴────────┘
```

## `vp outdated -D`

should support dev output

**Exit code:** 1

```
┌──────────────────────────────────┬─────────┬────────┐
│ Package                          │ Current │ Latest │
├──────────────────────────────────┼─────────┼────────┤
│ test-vite-plus-top-package (dev) │ 1.0.0   │ 1.1.0  │
└──────────────────────────────────┴─────────┴────────┘
```

## `vp outdated --no-optional`

should support no-optional output

**Exit code:** 1

```
┌──────────────────────────────────┬─────────┬────────┐
│ Package                          │ Current │ Latest │
├──────────────────────────────────┼─────────┼────────┤
│ testnpm2                         │ 1.0.0   │ 1.0.1  │
├──────────────────────────────────┼─────────┼────────┤
│ test-vite-plus-top-package (dev) │ 1.0.0   │ 1.1.0  │
└──────────────────────────────────┴─────────┴────────┘
```

## `vp outdated --compatible`

should compatible output nothing

```
```

## `vpt json-edit package.json optionalDependencies.test-vite-plus-other-optional '"^1.0.0"'`

should support compatible output with optional dependencies


## `vp outdated --compatible`

**Exit code:** 1

```
┌──────────────────────────────────────────┬─────────┬────────┐
│ Package                                  │ Current │ Latest │
├──────────────────────────────────────────┼─────────┼────────┤
│ test-vite-plus-other-optional (optional) │ 1.0.0   │ 1.1.0  │
└──────────────────────────────────────────┴─────────┴────────┘
```

## `vp outdated --sort-by name`

should support sort-by output

**Exit code:** 1

```
┌──────────────────────────────────────────┬─────────┬────────┐
│ Package                                  │ Current │ Latest │
├──────────────────────────────────────────┼─────────┼────────┤
│ test-vite-plus-other-optional (optional) │ 1.0.0   │ 1.1.0  │
├──────────────────────────────────────────┼─────────┼────────┤
│ test-vite-plus-top-package (dev)         │ 1.0.0   │ 1.1.0  │
├──────────────────────────────────────────┼─────────┼────────┤
│ testnpm2                                 │ 1.0.0   │ 1.0.1  │
└──────────────────────────────────────────┴─────────┴────────┘
```
