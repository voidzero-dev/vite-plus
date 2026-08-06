# migration_from_vitest_config

## `vp migrate --no-interactive`

migration should rewrite imports to vite-plus

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied, 1 file had imports rewritten
```

## `vpt print-file vitest.config.ts`

check vitest.config.ts

```
import { join } from 'node:path';

import { foo } from '@foo/vite-plugin-foo';
import { playwright } from 'vite-plus/test/browser-playwright';
import { server } from 'vite-plus/test/browser/context';
import { preview } from 'vite-plus/test/browser-preview';
import { webdriverio } from 'vite-plus/test/browser-webdriverio';
import { userEvent } from 'vite-plus/test/browser/context';
import { defineConfig } from 'vite-plus';

export default defineConfig({
  plugins: [foo()],
  test: {
    dir: join(import.meta.dirname, 'test'),
    browser: {
      enabled: true,
      provider: playwright(),
      headless: true,
      screenshotFailures: false,
      instances: [{ browser: 'chromium' }],
    },
  },
});
```

## `vpt print-file package.json`

check package.json

```
{
  "name": "migration-from-vitest-config",
  "scripts": {
    "test:run": "vp test run",
    "test:ui": "vp test --ui",
    "test:coverage": "vp test run --coverage",
    "test:watch": "vp test --watch",
    "test": "vp test",
    "prepare": "vp config"
  },
  "devDependencies": {
    "@vitest/browser-playwright": "catalog:",
    "@vitest/coverage-v8": "catalog:",
    "vite": "catalog:",
    "vitest": "catalog:",
    "@vitest/browser-webdriverio": "catalog:",
    "webdriverio": "*",
    "playwright": "*",
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

check pnpm-workspace.yaml has overrides and catalog

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vitest: <version>
  vite-plus: <version>
  '@vitest/browser-webdriverio': <version>
  '@vitest/browser-playwright': <version>
  '@vitest/coverage-v8': <version>
allowBuilds:
  edgedriver: true
  geckodriver: true
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
```
