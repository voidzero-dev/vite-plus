# migration_no_hooks_with_husky

## `git init`


## `vp migrate --no-hooks --no-interactive`

--no-hooks should keep husky/lint-staged and preserve config

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
```

## `vpt print-file package.json`

prepare script, lint-staged config, check-staged script, and deps should all be preserved

```
{
  "name": "migration-no-hooks-with-husky",
  "scripts": {
    "prepare": "husky",
    "check-staged": "lint-staged"
  },
  "devDependencies": {
    "husky": "^9.1.7",
    "lint-staged": "^16.2.6",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "lint-staged": {
    "*.ts": "eslint --fix"
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

## `vpt stat-file .husky --assert-not dir`

no .husky directory

```
.husky: missing
```
