# migration_eslint_rerun

## `vp migrate --full --no-interactive`

should detect vite-plus + eslint and auto-migrate eslint

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• 4 config updates applied
• ESLint rules migrated to Oxlint
• Package manager settings configured
```

## `vpt print-file package.json`

check eslint removed from devDependencies and scripts rewritten

```
{
  "name": "migration-eslint-rerun",
  "scripts": {
    "lint": "vp lint .",
    "prepare": "vp config"
  },
  "devDependencies": {
    "vite-plus": "catalog:"
  }
}
```

## `vpt stat-file eslint.config.mjs --assert-not file`

check eslint config is removed

```
eslint.config.mjs: missing
```

## `vpt print-file vite.config.ts`

check oxlint config merged into vite.config.ts

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },
  lint: {
    "plugins": [
      "oxc",
      "typescript",
      "unicorn"
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
