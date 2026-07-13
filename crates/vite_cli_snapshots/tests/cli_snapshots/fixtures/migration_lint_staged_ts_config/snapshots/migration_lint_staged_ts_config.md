# migration_lint_staged_ts_config

## `git init`


## `vp migrate --no-interactive`

migration should warn about unsupported TS lint-staged config

```
VITE+ - The Unified Toolchain for the Web

⚠ Unsupported lint-staged config format — skipping git hooks setup. Please configure git hooks manually.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
```

## `vpt print-file package.json`

check lint-staged NOT added to package.json, husky/lint-staged removed from devDependencies

```
{
  "name": "migration-lint-staged-ts-config",
  "type": "module",
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

## `vpt print-file lint-staged.config.ts`

check TS config is not modified

```
export default {
  "*.{js,ts}": ["oxlint --fix", "oxfmt"],
};
```
