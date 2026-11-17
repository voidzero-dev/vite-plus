# create_existing_bun_workspace

Creating an application supplies missing Bun catalog entries before installation.

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
  "workspaces": [
    "apps/*"
  ],
  "catalog": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vite-plus": "<version>"
  }
}
```

## `vp install --frozen-lockfile`


## `cd apps/web && vp run build`

