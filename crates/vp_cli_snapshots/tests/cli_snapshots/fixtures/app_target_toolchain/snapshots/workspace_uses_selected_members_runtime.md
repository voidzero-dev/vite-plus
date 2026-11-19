# workspace_uses_selected_members_runtime

Auto-selection and the picker use the member's Node.js pin with one shared Vite+ installation.

## `vpt rm vite.config.ts`


## `vpt write-file pnpm-workspace.yaml 'packages:
  - app
  - other
'`


## `vpt touch-file app/index.html`


## `vpt write-file other/package.json '{"name":"other","private":true}'`


## `vpt mkdir -p node_modules/vite-plus/dist`


## `vpt write-file node_modules/vite-plus/package.json '{"name":"vite-plus","version":"0.0.0","type":"module"}'`


## `vpt cp probe.mjs node_modules/vite-plus/dist/bin.js`


## `vp -C app build`

```
VITE+ - The Unified Toolchain for the Web

Vite+ <version>: build in app
Process Node.js: <version>
PATH Node.js: <version>
```

## `vp build`

```
VITE+ - The Unified Toolchain for the Web

Selected package: app (app)
Tip: run this directly with `vp -C app build`
Vite+ <version>: build in app
Process Node.js: <version>
PATH Node.js: <version>
```

## `vpt touch-file other/index.html`


## `vp build`

**→ expect-milestone:** `package-select::0`

```
VITE+ - The Unified Toolchain for the Web

Select a package to build (↑/↓, Enter to run, type to search):

  › app   app
    other other
    root  .
```

**← write:** `app`

**→ expect-milestone:** `package-select:app:0`

```
VITE+ - The Unified Toolchain for the Web

Select a package to build (↑/↓, Enter to run, type to search): app

  › app app
```

**← write-key:** `enter`

```
VITE+ - The Unified Toolchain for the Web

Selected package: app (app)
Tip: run this directly with `vp -C app build`
Vite+ <version>: build in app
Process Node.js: <version>
PATH Node.js: <version>
```
