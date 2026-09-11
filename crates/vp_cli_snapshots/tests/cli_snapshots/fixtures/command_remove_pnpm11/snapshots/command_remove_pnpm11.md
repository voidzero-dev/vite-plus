# command_remove_pnpm11

## `vp remove --help`

should show help

```
VITE+ - The Unified Toolchain for the Web

Usage: vp remove [OPTIONS] <PACKAGES>... [-- <PASS_THROUGH_ARGS>...]

Remove packages from dependencies

Arguments:
  <PACKAGES>...           Packages to remove
  [PASS_THROUGH_ARGS]...  Additional arguments to pass through to the package manager

Options:
  -D, --save-dev        Only remove from `devDependencies` (pnpm-specific)
  -O, --save-optional   Only remove from `optionalDependencies` (pnpm-specific)
  -P, --save-prod       Only remove from `dependencies` (pnpm-specific)
  --filter <PATTERN>    Filter packages in monorepo (can be used multiple times)
  -w, --workspace-root  Remove from workspace root
  -r, --recursive       Remove recursively from all workspace packages
  -g, --global          Remove global packages
  --dry-run             Preview what would be removed without actually removing (only with -g)
  -h, --help            Print help

Documentation: https://viteplus.dev/guide/install
```

## `vp remove`

should error because no packages specified

**Exit code:** 2

```
error: the following required arguments were not provided:
  <PACKAGES>...

Usage: vp remove <PACKAGES>... [-- <PASS_THROUGH_ARGS>...]

For more information, try '--help'.
```

## `vp remove testnpm2 -D`

should error when remove not exists package from dev dependencies

**Exit code:** 1

```
[ERR_PNPM_CANNOT_REMOVE_MISSING_DEPS] Cannot remove 'testnpm2': project has no 'devDependencies'
```

*(skipped 1 step(s) to the next line boundary: step failed)*

## `vp add testnpm2`

should add packages to dependencies

```

dependencies:
 testnpm2 1.0.1

Done in <duration> using pnpm <version>
```

## `vp add -D test-vite-plus-install`

```

devDependencies:
 test-vite-plus-install 1.0.0

Done in <duration> using pnpm <version>
```

## `vp add -O test-vite-plus-package-optional`

```

optionalDependencies:
 test-vite-plus-package-optional 1.0.0

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json`

```
{
  "name": "command-remove-pnpm11",
  "version": "1.0.0",
  "packageManager": "pnpm@11.0.6",
  "dependencies": {
    "testnpm2": "^1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-install": "^1.0.0"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "^1.0.0"
  }
}
```

## `vp remove testnpm2 test-vite-plus-install`

should remove packages from dependencies

```
Packages: -2
--

dependencies:
- testnpm2 1.0.1

devDependencies:
- test-vite-plus-install 1.0.0

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json`

```
{
  "name": "command-remove-pnpm11",
  "version": "1.0.0",
  "packageManager": "pnpm@11.0.6",
  "optionalDependencies": {
    "test-vite-plus-package-optional": "^1.0.0"
  }
}
```

## `vp remove -O test-vite-plus-package-optional -- --loglevel=warn`

support remove package from optional dependencies and pass through arguments

```
```

## `vpt print-file package.json`

```
{
  "name": "command-remove-pnpm11",
  "version": "1.0.0",
  "packageManager": "pnpm@11.0.6"
}
```

## `vp remove -g --dry-run testnpm2`

support remove global package with dry-run

**Exit code:** 1

```
Failed to uninstall testnpm2: Package testnpm2 is not installed
```

*(skipped 1 step(s) to the next line boundary: step failed)*

## `vp rm --stream foo`

should show tips to use pass through arguments when options are not supported

**Exit code:** 2

```
VITE+ - The Unified Toolchain for the Web

error: Unexpected argument '--stream'

Use `-- --stream` to pass the argument as a value
```
