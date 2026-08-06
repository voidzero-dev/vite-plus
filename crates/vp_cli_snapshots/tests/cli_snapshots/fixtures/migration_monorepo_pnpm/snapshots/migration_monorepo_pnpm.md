# migration_monorepo_pnpm

## `vp migrate --no-interactive`

migration should merge vite.config.ts and remove oxlintrc and oxfmtrc

```
VITE+ - The Unified Toolchain for the Web

✔ Merged .oxlintrc.json into vite.config.ts

✔ Merged .oxfmtrc.json into vite.config.ts
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 4 config updates applied, 1 file had imports rewritten
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
  "name": "migration-monorepo-pnpm",
  "version": "1.0.0",
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
  "resolutions": {
    "vue": "3.5.25"
  },
  "packageManager": "pnpm@10.18.0"
}
```

## `vpt print-file pnpm-workspace.yaml`

check pnpm-workspace.yaml

```
packages:
  - packages/*

catalog:
  testnpm2: ^1.0.0
  # test comment here to check if the comment is preserved
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vitest: <version>
  vite-plus: <version>

minimumReleaseAge: 1440
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
minimumReleaseAgeExclude:
  - vite-plus
  - '@voidzero-dev/*'
  - oxlint
  - '@oxlint/*'
  - oxlint-tsgolint
  - '@oxlint-tsgolint/*'
  - oxfmt
  - '@oxfmt/*'
  - vitest
  - '@vitest/*'
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

## `vpt print-file packages/only-oxlint/package.json`

check only-oxlint package.json

```
{
  "name": "@vite-plus-test/only-oxlint",
  "scripts": {
    "lint": "vp lint --fix"
  },
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  }
}
```

## `vpt print-file packages/only-oxlint/vite.config.ts`

check only-oxlint vite.config.ts

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: {
    "rules": {
      "no-unused-vars": "warn",
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

});
```

## `vpt stat-file packages/only-oxlint/.oxlintrc.json --assert-not file`

check only-oxlint .oxlintrc.json is removed

```
packages/only-oxlint/.oxlintrc.json: missing
```
