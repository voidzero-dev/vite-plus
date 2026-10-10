# vpx_script_ts_dependency

TypeScript published in node_modules is transformed, which Node.js type stripping refuses.

## `vpt mkdir -p node_modules/ts-dep`


## `vpt write-file node_modules/ts-dep/package.json '{ "name": "ts-dep", "type": "module", "exports": "./index.ts" }
'`


## `vpt write-file node_modules/ts-dep/index.ts 'export enum Level { High = '\''high'\'' }
export const level: Level = Level.High;
'`


## `vpx ./src/ts-dependency.ts`

```
ts-dep: high
```
