# migration_eslint_svelte_runes

## `vp migrate --no-interactive`

migration should add Svelte rune globals to the lint override

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 4 config updates applied
• ESLint rules migrated to Oxlint
```

## `vpt print-file vite.config.ts`

Svelte override includes every built-in rune as a readonly global

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
      "no-undef": "error",
      "vite-plus/prefer-vite-plus-imports": "error"
    },
    "overrides": [
      {
        "files": [
          "*.svelte",
          "**/*.svelte"
        ],
        "rules": {
          "no-inner-declarations": "off",
          "no-self-assign": "off"
        },
        "globals": {
          "$state": "readonly",
          "$derived": "readonly",
          "$effect": "readonly",
          "$props": "readonly",
          "$bindable": "readonly",
          "$inspect": "readonly",
          "$host": "readonly"
        }
      }
    ],
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

## `vp lint src/App.svelte`

valid Svelte rune usage should pass no-undef

```
VITE+ - The Unified Toolchain for the Web

Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
