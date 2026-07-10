# migration_merge_vite_config_js

## `vp migrate --no-interactive`

migration should merge vite.config.js and remove oxlintrc

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 3 config updates applied
• Inline Vite plugins wrapped with lazyPlugins for check/lint/fmt
```

## `vpt print-file vite.config.js`

check vite.config.js

```
import react from '@vitejs/plugin-react';
import { lazyPlugins } from 'vite-plus';

export default {
  staged: {
    "*": "vp check --fix"
  },
  fmt: {},
  lint: {
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
  plugins: lazyPlugins(() => [react()]),
}
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
  "scripts": {
    "dev": "vp dev --port 3000",
    "build": "vp build",
    "lint": "vp lint",
    "prepare": "vp config"
  },
  "devDependencies": {
    "@vitejs/plugin-react": "^4.2.0",
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
