# migration_full_replaces_injected_defaults

## `vp migrate --no-hooks --no-agent --no-editor`

the first run declines both the ESLint and the Prettier migration


## `vpt print-file vite.config.ts`

both tools stay, but the first run still injects default fmt and lint blocks

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
});
```

## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`

a later full run converts both tools

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
● Prettier configuration detected. Auto-migrating to Oxfmt...
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite   → <version>
• 3 config updates applied
• ESLint rules migrated to Oxlint
• Prettier migrated to Oxfmt
```

## `vpt print-file vite.config.ts`

the converted rules and format options replace the injected defaults. The default blocks hold no settings, so treating them as an existing config would lose every converted setting

```
// vp migrate: merged .oxlintrc.json (a7a54b89d952577ec10ce2ffd3ab980e).
import { defineConfig } from "vite-plus";

export default defineConfig({
  fmt: {
    semi: false,
    singleQuote: true,
    printWidth: 120,
    sortPackageJson: false,
    ignorePatterns: [],
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

the converted Oxlint config is removed only after its merge

```
.oxlintrc.json: missing
```
