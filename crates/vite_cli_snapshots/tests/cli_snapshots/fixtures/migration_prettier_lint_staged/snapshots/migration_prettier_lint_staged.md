# migration_prettier_lint_staged

## `vp migrate --no-interactive`

migration should detect prettier and auto-migrate including lint-staged

```
VITE+ - The Unified Toolchain for the Web

Prettier configuration detected. Auto-migrating to Oxfmt...
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
• Prettier migrated to Oxfmt
```

## `vpt print-file package.json`

check prettier removed, scripts rewritten, lint-staged rewritten

```
{
  "name": "migration-prettier-lint-staged",
  "scripts": {
    "format": "vp fmt .",
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

check oxfmt config and staged config merged into vite.config.ts

```
import { defineConfig } from "vite-plus";

export default defineConfig({
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
  staged: {
    "*.ts": "vp fmt"
  },
  fmt: {
    semi: true,
    printWidth: 80,
    sortPackageJson: false,
    ignorePatterns: [],
  },
});
```
