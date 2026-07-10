# migration_existing_pnpm_exec_lint_staged

## `git init`


## `vp migrate --no-interactive`

migration should strip pnpm exec lint-staged and add vp staged

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
• Git hooks configured
```

## `vpt print-file package.json`

check prepare rewritten and husky/lint-staged removed

```
{
  "name": "migration-existing-pnpm-exec-lint-staged",
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

check pnpm exec lint-staged replaced with vp staged

```
vp staged --concurrent false
```
