# migration_oxlintrc_json_with_comments

## `vp migrate --no-interactive`

migration should handle .oxlintrc.json with JSONC comments

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 3 config updates applied
```

## `vpt print-file vite.config.ts`

check vite.config.ts

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },
  fmt: {},
  lint: {
    "categories": {
      "correctness": "error"
    },
    "rules": {
      "no-console": "error",
      "vite-plus/prefer-vite-plus-imports": "error"
    },
    "globals": {},
    "ignorePatterns": [],
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

check .oxlintrc.json is removed

```
.oxlintrc.json: missing
```

## `vpt print-file package.json`

check package.json

```
{
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
  },
  "scripts": {
    "prepare": "vp config"
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
