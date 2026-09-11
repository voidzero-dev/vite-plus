# migration_eslint_plugins_cleanup

## `vp migrate --no-interactive`

migration should remove ESLint, plugins, configs, scopes, formatters, and peer eslint

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 4 config updates applied
• ESLint rules migrated to Oxlint
• TypeScript shim added for framework component files
```

## `vpt print-file package.json`

verify the comprehensive ESLint ecosystem cleanup

```
{
  "name": "migration-eslint-plugins-cleanup",
  "scripts": {
    "dev": "vp dev",
    "build": "vp build",
    "lint": "vp lint .",
    "prepare": "vp config"
  },
  "devDependencies": {
    "@nuxt/kit": "^3.13.0",
    "@types/node": "^22.0.0",
    "@typescript-eslint/utils": "^8.0.0",
    "@vitejs/plugin-vue": "^6.0.0",
    "vite": "catalog:",
    "vue": "^3.5.0",
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

## `vpt stat-file eslint.config.mjs --assert-not file`

check eslint config is removed

```
eslint.config.mjs: missing
```

## `vpt print-file vite.config.ts`

verify the generated vite.config.ts

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },
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
});
```
