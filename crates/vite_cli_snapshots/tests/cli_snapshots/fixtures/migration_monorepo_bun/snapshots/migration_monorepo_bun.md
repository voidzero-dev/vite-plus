# migration_monorepo_bun

## `vp migrate --no-interactive`

migration should work with bun object-form workspaces

```
VITE+ - The Unified Toolchain for the Web

✔ Merged .oxlintrc.json into vite.config.ts
◇ Migrated . to Vite+ <version>
• Node <version>  bun <version>
• 2 config updates applied, 1 file had imports rewritten
• Inline Vite plugins wrapped with lazyPlugins for check/lint/fmt
```

## `vpt print-file vite.config.ts`

check vite.config.ts

```
import react from '@vitejs/plugin-react';
import { defineConfig, lazyPlugins } from 'vite-plus';

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },
  fmt: {},
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
});
```

## `vpt stat-file .oxlintrc.json --assert-not file`

check .oxlintrc.json is removed

```
.oxlintrc.json: missing
```

## `vpt print-file package.json`

check package.json preserves workspaces object form

```
{
  "name": "migration-monorepo-bun",
  "version": "1.0.0",
  "workspaces": {
    "packages": [
      "packages/*"
    ],
    "catalog": {
      "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
      "vitest": "<version>",
      "vite-plus": "<version>"
    }
  },
  "scripts": {
    "dev": "vp dev",
    "build": "vp build",
    "test": "vp test",
    "lint": "vp lint",
    "fmt": "vp fmt",
    "prepare": "vp config"
  },
  "dependencies": {
    "testnpm2": "1.0.0"
  },
  "devDependencies": {
    "@vitejs/plugin-react": "catalog:",
    "vite": "catalog:",
    "vitest": "catalog:",
    "vite-plus": "catalog:"
  },
  "packageManager": "bun@1.3.11",
  "overrides": {
    "vite": "catalog:",
    "vitest": "catalog:"
  }
}
```

## `vpt print-file packages/app/package.json`

check app package.json

```
{
  "name": "app",
  "scripts": {
    "dev": "vp dev",
    "build": "vp build",
    "test": "vp test"
  },
  "dependencies": {
    "@migration-bun-test/utils": "workspace:*",
    "test-vite-plus-install": "1.0.0",
    "testnpm2": "catalog:"
  },
  "devDependencies": {
    "test-vite-plus-package": "1.0.0",
    "vite": "catalog:",
    "vitest": "catalog:",
    "vite-plus": "catalog:"
  }
}
```

## `vpt print-file packages/utils/package.json`

check utils package.json

```
{
  "name": "@migration-bun-test/utils",
  "scripts": {
    "dev": "vp dev",
    "build": "vp build",
    "test": "vp test"
  },
  "dependencies": {
    "testnpm2": "1.0.0"
  },
  "devDependencies": {
    "vite": "catalog:",
    "vitest": "catalog:",
    "vite-plus": "catalog:"
  }
}
```
