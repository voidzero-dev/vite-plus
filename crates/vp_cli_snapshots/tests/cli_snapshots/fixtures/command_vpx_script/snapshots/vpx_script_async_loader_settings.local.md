# vpx_script_async_loader_settings

Off-thread ESM hooks receive the CommonJS preload's tsconfig selection and TypeScript dependency policy.

## `vpx --tsconfig tsconfig.alt.json src/resolution.ts`

The loader thread honors --tsconfig instead of discovering the default config

```
hello vpx from alt
4 6 42 cts counter=1
```

## `TS_NODE_PROJECT=missing.json vpx ./src/print-setup.ts`

A missing environment-selected config keeps its TS_NODE_PROJECT diagnostic label

```
oxc-node: warning: the tsconfig TS_NODE_PROJECT names, <workspace>/missing.json, does not exist; running without a tsconfig
no preload
```

## `vpt mkdir -p node_modules/ts-dep`


## `vpt write-file node_modules/ts-dep/package.json '{ "name": "ts-dep", "type": "module", "exports": "./index.ts" }
'`


## `vpt write-file node_modules/ts-dep/index.ts 'export enum Level { High = '\''high'\'' }
export const level: Level = Level.High;
'`


## `vpx ./src/ts-dependency.ts`

The loader thread transforms published TypeScript dependencies

```
ts-dep: high
```
