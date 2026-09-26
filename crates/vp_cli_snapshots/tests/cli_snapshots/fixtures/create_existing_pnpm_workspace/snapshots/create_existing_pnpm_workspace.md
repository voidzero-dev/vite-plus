# create_existing_pnpm_workspace

Adding an application to a plain pnpm workspace supplies the catalog entries needed for installation.

## `vp create vite:application --directory apps/web --package-manager pnpm --no-interactive --no-agent --no-editor --no-hooks`


## `vpt stat-file pnpm-lock.yaml --assert file`

```
pnpm-lock.yaml: file
```

## `vpt print-file pnpm-workspace.yaml`

```
packages:
  - apps/*
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
```

## `vp install --frozen-lockfile`


## `cd apps/web && vp run build`

