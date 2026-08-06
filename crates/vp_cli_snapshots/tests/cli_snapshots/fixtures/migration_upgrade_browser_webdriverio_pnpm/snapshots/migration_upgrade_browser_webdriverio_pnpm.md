# migration_upgrade_browser_webdriverio_pnpm

## `vp migrate --no-interactive`

source-only WebdriverIO provider should be restored

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• Package manager settings configured
```

## `vpt print-file package.json`

provider, webdriverio, and local vitest should be present

```
{
  "name": "migration-upgrade-browser-webdriverio-pnpm",
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:",
    "@vitest/browser-webdriverio": "catalog:",
    "webdriverio": "*",
    "vitest": "catalog:"
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

driver builds and shared vitest should be enabled

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
  vitest: <version>
  '@vitest/browser-webdriverio': <version>
overrides:
  vite: 'catalog:'
  vitest: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
    - vitest
  allowedVersions:
    vite: '*'
    vitest: '*'
allowBuilds:
  edgedriver: true
  geckodriver: true
```
