# migration_upgrade_stale_local_pnpm

## `node setup-local.mjs`


## `vp migrate --no-interactive`

newer global CLI must bypass the installed stale local CLI

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  0.1.24 → <version>
    vite              → <version>
• Package manager settings configured
```

## `vpt print-file package.json`

stale wrapper deps and plain vite-plus range should be repaired; empty pnpm field should be removed

```
{
  "name": "migration-upgrade-stale-local-pnpm",
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

pnpm settings should be consolidated here

```
overrides:
  vite: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
```
