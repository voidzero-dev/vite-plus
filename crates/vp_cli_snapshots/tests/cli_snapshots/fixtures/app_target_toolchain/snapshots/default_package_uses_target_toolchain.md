# default_package_uses_target_toolchain

The implicit target uses the same local CLI, working directory, and Node.js runtime as an explicit -C.

## `vpt mkdir -p app/node_modules/vite-plus/dist`


## `vpt write-file app/node_modules/vite-plus/package.json '{"name":"vite-plus","version":"0.0.0","type":"module"}'`


## `vpt cp probe.mjs app/node_modules/vite-plus/dist/bin.js`


## `vp build`

```
VITE+ - The Unified Toolchain for the Web

note: vp build: using ./app (defaultPackage in vite.config.ts)
Vite+ <version>: build in app
Process Node.js: <version>
PATH Node.js: <version>
```

## `vp -C app build`

```
VITE+ - The Unified Toolchain for the Web

Vite+ <version>: build in app
Process Node.js: <version>
PATH Node.js: <version>
```
