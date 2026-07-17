# migration_subpath

## `git init`


## `vp migrate foo --no-interactive`

migration work with subpath

```
VITE+ - The Unified Toolchain for the Web

⚠ Subdirectory project detected — skipping git hooks setup. Configure hooks at the repository root.
◇ Migrated foo to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
```

## `vpt print-file foo/package.json`

check package.json

```
{
  "name": "migration-subpath",
  "lint-staged": {
    "*.@(js|ts|tsx|yml|yaml|md|json|html|toml)": [
      "oxfmt --staged",
      "eslint --fix"
    ],
    "*.@(js|ts|tsx)": [
      "oxlint --fix"
    ]
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

## `vpt print-file foo/vite.config.ts`

check vite.config.ts

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
});
```

## `vpt stat-file .vite-hooks --assert-not dir`

root git hooks are NOT set up for a subdirectory migration

```
.vite-hooks: missing
```

## `vpt print-file foo/pnpm-workspace.yaml`

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
