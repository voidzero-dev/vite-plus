# migration_existing_prepare_script

## `git init`


## `vp migrate --no-interactive`

migration should compose vp config with existing prepare script

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
• Git hooks configured
```

## `vpt print-file package.json`

check prepare script is composed: vp config && npm run build

```
{
  "name": "migration-existing-prepare-script",
  "scripts": {
    "build": "tsc",
    "prepare": "vp config && npm run build"
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

## `vpt print-file .vite-hooks/pre-commit`

check pre-commit hook

```
vp staged
```
