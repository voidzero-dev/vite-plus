# create_existing_bun_aligned_vitest_catalog

Creating a member completes aligned Vitest entries in the named toolchain catalog without changing the default catalog.

## `vpt cp vitest-template.config.ts vite.config.ts`


## `vpt json-edit package.json catalog.@vitest/utils ^5.0.0`


## `vpt json-edit package.json catalogs.tools.vite-plus *`


## `vp install`


## `vp create vitest-app --no-interactive --no-agent --no-editor --no-hooks`


## `vpt print-file apps/web/package.json`

```
{
  "name": "web",
  "private": true,
  "devDependencies": {
    "@vitest/utils": "catalog:tools",
    "vite-plus": "catalog:tools"
  }
}
```

## `vpt print-file package.json`

```
{
  "catalog": {
    "@vitest/utils": "^5.0.0"
  },
  "catalogs": {
    "tools": {
      "vite-plus": "*",
      "@vitest/utils": "<version>"
    }
  },
  "name": "plain-workspace",
  "packageManager": "bun@1.4.0",
  "private": true,
  "workspaces": [
    "apps/*"
  ]
}
```

## `vp install --frozen-lockfile`

