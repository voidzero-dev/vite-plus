# migration_other_hook_tool

## `vp migrate --no-interactive`

hooks should be skipped due to simple-git-hooks

```
VITE+ - The Unified Toolchain for the Web

⚠ Detected simple-git-hooks — skipping git hooks setup. Please configure git hooks manually, see https://viteplus.dev/guide/migrate#git-hook-tools
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
```

## `vpt print-file package.json`

lint-staged config, scripts, and simple-git-hooks config should all be preserved

```
{
  "name": "migration-other-hook-tool-with-lint-staged",
  "scripts": {
    "check-staged": "lint-staged"
  },
  "devDependencies": {
    "lint-staged": "^16.2.6",
    "simple-git-hooks": "^2.11.1",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "simple-git-hooks": {
    "pre-commit": "npx lint-staged"
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
