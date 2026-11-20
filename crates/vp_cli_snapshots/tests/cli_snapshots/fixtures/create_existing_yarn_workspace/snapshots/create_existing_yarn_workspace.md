# create_existing_yarn_workspace

Creating an application supplies missing Yarn catalog entries before installation.

## `vp create vite:application --directory apps/web --no-interactive --no-agent --no-editor --no-hooks`


## `vpt stat-file yarn.lock --assert file`

```
yarn.lock: file
```

## `vpt print-file .yarnrc.yml`

```
nodeLinker: node-modules
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
```

## `vp install --frozen-lockfile`


## `cd apps/web && vp run build`

