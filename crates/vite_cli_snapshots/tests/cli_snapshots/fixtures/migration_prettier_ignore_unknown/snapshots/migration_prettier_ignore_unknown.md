# migration_prettier_ignore_unknown

## `vp migrate --no-interactive`

migration should strip --ignore-unknown and -u flags

```
VITE+ - The Unified Toolchain for the Web

Prettier configuration detected. Auto-migrating to Oxfmt...
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
• Prettier migrated to Oxfmt
```

## `vpt print-file package.json`

check prettier removed and --ignore-unknown stripped from scripts

```
{
  "name": "migration-prettier-ignore-unknown",
  "scripts": {
    "dev": "vp dev",
    "build": "vp build",
    "format": "vp fmt .",
    "format:check": "vp fmt --check .",
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

## `vpt stat-file .prettierrc.json --assert-not file`

check prettier config is removed

```
.prettierrc.json: missing
```
