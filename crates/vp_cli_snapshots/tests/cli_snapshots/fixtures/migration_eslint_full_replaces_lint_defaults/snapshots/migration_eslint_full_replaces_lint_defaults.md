# migration_eslint_full_replaces_lint_defaults

## `vp migrate --no-hooks --no-agent --no-editor`

the first run declines the ESLint migration


## `vpt print-file vite.config.ts`

ESLint stays, but the first run still injects a default lint block

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
});
```

## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`

a later full run converts the ESLint rules

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite   → <version>
• 2 config updates applied
• ESLint rules migrated to Oxlint
```

## `vpt print-file vite.config.ts`

the converted rules replace the injected defaults. The default block is not a second copy of the converted config, so dropping that config would lose every rule

```
// vp migrate: merged .oxlintrc.json (a7a54b89d952577ec10ce2ffd3ab980e).
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {},
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
    "eqeqeq": "error",
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

## `vpt stat-file .oxlintrc.json --assert missing`

the converted config is removed only after its merge

```
.oxlintrc.json: missing
```
