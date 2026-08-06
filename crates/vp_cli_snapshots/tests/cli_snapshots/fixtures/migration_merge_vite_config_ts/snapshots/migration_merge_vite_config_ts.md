# migration_merge_vite_config_ts

## `vp migrate --no-interactive`

migration should merge vite.config.ts and remove oxlintrc and oxfmtrc

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 4 config updates applied, 1 file had imports rewritten
• Inline Vite plugins wrapped with lazyPlugins for check/lint/fmt
```

## `vpt print-file vite.config.ts`

check vite.config.ts

```
import { join } from 'node:path';

import react from '@vitejs/plugin-react';
import { playwright } from 'vite-plus/test/browser-playwright';
import { defineConfig, lazyPlugins } from 'vite-plus';

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },
  fmt: {
    "printWidth": 100,
    "tabWidth": 2,
    "semi": true,
    "singleQuote": true,
    "trailingComma": "es5"
  },
  lint: {
    "rules": {
      "no-unused-vars": "error",
      "vite-plus/prefer-vite-plus-imports": "error"
    },
    "options": {
      "typeAware": true,
      "typeCheck": true
    },
    "jsPlugins": [
      {
        "name": "vite-plus",
        "specifier": "vite-plus/oxlint-plugin"
      }
    ]
  },
  plugins: lazyPlugins(() => [react()]),
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

## `vpt stat-file .oxlintrc.json --assert-not file`

check .oxlintrc.json is removed

```
.oxlintrc.json: missing
```

## `vpt stat-file .oxfmtrc.json --assert-not file`

check .oxfmtrc.json is removed

```
.oxfmtrc.json: missing
```

## `vpt print-file package.json`

check package.json

```
{
  "scripts": {
    "dev": "vp dev",
    "build": "vp build",
    "test:run": "vp test run",
    "test:ui": "vp test --ui",
    "test:coverage": "vp test run --coverage",
    "test:watch": "vp test --watch",
    "test": "vp test",
    "lint": "vp lint",
    "lint:fix": "vp lint --fix",
    "lint:type-aware": "vp lint --type-aware",
    "fmt": "vp fmt",
    "fmt:fix": "vp fmt --fix",
    "fmt:staged": "vp fmt --staged",
    "fmt:staged:fix": "vp fmt --staged --fix",
    "prepare": "vp config"
  },
  "devDependencies": {
    "@vitejs/plugin-react": "^4.2.0",
    "@vitest/browser-playwright": "catalog:",
    "vite": "catalog:",
    "vitest": "catalog:",
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
  '@vitest/browser-playwright': <version>
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
