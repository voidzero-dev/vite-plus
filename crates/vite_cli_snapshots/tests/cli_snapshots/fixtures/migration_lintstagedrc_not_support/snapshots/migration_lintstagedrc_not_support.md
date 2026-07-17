# migration_lintstagedrc_not_support

## `git init`


## `vp migrate --no-interactive`

migration should not support non-json format lintstagedrc

```
VITE+ - The Unified Toolchain for the Web

⚠ Unsupported lint-staged config format — skipping git hooks setup. Please configure git hooks manually.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
```

## `vpt print-file .lintstagedrc`

check .lintstagedrc is not updated

```
'*.js':
  - oxlint
  - oxfmt
```

## `vpt print-file .lintstagedrc.yaml`

check .lintstagedrc.yaml is not updated

```
'*.js':
  - oxlint
  - oxfmt
```

## `vpt print-file lint-staged.config.mjs`

check lint-staged.config.mjs is not updated

```
export default {
  '*.js': ['oxlint', 'oxfmt'],
};
```

## `vpt print-file package.json`

check hooks setup skipped but husky/lint-staged removed from devDependencies

```
{
  "name": "migration-lintstagedrc-not-support",
  "scripts": {
    "prepare": "husky"
  },
  "devDependencies": {
    "husky": "^9.1.7",
    "lint-staged": "^16.2.6",
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
