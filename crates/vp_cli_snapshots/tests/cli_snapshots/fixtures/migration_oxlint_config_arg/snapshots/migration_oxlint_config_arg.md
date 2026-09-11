# migration_oxlint_config_arg

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

Migrate a project with an Oxlint config argument

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
```

## `vpt print-file package.json`

oxlint -c .oxlintrc.json should be rewritten to vp lint

```
{
  "scripts": {
    "lint": "vp lint"
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

## `vpt print-file vite.config.ts`

.oxlintrc.json should be merged into vite.config.ts

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
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

## `vpt stat-file .oxlintrc.json --assert-not file`

The merged .oxlintrc.json should be removed

```
.oxlintrc.json: missing
```
