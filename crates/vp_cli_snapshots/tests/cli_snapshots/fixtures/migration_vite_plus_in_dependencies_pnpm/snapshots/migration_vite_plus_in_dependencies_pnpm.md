# migration_vite_plus_in_dependencies_pnpm

## `vp migrate --no-interactive --no-hooks`

vite-plus declared in dependencies must NOT be duplicated into devDependencies

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  0.1.20 → <version>
    vite              → <version>
• Package manager settings configured
```

## `vpt print-file package.json`

vite-plus stays in dependencies (normalized to catalog:); no duplicate devDependencies entry

```
{
  "name": "migration-vite-plus-in-dependencies-pnpm",
  "dependencies": {
    "vite-plus": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  },
  "devDependencies": {
    "vite": "catalog:"
  }
}
```

## `vpt print-file pnpm-workspace.yaml`

the catalog carries the managed vite-plus version that the dependencies catalog: ref resolves to

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
overrides:
  vite: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```
