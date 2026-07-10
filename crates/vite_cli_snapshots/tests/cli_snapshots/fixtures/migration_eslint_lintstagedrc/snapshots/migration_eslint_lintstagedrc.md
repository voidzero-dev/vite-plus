# migration_eslint_lintstagedrc

## `vp migrate --no-interactive`

migration should detect eslint and auto-migrate including lintstagedrc

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 5 config updates applied
• ESLint rules migrated to Oxlint
```

## `vpt print-file package.json`

check eslint removed and scripts rewritten

```
{
  "name": "migration-eslint-lintstagedrc",
  "scripts": {
    "lint": "vp lint .",
    "prepare": "vp config"
  },
  "devDependencies": {
    "vite": "catalog:",
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
  vite-plus: <version>
overrides:
  vite: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```

## `vpt stat-file .lintstagedrc.json --assert-not file`

check lintstagedrc.json is removed

```
.lintstagedrc.json: missing
```

## `vpt print-file vite.config.ts`

check oxlint config merged into vite.config.ts

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {},
  lint: {
    "plugins": [
      "oxc",
      "typescript",
      "unicorn",
      "react"
    ],
    "categories": {
      "correctness": "warn"
    },
    "env": {
      "builtin": true
    },
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
  staged: {
    "*.ts": "vp lint --fix"
  },
});
```
