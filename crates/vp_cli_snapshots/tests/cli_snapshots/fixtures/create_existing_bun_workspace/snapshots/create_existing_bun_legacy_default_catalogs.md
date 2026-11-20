# create_existing_bun_legacy_default_catalogs

Bun 1.3.14 still needs shorthand references completed in catalog rather than catalogs.default.

## `vpt json-edit package.json packageManager bun@1.3.14`


## `vpt json-edit package.json workspaces '{"packages":["apps/*"],"catalog":{"vite":"npm:@voidzero-dev/vite-plus-core@latest"},"catalogs":{"default":{"react":"19.2.0"}}}'`


## `vpt json-edit package.json devDependencies '{"vite":"catalog:","react":"catalog:default"}'`


## `vp install`


## `vp create vite:application --directory apps/web --no-interactive --no-agent --no-editor --no-hooks`


## `vpt print-file package.json`

```
{
  "devDependencies": {
    "react": "catalog:default",
    "vite": "catalog:"
  },
  "name": "plain-workspace",
  "packageManager": "bun@1.3.14",
  "private": true,
  "workspaces": {
    "catalog": {
      "vite": "npm:@voidzero-dev/vite-plus-core@latest",
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
  }
}
```

## `vpt print-file apps/web/package.json`

```
{
  "name": "web",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vp dev",
    "build": "tsc && vp build",
    "preview": "vp preview"
  },
  "devDependencies": {
    "typescript": "~7.0.2",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  }
}
```

## `vp install --frozen-lockfile`


## `cd apps/web && vp run build`

