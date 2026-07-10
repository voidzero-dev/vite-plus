# migration_upgrade_vitest_exact_peer_yarn4

## `vp migrate --no-interactive`

Yarn PnP converts to node-modules before exact-peer migration

```
VITE+ - The Unified Toolchain for the Web

⚠ Vite+ does not currently support Yarn Plug'n'Play (PnP).

✔ Switched Yarn to node-modules mode
◇ Updated . to Vite+ <version>
• Node <version>  yarn <version>
• Dependencies:
    vite-plus   latest → <version>
    vite               → <version>
    @vitest/ui  4.1.8  → <version>
• Package manager settings configured
```

## `vpt print-file package.json`

direct deps and resolutions should use the managed catalog/version

```
{
  "name": "migration-upgrade-vitest-exact-peer-yarn4",
  "devDependencies": {
    "@vitest/ui": "catalog:",
    "vite-plus": "catalog:",
    "vitest": "catalog:"
  },
  "resolutions": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vitest": "<version>"
  },
  "devEngines": {
    "packageManager": {
      "name": "yarn",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vpt print-file .yarnrc.yml`

linker conversion and aligned Vitest catalog are persisted

```
nodeLinker: node-modules
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
  vitest: <version>
  '@vitest/ui': <version>
npmPreapprovedPackages:
  - vitest
  - '@vitest/*'
```
