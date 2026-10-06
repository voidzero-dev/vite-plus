# migration_oxlint_unknown_rule_strip

## `vp migrate --no-interactive`

unknown rules in an existing Oxlint config should be stripped with report

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 3 config updates applied
! Warnings:
  - Stripped unsupported Oxlint rule(s) from the generated lint config: camelcase, array-bracket-newline, no-invalid-this. These rule(s) are not available in Oxlint.
```

## `vpt print-file vite.config.ts`

vite.config.ts should omit unsupported `camelcase` and preserve supported `no-console`

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },
  fmt: {},
  lint: {
    "rules": {
      "no-console": "error",
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
