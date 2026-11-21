# legacy_local_cli_selects_target

A local CLI that predates the fixed target marker receives the original directory and arguments.

## `vpt rm vite.config.ts`


## `vpt write-file pnpm-workspace.yaml 'packages:
  - app
'`


## `vpt touch-file app/index.html`


## `vpt mkdir -p node_modules/vite-plus/dist`


## `vpt write-file node_modules/vite-plus/package.json '{"name":"vite-plus","version":"0.2.9","type":"module"}'`


## `vpt cp legacy-probe.mjs node_modules/vite-plus/dist/bin.js`


## `vp build`

```
VITE+ - The Unified Toolchain for the Web

Legacy CLI in root: ["build"]
```

## `vp build --`

```
VITE+ - The Unified Toolchain for the Web

Legacy CLI in root: ["build","--"]
```
