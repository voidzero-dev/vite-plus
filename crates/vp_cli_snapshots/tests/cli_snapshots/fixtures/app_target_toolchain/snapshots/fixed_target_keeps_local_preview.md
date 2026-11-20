# fixed_target_keeps_local_preview

A local preview that supports the fixed-target marker runs both explicit and selected targets.

## `vpt mkdir -p app/node_modules/vite-plus/dist`


## `vpt write-file app/node_modules/vite-plus/package.json '{"name":"vite-plus","version":"0.0.0-commit.preview","type":"module"}'`


## `vpt cp probe.mjs app/node_modules/vite-plus/dist/bin.js`


## `vpt cp fixed-target-config.mjs app/vite.config.mjs`


## `vpt write-file app/index.html '<h1>Selected app</h1>
'`


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

note: vp build: using ./app (defaultPackage in vite.config.ts)
Vite+ <version>: build in app
Process Node.js: <version>
PATH Node.js: <version>
```

## `vpt cp legacy-probe.mjs app/node_modules/vite-plus/dist/bin.js`


## `VP_GLOBAL_CLI_JS_SCRIPTS_DIR=${workspace}/global-cli vp -C app build`

```
VITE+ - The Unified Toolchain for the Web

Global CLI in app: ["build"]
```
