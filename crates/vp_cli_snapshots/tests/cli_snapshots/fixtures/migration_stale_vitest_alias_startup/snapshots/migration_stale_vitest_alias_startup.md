# migration_stale_vitest_alias_startup

## `node setup-local.mjs`


## `npm_lifecycle_event=prepare vp config --no-hooks --no-agent`

prepare should report the stale alias instead of loading its native binding

**Exit code:** 1

```
error: Found a stale Vitest alias in pnpm-workspace.yaml that points to the removed `@voidzero-dev/vite-plus-test` package. Run `vp migrate` to update the project before installing dependencies.
```

## `VP_SKIP_INSTALL=1 vp migrate --no-interactive`

migrate should start without loading the stale Vitest alias

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite   → <version>
• Package manager settings configured
```

## `vpt print-file package.json`

the stale Vitest dependency should be removed

```
{
  "name": "migration-stale-vitest-alias-startup",
  "scripts": {
    "prepare": "vp config"
  },
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vpt print-file pnpm-workspace.yaml`

the stale catalog alias and override should be removed

```
packages:
  - .

catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>

overrides:
  vite@*: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```
