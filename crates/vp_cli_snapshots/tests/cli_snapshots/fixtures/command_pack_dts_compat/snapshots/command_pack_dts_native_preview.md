# command_pack_dts_native_preview

An explicit tsgo generator uses native-preview with TypeScript 5.9.3. pnpm hoist:false also verifies core declares the optional native-preview peer.

## `vpt json-edit package.json devDependencies.typescript 5.9.3`


## `vpt json-edit package.json devDependencies.@typescript/native-preview 7.0.0-dev.20260707.2`


## `vp install --ignore-scripts`


## `vp pack`

```
VITE+ - The Unified Toolchain for the Web

ℹ entry: src/index.ts
ℹ tsconfig: tsconfig.json
ℹ Build start
warn: TypeScript 7.0 does not yet have a stable API and is experimental. Some options will be unavailable.
ℹ Emit types with @typescript/native-preview@7.0.0-dev.20260707.2
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
