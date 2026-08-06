# command_update_pnpm11

## `vp update --help`

should show help

```
VITE+ - The Unified Toolchain for the Web

Usage: vp update [OPTIONS] [PACKAGES]... [-- <PASS_THROUGH_ARGS>...]

Update packages to their latest versions

Arguments:
  [PACKAGES]...           Packages to update (optional - updates all if omitted)
  [PASS_THROUGH_ARGS]...  Additional arguments to pass through to the package manager

Options:
  -L, --latest                 Update to latest version (ignore semver range)
  -g, --global                 Update global packages
  --concurrency <CONCURRENCY>  Number of global package updates to run in parallel (only with -g)
  --reinstall-node-mismatch    Reinstall up-to-date global packages installed with a different Node.js version
  --ignore-node-mismatch       Skip up-to-date global packages installed with a different Node.js version
  -r, --recursive              Update recursively in all workspace packages
  --filter <PATTERN>           Filter packages in monorepo (can be used multiple times)
  -w, --workspace-root         Include workspace root
  -D, --dev                    Update only devDependencies
  -P, --prod                   Update only dependencies (production)
  -i, --interactive            Interactive mode
  --no-optional                Don't update optionalDependencies
  --no-save                    Update lockfile only, don't modify package.json
  --workspace                  Only update if package exists in workspace (pnpm-specific)
  -h, --help                   Print help

Documentation: https://viteplus.dev/guide/install
```

## `vp update testnpm2`

should update package within semver range

```

dependencies:
 testnpm2 1.0.1

optionalDependencies:
 test-vite-plus-package-optional 1.0.0

devDependencies:
 test-vite-plus-package 1.0.0

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json`

```
{
  "name": "command-update-pnpm11",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "^1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-package": "*"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "*"
  },
  "packageManager": "pnpm@11.0.6"
}
```

## `vp up testnpm2 --latest`

should to absolute latest version

```
Already up to date

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json`

```
{
  "name": "command-update-pnpm11",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "^1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-package": "*"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "*"
  },
  "packageManager": "pnpm@11.0.6"
}
```

## `vp update -D`

should update only dev dependencies

```
Already up to date

dependencies: skipped

optionalDependencies: skipped

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json`

```
{
  "name": "command-update-pnpm11",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "^1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-package": "^1.0.0"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "*"
  },
  "packageManager": "pnpm@11.0.6"
}
```

## `vp update -P --no-save`

should update only dependencies and optionalDependencies without saving

```
Already up to date

devDependencies: skipped

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json`

```
{
  "name": "command-update-pnpm11",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "^1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-package": "^1.0.0"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "*"
  },
  "packageManager": "pnpm@11.0.6"
}
```

## `vp rm testnpm2`

should remove package from dependencies for the next test


## `vp add testnpm2@1.0.0 -O`

should skip optional dependencies

```

optionalDependencies:
 testnpm2 1.0.0 (1.0.1 is available)

Done in <duration> using pnpm <version>
```

## `vp update --no-optional --latest`

```
 -1
-

optionalDependencies:
- testnpm2 1.0.0
 testnpm2 1.0.1

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json`

```
{
  "name": "command-update-pnpm11",
  "version": "1.0.0",
  "devDependencies": {
    "test-vite-plus-package": "^1.0.0"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "^1.0.0",
    "testnpm2": "1.0.1"
  },
  "packageManager": "pnpm@11.0.6"
}
```

## `vp update`

should update all packages and change the package.json

```
Already up to date

Done in <duration> using pnpm <version>
```

## `vp update --recursive`

```

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json`

```
{
  "name": "command-update-pnpm11",
  "version": "1.0.0",
  "devDependencies": {
    "test-vite-plus-package": "^1.0.0"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "^1.0.0",
    "testnpm2": "1.0.1"
  },
  "packageManager": "pnpm@11.0.6"
}
```
