# migration_no_git_repo

## `vp migrate --no-interactive`

migration should create .vite-hooks/pre-commit even without .git

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
```

## `vpt print-file package.json`

check package.json has prepare script and lint-staged config

```
{
  "name": "migration-no-git-repo",
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

## `vpt stat-file .vite-hooks --assert dir`

hooks dir exists even without .git

```
.vite-hooks: dir
```

## `vpt print-file .vite-hooks/pre-commit`

pre-commit hook should exist even without .git

```
vp staged
```
