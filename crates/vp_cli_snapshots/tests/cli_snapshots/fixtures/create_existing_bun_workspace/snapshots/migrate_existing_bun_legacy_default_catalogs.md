# migrate_existing_bun_legacy_default_catalogs

Bun 1.2.14 keeps catalog and catalogs.default separate through migration and a second bootstrap pass.

## `vpt json-edit package.json packageManager bun@1.2.14`


## `vpt json-edit package.json workspaces '{"packages":["apps/*"],"catalog":{"vite":"npm:@voidzero-dev/vite-plus-core@*"},"catalogs":{"default":{"react":"19.2.0"}}}'`


## `vpt json-edit package.json devDependencies '{"vite":"catalog:","react":"catalog:default"}'`


## `vp install`


## `vp migrate --no-interactive`


## `vpt print-file package.json`

```
{
  "name": "plain-workspace",
  "private": true,
  "workspaces": {
    "catalog": {
      "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
      "vite-plus": "<version>"
    },
    "catalogs": {
      "default": {
        "react": "19.2.0"
      }
    },
    "packages": [
      "apps/*"
    ]
  },
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
  "packageManager": "bun@1.2.14"
}
```

## `vp install --frozen-lockfile`


## `vp migrate --no-interactive`


## `vp install --frozen-lockfile`

