# migration_hooks_skip_on_existing_hookspath

## `git init`


## `git config core.hooksPath .custom-hooks`


## `vpt mkdir -p .custom-hooks`


## `vpt write-file .custom-hooks/pre-commit 'npx lint-staged
'`


## `vpt write-file .lintstagedrc.json '{"*.ts":"eslint --fix"}
'`


## `vp migrate --no-interactive`

should skip hooks because core.hooksPath is already set

```
VITE+ - The Unified Toolchain for the Web

⚠ core.hooksPath is already set to ".custom-hooks", skipping git hooks setup.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
```

## `vpt print-file package.json`

keep the existing hook dependencies

```
{
  "name": "migration-hooks-skip-on-existing-hookspath",
  "scripts": {
    "prepare": "husky"
  },
  "devDependencies": {
    "husky": "^9.1.7",
    "lint-staged": "^16.2.7",
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

## `vpt print-file .lintstagedrc.json`

keep the active hook configuration

```
{"*.ts":"eslint --fix"}
```

## `vpt print-file .custom-hooks/pre-commit`

keep the active custom hook

```
npx lint-staged
```

## `vpt print-file vite.config.ts`

do not migrate staged configuration

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
});
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

## `git config --local core.hooksPath`

should still be .custom-hooks

```
.custom-hooks
```
