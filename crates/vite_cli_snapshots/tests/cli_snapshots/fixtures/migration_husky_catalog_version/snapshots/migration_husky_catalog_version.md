# migration_husky_catalog_version

## `git init`


## `vp migrate --no-interactive`

should resolve husky version from catalog and configure hooks without warning

```
VITE+ - The Unified Toolchain for the Web

✔ Created vite.config.ts in vite.config.ts

✔ Merged staged config into vite.config.ts
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• Git hooks configured
```

## `vpt print-file package.json`

husky and lint-staged should be removed, prepare rewritten to vp config

```
{
  "name": "migration-husky-catalog-version",
  "scripts": {
    "prepare": "vp config"
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

## `vpt print-file pnpm-workspace.yaml`

check pnpm-workspace.yaml has overrides and catalog

```
packages:
  - .

catalog:
  husky: ^9.1.7
  lint-staged: ^16.2.6
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

## `vpt print-file vite.config.ts`

check staged config migrated to vite.config.ts

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
  staged: {
    "*.js": "vp lint --fix"
  },
});
```

## `vpt print-file .vite-hooks/pre-commit`

check pre-commit hook rewritten

```
vp staged
```
