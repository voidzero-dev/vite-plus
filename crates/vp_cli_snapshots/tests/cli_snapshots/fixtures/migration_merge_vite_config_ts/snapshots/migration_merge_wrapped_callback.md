# migration_merge_wrapped_callback

Merge settings through parentheses and type wrappers while preserving nested function returns.

## `vpt replace-file-content vite.config.ts 'export default defineConfig({' 'import type { UserConfigFn } from '\''vite'\'';

export default defineConfig(<UserConfigFn>(((() => {
  return {'`


## `vpt replace-file-content vite.config.ts }); '  };
}) as UserConfigFn) satisfies UserConfigFn)'\!');'`


## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 4 config updates applied, 1 file had imports rewritten
• Inline Vite plugins wrapped with lazyPlugins for check/lint/fmt
! Warnings:
  - Vitest v5: 1 review item

vite.config.ts
  9:29 REVIEW [dynamic-config] Review this dynamic config; explicit v4 compatibility settings need manual insertion.
    Docs: https://viteplus.dev/guide/vitest-v5#resolve-migration-findings
```

## `vpt print-file vite.config.ts`

```
import { join } from 'node:path';

import react from '@vitejs/plugin-react';
import { playwright } from 'vite-plus/test/browser-playwright';
import { defineConfig, lazyPlugins } from 'vite-plus';

import type { UserConfigFn } from 'vite-plus';

export default defineConfig(<UserConfigFn>(((() => {
  return {
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
    build: {
      rolldownOptions: {
        output: {
          codeSplitting: {
            groups: [
              {
                name(id) {
                  const c = id.includes('node_modules') ? 'vendor' : null;
                  return c;
                },
              },
            ],
          },
        },
      },
    },
  }
}) as UserConfigFn) satisfies UserConfigFn)!);
```

## `vpt stat-file .oxlintrc.json --assert missing`

```
.oxlintrc.json: missing
```

## `vpt stat-file .oxfmtrc.json --assert missing`

```
.oxfmtrc.json: missing
```
