# command_add_pnpm12

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
  --ignore-scripts                    Do not run lifecycle scripts
  --no-optional                       Do not install optionalDependencies
  --frozen-lockfile                   Fail if lockfile needs to be updated
  --no-frozen-lockfile                Allow lockfile updates
  --lockfile-only                     Only update lockfile, don't install
  --prefer-offline                    Use cached packages when available
  --offline                           Only use packages already in cache
  -f, --force                         Force reinstall all dependencies
  --no-lockfile                       Don't read or generate lockfile
  --shamefully-hoist                  Create flat node_modules (pnpm only)
  --silent                            Suppress Vite+ output and enable native silent mode where supported
  --filter <PATTERN>                  Filter packages in monorepo (can be used multiple times)
  -w, --workspace-root                Add to workspace root
  --workspace                         Only add if package exists in workspace (pnpm-specific)
  -g, --global                        Install globally
  --node <NODE>                       Node.js version to use for global installation (only with -g)
  --concurrency <CONCURRENCY>         Number of global package installs to run in parallel (only with -g)
  --run-scripts                       Run all lifecycle scripts (only with -g)
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

## `vp add testnpm2 -D -- --loglevel=verbose --verbose`

should add package as dev dependencies

**Exit code:** 2

```
error: unexpected argument '--loglevel' found

  tip: to pass '--loglevel' as a value, use '-- --loglevel'

Usage: pnpm add --save-dev <PACKAGE_NAMES>...

For more information, try '--help'.
```

*(skipped 1 step(s) to the next line boundary: step failed)*

## `vp add testnpm2 test-vite-plus-install --allow-build=test-vite-plus-install`

should add packages to dependencies

```

dependencies:
 test-vite-plus-install 1.0.0
 testnpm2 1.0.1

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json`

```
{
  "name": "command-add-pnpm12",
  "version": "1.0.0",
  "packageManager": "pnpm@12.0.0-beta.0",
  "dependencies": {
    "test-vite-plus-install": "^1.0.0",
    "testnpm2": "^1.0.1"
  }
}
```

## `vp install test-vite-plus-package@1.0.0 --save-peer`

should install package alias for add

```
VITE+ - The Unified Toolchain for the Web

✓ Lockfile passes supply-chain policies (verified <duration> ago)

devDependencies:
 test-vite-plus-package 1.0.0

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json`

```
{
  "name": "command-add-pnpm12",
  "version": "1.0.0",
  "packageManager": "pnpm@12.0.0-beta.0",
  "dependencies": {
    "test-vite-plus-install": "^1.0.0",
    "testnpm2": "^1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-package": "1.0.0"
  },
  "peerDependencies": {
    "test-vite-plus-package": "1.0.0"
  }
}
```

## `vp add test-vite-plus-package-optional -O`

should add package as optional dependencies

```
✓ Lockfile passes supply-chain policies (verified <duration> ago)

optionalDependencies:
 test-vite-plus-package-optional 1.0.0

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json`

```
{
  "name": "command-add-pnpm12",
  "version": "1.0.0",
  "packageManager": "pnpm@12.0.0-beta.0",
  "dependencies": {
    "test-vite-plus-install": "^1.0.0",
    "testnpm2": "^1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-package": "1.0.0"
  },
  "peerDependencies": {
    "test-vite-plus-package": "1.0.0"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "^1.0.0"
  }
}
```

## `vp add test-vite-plus-package-optional -- --loglevel=warn`

support pass through arguments

**Exit code:** 2

```
error: unexpected argument '--loglevel' found

  tip: to pass '--loglevel' as a value, use '-- --loglevel'

Usage: pnpm add [OPTIONS] <PACKAGE_NAMES>...

For more information, try '--help'.
```

*(skipped 1 step(s) to the next line boundary: step failed)*
