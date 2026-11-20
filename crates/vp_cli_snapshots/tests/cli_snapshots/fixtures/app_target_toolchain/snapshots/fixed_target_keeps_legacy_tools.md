# fixed_target_keeps_legacy_tools

An explicit target keeps the installed legacy toolchain and forwards the original arguments.

## `vpt mkdir -p app/node_modules/vite-plus/dist`


## `vpt write-file app/node_modules/vite-plus/package.json '{"name":"vite-plus","version":"0.2.7","type":"module"}'`


## `vpt cp legacy-probe.mjs app/node_modules/vite-plus/dist/bin.js`


## `vpt cp fixed-target-config.mjs app/vite.config.mjs`


## `vp -C app build`

```
VITE+ - The Unified Toolchain for the Web

Legacy CLI in app: ["build"]
```

## `vpt write-file app/node_modules/vite-plus/package.json '{"name":"vite-plus","version":"0.2.9","type":"module"}'`


## `vpt write-file app/pnpm-workspace.yaml 'linkWorkspacePackages: false
'`


## `vpt write-file app/vite.config.mjs 'export default { root: '\''site'\'' };
'`


## `vp -C app build`

```
VITE+ - The Unified Toolchain for the Web

Legacy CLI in app: ["build"]
```

## `vpt mkdir -p app/node_modules/@voidzero-dev/vite-plus-core/dist`


## `vpt write-file app/node_modules/@voidzero-dev/vite-plus-core/package.json '{"name":"@voidzero-dev/vite-plus-core","version":"0.3.0","type":"module","main":"dist/cli.js"}'`


## `vpt write-file app/node_modules/@voidzero-dev/vite-plus-core/dist/cli.js 'throw new Error('\''The hoisted Core has the wrong version'\'');
'`


## `vpt mkdir -p app/node_modules/vite-plus/node_modules/@voidzero-dev/vite-plus-core/dist`


## `vpt write-file app/node_modules/vite-plus/node_modules/@voidzero-dev/vite-plus-core/package.json '{"name":"@voidzero-dev/vite-plus-core","version":"0.2.9","type":"module","main":"dist/cli.js"}'`


## `vpt cp probe.mjs app/node_modules/vite-plus/node_modules/@voidzero-dev/vite-plus-core/dist/cli.js`


## `vpt cp probe.mjs app/node_modules/vite-plus/dist/pack-bin.js`


## `vpt write-file app/vite.config.mjs 'export default { defaultPackage: '\''./nested'\'' };
'`


## `vp -C app build`

```
VITE+ - The Unified Toolchain for the Web

Vite+ <version>: build in app
Process Node.js: <version>
PATH Node.js: <version>
```

## `vp -C app build --mode staging`

```
VITE+ - The Unified Toolchain for the Web

Vite+ <version>: build --mode staging in app
Process Node.js: <version>
PATH Node.js: <version>
```

## `vp -C app build --config vite.config.mjs`

```
VITE+ - The Unified Toolchain for the Web

Vite+ <version>: build --config vite.config.mjs in app
Process Node.js: <version>
PATH Node.js: <version>
```

## `vp -C app build --help`

```
Legacy CLI in app: ["build","--help"]
```

## `vp -C app build -- --help`

```
VITE+ - The Unified Toolchain for the Web

Vite+ <version>: build -- --help in app
Process Node.js: <version>
PATH Node.js: <version>
```

## `vp -C app pack --out-dir lib-out`

```
VITE+ - The Unified Toolchain for the Web

Vite+ <version>: --out-dir lib-out in app
Process Node.js: <version>
PATH Node.js: <version>
```
