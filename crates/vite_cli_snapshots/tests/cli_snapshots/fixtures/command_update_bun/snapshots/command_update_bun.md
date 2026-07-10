# command_update_bun

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
bun update <version> (af24e281)

 test-vite-plus-package@1.0.0
 test-vite-plus-package-optional@1.0.0

installed testnpm2@1.0.1

3 packages installed [<duration>]
```

## `vpt print-file package.json`

```
{
  "name": "command-update-bun",
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
  "packageManager": "bun@1.3.11"
}
```

## `vp up testnpm2 --latest`

should update to absolute latest version

```
bun update <version> (af24e281)

installed testnpm2@1.0.1

[<duration>] done
```

## `vpt print-file package.json`

```
{
  "name": "command-update-bun",
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
  "packageManager": "bun@1.3.11"
}
```
