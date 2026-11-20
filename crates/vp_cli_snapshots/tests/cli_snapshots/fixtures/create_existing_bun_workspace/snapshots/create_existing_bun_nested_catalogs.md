# create_existing_bun_nested_catalogs

Bun uses nested catalog configuration without falling back to ignored root entries.

## `vpt cp nested-catalogs.json package.json`


## `vp install`

The ignored root catalog does not prevent installation before creation.


## `vp create vite:application --directory apps/web --no-interactive --no-agent --no-editor --no-hooks`


## `vpt stat-file bun.lock --assert file`

```
bun.lock: file
```

## `vpt print-file package.json`

```
{
  "name": "plain-workspace",
  "private": true,
  "packageManager": "bun@1.4.0",
  "workspaces": {
    "packages": [
      "apps/*"
    ],
    "catalog": {
      "is-number": "7.0.0",
      "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
      "vite-plus": "<version>"
    }
  },
  "catalog": {
    "vite": "0.0.0-unused",
    "vite-plus": "<version>"
  }
}
```

## `vp install --frozen-lockfile`


## `cd apps/web && vp run build`

