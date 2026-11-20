# migration_upgrade_browser_peer_only_pnpm

## `vp migrate --no-interactive`

peer-only browser provider is promoted with its required peers

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• Package manager settings configured
• Removed `vite` from pnpm peerDependencyRules: the core version now matches the bundled Vite
```

## `vpt print-file package.json`

provider, Playwright, and package-local Vitest are installed

```
{
  "name": "migration-upgrade-browser-peer-only-pnpm",
  "devDependencies": {
    "vite-plus": "catalog:",
    "@vitest/browser-playwright": "catalog:",
    "playwright": "*",
    "vitest": "catalog:"
  },
  "peerDependencies": {
    "@vitest/browser-playwright": "^4.0.0"
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

promoted provider keeps shared Vitest management

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
  vitest: <version>
  '@vitest/browser-playwright': <version>
overrides:
  vite@*: 'catalog:'
  vitest@*: 'catalog:'
peerDependencyRules:
  allowAny:
    - vitest
  allowedVersions:
    vitest: '*'
```

## `vp migrate --no-interactive`

repaired project should no longer be pending

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
