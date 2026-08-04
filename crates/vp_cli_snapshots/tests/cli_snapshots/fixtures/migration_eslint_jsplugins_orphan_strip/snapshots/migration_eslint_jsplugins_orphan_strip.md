# migration_eslint_jsplugins_orphan_strip

## `vp migrate --no-interactive`

orphan jsPlugin / unknown plugin / dangling rule should be stripped, with warnings

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 4 config updates applied
• ESLint rules migrated to Oxlint
! Warnings:
  - Stripped JS plugin reference(s) from the generated lint config: eslint-plugin-fictional, eslint-plugin-override-only. No matching package is present in this workspace, so loading them at lint time would fail. If you want their Oxlint coverage back, install each package (e.g. `vp install <name>`) and add its name back to `lint.jsPlugins` in vite.config.ts.
```

## `vpt print-file vite.config.ts`

lint block should NOT contain `fictional` in plugins / jsPlugins / rules

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
      "vite-plus/prefer-vite-plus-imports": "error"
    },
    "overrides": [
      {
        "files": [
          "**/*.test.js"
        ],
        "rules": {},
        "jsPlugins": []
      }
    ],
    "options": {
      "typeAware": true,
      "typeCheck": true
    }
  },
});
```
