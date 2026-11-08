# command_pack_dts_tsgo_beta

An explicit tsgo generator uses the TypeScript 7.1 beta API with pnpm hoist:false.

## `vpt json-edit package.json devDependencies.typescript 7.1.0-beta`


## `vp install --ignore-scripts`


## `vp pack`

```
VITE+ - The Unified Toolchain for the Web

ℹ entry: src/index.ts
ℹ tsconfig: tsconfig.json
ℹ Build start
warn: TypeScript Go is experimental. Some options will be unavailable.
ℹ Emit types with TypeScript Go
ℹ dist/index.mjs    <size> kB │ gzip: <size> kB
ℹ dist/index.d.mts  <size> kB │ gzip: <size> kB
ℹ 2 files, total: <size> kB
✔ Build complete in <duration>
```

## `vpt print-file dist/index.d.mts`

```
export declare function greet(name: string): {
  message: string;
  length: number;
};
```
