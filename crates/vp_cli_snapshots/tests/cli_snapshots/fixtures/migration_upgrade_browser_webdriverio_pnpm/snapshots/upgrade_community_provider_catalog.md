# upgrade_community_provider_catalog

## `vpt write-file node_modules/vitest/package.json '{"name":"vitest","version":"4.1.11"}'`


## `vpt json-edit package.json devDependencies.@vitest/browser-webdriverio catalog:browser`


## `vpt json-edit package.json devDependencies.webdriverio ^9.20.0`


## `vpt write-file pnpm-workspace.yaml 'catalog:
  vite: npm:@voidzero-dev/vite-plus-core@latest
  vite-plus: latest
catalogs:
  browser:
    '\''@vitest/browser-webdriverio'\'': ^4.1.11
overrides:
  vite: '\''catalog:'\''
  '\''@vitest/browser-webdriverio'\'': 4.1.11
'`


## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• 1 file had imports rewritten
• Package manager settings configured
```

## `vpt print-file package.json`

```
{
  "devDependencies": {
    "@vitest/browser-webdriverio": "catalog:browser",
    "vite-plus": "catalog:",
    "webdriverio": "^9.20.0",
    "vitest": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "onFail": "download",
      "version": "10.33.0"
    }
  },
  "name": "migration-upgrade-browser-webdriverio-pnpm"
}
```

## `vpt print-file pnpm-workspace.yaml`

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
  vitest: <version>
catalogs:
  browser:
    '@vitest/browser-webdriverio': ^5.0.0
overrides:
  vite@*: 'catalog:'
  vitest@*: 'catalog:'
  '@vitest/browser@*': 5.0.1
allowBuilds:
  edgedriver: true
  geckodriver: true
peerDependencyRules:
  allowAny:
    - vite
    - vitest
  allowedVersions:
    vite: '*'
    vitest: '*'
```

## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
