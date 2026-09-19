# migration_upgrade_browser_webdriverio_pnpm

## `vpt write-file node_modules/vitest/package.json '{"name":"vitest","version":"4.1.11"}'`

record the original runner version without adding a direct dependency


## `vp migrate --no-interactive`

restore the community import and request a user-selected provider version

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• 1 file had imports rewritten
• Package manager settings configured
! Warnings:
  - Vitest v5: 1 review item

package.json
  1:1 REVIEW [browser-provider] Add @vitest/browser-webdriverio and its required peers using versions compatible with your tests. Vite+ no longer exports or manages this community provider.
    Docs: https://viteplus.dev/guide/vitest-v5#community-webdriverio-provider
```

## `vpt print-file package.json`

do not inject a community provider or framework version

```
{
  "name": "migration-upgrade-browser-webdriverio-pnpm",
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:",
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

## `vpt print-file vite.config.ts`

legacy Vite+ provider import points to the community package

```
import { defineConfig } from 'vite-plus';
import { webdriverio } from '@vitest/browser-webdriverio';

export default defineConfig({
  test: {
    browser: {
      enabled: true,
      provider: webdriverio(),
    },
  },
});
```

## `vpt print-file pnpm-workspace.yaml`

driver builds and shared vitest should be enabled

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
  vitest: <version>
overrides:
  vite@*: 'catalog:'
  vitest@*: 'catalog:'
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
