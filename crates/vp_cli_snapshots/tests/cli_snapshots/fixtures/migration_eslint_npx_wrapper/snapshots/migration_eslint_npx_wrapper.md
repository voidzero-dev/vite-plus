# migration_eslint_npx_wrapper

## `vp migrate --no-interactive`

migration should rewrite bare and bunx eslint but leave other wrappers unchanged

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 4 config updates applied
• ESLint rules migrated to Oxlint
```

## `vpt print-file package.json`

check eslint removed, bare and bunx eslint rewritten, npx/pnpm exec unchanged

```
{
  "name": "migration-eslint-npx-wrapper",
  "scripts": {
    "dev": "vp dev",
    "build": "vp build",
    "lint": "npx eslint .",
    "lint:fix": "pnpm exec eslint --fix .",
    "lint:bunx": "bunx vp lint .",
    "lint:bare": "vp lint --fix .",
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

## `vpt stat-file eslint.config.mjs --assert-not file`

check eslint config is removed

```
eslint.config.mjs: missing
```
