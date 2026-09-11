# migration_eslint_jsplugins_preserve

## `vp migrate --no-interactive`

plugin referenced via lint.jsPlugins must be preserved through cleanup AND sanitization

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 4 config updates applied
• ESLint rules migrated to Oxlint
```

## `vpt print-file package.json`

eslint-plugin-survives stays in devDependencies (eslint itself is removed)

```
{
  "name": "migration-eslint-jsplugins-preserve",
  "scripts": {
    "lint": "vp lint .",
    "prepare": "vp config"
  },
  "devDependencies": {
    "eslint-plugin-survives": "^1.0.0",
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

## `vpt print-file vite.config.ts`

lint.jsPlugins keeps `eslint-plugin-survives`; lint.rules keeps `survives/no-fiction`

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
    "jsPlugins": [
      "eslint-plugin-survives",
      {
        "name": "vite-plus",
        "specifier": "vite-plus/oxlint-plugin"
      }
    ],
    "categories": {
      "correctness": "warn"
    },
    "env": {
      "builtin": true
    },
    "rules": {
      "survives/no-fiction": "warn",
      "vite-plus/prefer-vite-plus-imports": "error"
    },
    "options": {
      "typeAware": true,
      "typeCheck": true
    }
  },
});
```
