# command_add_bun

## `vp add --help`

should show help

```
VITE+ - The Unified Toolchain for the Web

Usage: vp add [OPTIONS] <PACKAGES>... [-- <PASS_THROUGH_ARGS>...]

Add packages to dependencies

Arguments:
  <PACKAGES>...           Packages to add
  [PASS_THROUGH_ARGS]...  Additional arguments to pass through to the package manager

Options:
  -P, --save-prod                     Save to `dependencies` (default)
  -D, --save-dev                      Save to `devDependencies`
  --save-peer                         Save to `peerDependencies` and `devDependencies`
  -O, --save-optional                 Save to `optionalDependencies`
  -E, --save-exact                    Save exact version rather than semver range
  --save-catalog-name <CATALOG_NAME>  Save the new dependency to the specified catalog name
  --save-catalog                      Save the new dependency to the default catalog
  --allow-build <NAMES>               A list of package names allowed to run postinstall
  --filter <PATTERN>                  Filter packages in monorepo (can be used multiple times)
  -w, --workspace-root                Add to workspace root
  --workspace                         Only add if package exists in workspace (pnpm-specific)
  -g, --global                        Install globally
  --node <NODE>                       Node.js version to use for global installation (only with -g)
  --concurrency <CONCURRENCY>         Number of global package installs to run in parallel (only with -g)
  -h, --help                          Print help

Documentation: https://viteplus.dev/guide/install
```

## `vp add`

should error because no packages specified

**Exit code:** 2

```
error: the following required arguments were not provided:
  <PACKAGES>...

Usage: vp add <PACKAGES>... [-- <PASS_THROUGH_ARGS>...]

For more information, try '--help'.
```

## `vp add testnpm2 -D`

should add package as dev dependencies

```
bun add <version> (af24e281)

installed testnpm2@1.0.1

1 package installed [<duration>]
```

## `vpt print-file package.json`

```
{
  "name": "command-add-bun",
  "version": "1.0.0",
  "packageManager": "bun@1.3.11",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  }
}
```

## `vp add testnpm2 test-vite-plus-install`

should add packages to dependencies

```
bun add <version> (af24e281)

installed testnpm2@1.0.1
installed test-vite-plus-install@1.0.0

2 packages installed [<duration>]
```

## `vpt print-file package.json`

```
{
  "name": "command-add-bun",
  "version": "1.0.0",
  "packageManager": "bun@1.3.11",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  },
  "dependencies": {
    "test-vite-plus-install": "^1.0.0"
  }
}
```

## `vp install test-vite-plus-package@1.0.0 --save-peer`

should install package alias for add

```
VITE+ - The Unified Toolchain for the Web

bun add <version> (af24e281)

installed test-vite-plus-package@1.0.0

1 package installed [<duration>]
```

## `vpt print-file package.json`

```
{
  "name": "command-add-bun",
  "version": "1.0.0",
  "packageManager": "bun@1.3.11",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  },
  "dependencies": {
    "test-vite-plus-install": "^1.0.0"
  },
  "peerDependencies": {
    "test-vite-plus-package": "1.0.0"
  }
}
```

## `vp add test-vite-plus-package-optional -O`

should add package as optional dependencies

```
bun add <version> (af24e281)

installed test-vite-plus-package-optional@1.0.0

1 package installed [<duration>]
```

## `vpt print-file package.json`

```
{
  "name": "command-add-bun",
  "version": "1.0.0",
  "packageManager": "bun@1.3.11",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  },
  "dependencies": {
    "test-vite-plus-install": "^1.0.0"
  },
  "peerDependencies": {
    "test-vite-plus-package": "1.0.0"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "^1.0.0"
  }
}
```
