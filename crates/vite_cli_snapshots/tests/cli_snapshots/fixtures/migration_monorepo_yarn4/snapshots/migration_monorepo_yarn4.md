# migration_monorepo_yarn4

## `vp migrate --no-interactive`

migration should merge vite.config.ts and remove oxlintrc

```
VITE+ - The Unified Toolchain for the Web

⚠ Vite+ does not currently support Yarn Plug'n'Play (PnP).

✔ Switched Yarn to node-modules mode

✔ Merged .oxlintrc.json into vite.config.ts
◇ Migrated . to Vite+ <version>
• Node <version>  yarn <version>
• 2 config updates applied, 1 file had imports rewritten
• Inline Vite plugins wrapped with lazyPlugins for check/lint/fmt
• Package manager settings configured
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

check package.json

```
{
  "name": "migration-monorepo-yarn4",
  "version": "1.0.0",
  "workspaces": [
    "packages/*"
  ],
  "scripts": {
    "dev": "vp dev",
    "build": "vp build",
    "test:run": "vp test run",
    "test:ui": "vp test --ui",
    "test:coverage": "vp test run --coverage",
    "test:watch": "vp test --watch",
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
  "packageManager": "yarn@4.12.0",
  "resolutions": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vitest": "<version>"
  }
}
```

## `vpt print-file .yarnrc.yml`

check .yarnrc.yml

```
nodeLinker: node-modules
npmPreapprovedPackages:
  - vitest
  - '@vitest/*'
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vitest: <version>
  vite-plus: <version>
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
    "@vite-plus-test/utils": "workspace:*",
    "test-vite-plus-install": "1.0.0",
    "testnpm2": "catalog:"
  },
  "devDependencies": {
    "test-vite-plus-package": "1.0.0",
    "vite": "catalog:",
    "vitest": "catalog:",
    "vite-plus": "catalog:"
  },
  "optionalDependencies": {
    "test-vite-plus-other-optional": "1.0.0"
  }
}
```

## `vpt print-file packages/utils/package.json`

check utils package.json

```
{
  "name": "@vite-plus-test/utils",
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
