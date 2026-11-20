# migrate_existing_pnpm_catalog_order

Migration writes Vite+ into the same catalog selected for dependency references when a named catalog precedes the default.

## `vpt write-file pnpm-workspace.yaml 'packages:
  - apps/*
catalogs:
  legacy:
    vite: '\''npm:@voidzero-dev/vite-plus-core@*'\''
  default:
    vite: '\''npm:@voidzero-dev/vite-plus-core@*'\''
'`


## `vpt json-edit package.json devDependencies.vite catalog:legacy`


## `vp install`


## `vp migrate --no-interactive`


## `vpt print-file package.json`

```
{
  "name": "plain-workspace",
  "private": true,
  "scripts": {
    "prepare": "vp config"
  },
  "devDependencies": {
    "vite": "catalog:legacy",
    "vite-plus": "catalog:legacy"
  },
  "packageManager": "pnpm@11.24.0"
}
```

## `vpt print-file pnpm-workspace.yaml`

```
packages:
  - apps/*
catalogs:
  legacy:
    vite: npm:@voidzero-dev/vite-plus-core@<version>
    vite-plus: <version>
  default:
    vite: npm:@voidzero-dev/vite-plus-core@<version>
overrides:
  vite@*: catalog:legacy
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: "*"
```

## `vp install --frozen-lockfile`

