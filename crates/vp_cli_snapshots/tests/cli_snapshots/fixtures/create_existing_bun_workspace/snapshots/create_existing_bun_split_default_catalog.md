# create_existing_bun_split_default_catalog

Creating and migrating a member keeps Bun's shared default catalog installable without duplicating entries.

## `vpt json-edit package.json packageManager bun@1.4.4`


## `vpt json-edit package.json catalog.vite npm:@voidzero-dev/vite-plus-core@*`


## `vpt json-edit package.json catalog.vite-plus *`


## `vpt json-edit package.json catalogs.default.react 19.2.0`


## `vpt json-edit package.json devDependencies.react catalog:default`


## `vp install`


## `vp create vite:application --directory apps/web --no-interactive --no-agent --no-editor --no-hooks`


## `vpt print-file package.json`

```
{
  "catalog": {
    "vite": "npm:@voidzero-dev/vite-plus-core@*",
    "vite-plus": "*"
  },
  "catalogs": {
    "default": {
      "react": "19.2.0"
    }
  },
  "devDependencies": {
    "react": "catalog:default"
  },
  "name": "plain-workspace",
  "packageManager": "bun@1.4.4",
  "private": true,
  "workspaces": [
    "apps/*"
  ]
}
```

## `vp install --frozen-lockfile`


## `vp migrate --no-interactive`


## `vpt print-file package.json`

```
{
  "name": "plain-workspace",
  "private": true,
  "workspaces": [
    "apps/*"
  ],
  "scripts": {
    "prepare": "vp config"
  },
  "devDependencies": {
    "react": "catalog:default",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "overrides": {
    "vite": "catalog:"
  },
  "packageManager": "bun@1.4.4",
  "catalog": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vite-plus": "<version>"
  },
  "catalogs": {
    "default": {
      "react": "19.2.0"
    }
  }
}
```

## `vp install --frozen-lockfile`


## `cd apps/web && vp run build`

