# command_remove_bun

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

```
bun remove <version> (af24e281)
package.json doesn't have dependencies, there's nothing to remove!
```

## `vpt print-file package.json`

```
{
  "name": "command-remove-bun",
  "version": "1.0.0",
  "packageManager": "bun@1.3.11"
}
```

## `vp add testnpm2`

should add packages to dependencies

```
bun add <version> (af24e281)

installed testnpm2@1.0.1

1 package installed [<duration>]
```

## `vp add -D test-vite-plus-install`

```
bun add <version> (af24e281)

installed test-vite-plus-install@1.0.0

1 package installed [<duration>]
```

## `vp add -O test-vite-plus-package-optional`

```
bun add <version> (af24e281)

installed test-vite-plus-package-optional@1.0.0

1 package installed [<duration>]
```

## `vpt print-file package.json`

```
{
  "name": "command-remove-bun",
  "version": "1.0.0",
  "packageManager": "bun@1.3.11",
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
bun remove <version> (af24e281)

- testnpm2
- test-vite-plus-install
2 packages removed [<duration>]
```

## `vpt print-file package.json`

```
{
  "name": "command-remove-bun",
  "version": "1.0.0",
  "optionalDependencies": {
    "test-vite-plus-package-optional": "^1.0.0"
  },
  "packageManager": "bun@1.3.11"
}
```

## `vp remove -O test-vite-plus-package-optional`

should remove package from optional dependencies

```
bun remove <version> (af24e281)

package.json has no dependencies! Deleted empty lockfile

- test-vite-plus-package-optional
1 package removed [<duration>]
```

## `vpt print-file package.json`

```
{
  "name": "command-remove-bun",
  "version": "1.0.0",
  "packageManager": "bun@1.3.11"
}
```
