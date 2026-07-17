# migration_existing_lint_staged_config

## `git init`


## `vp migrate --no-interactive`

migration should add prepare script, remove lint-staged from devDeps

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 3 config updates applied
• Git hooks configured
```

## `vpt print-file package.json`

check prepare script added, lint-staged removed from devDeps

```
{
  "name": "migration-existing-lint-staged-config",
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

## `vpt stat-file .lintstagedrc.json --assert-not file`

check lintstagedrc.json (should be deleted after inlining to vite.config.ts)

```
.lintstagedrc.json: missing
```

## `vpt print-file vite.config.ts`

check staged config migrated to vite.config.ts

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
  staged: {
    "*.ts": "vp lint --fix"
  },
});
```

## `vpt print-file .vite-hooks/pre-commit`

check pre-commit hook created

```
vp staged
```
