# command_pack_dts_native_preview_only

An explicit tsgo generator emits inferred declarations with native-preview and no TypeScript package.

## `vpt json-edit package.json devDependencies '{"@typescript/native-preview":"7.0.0-dev.20260707.2","vite-plus":"latest"}'`


## `vp install --ignore-scripts`


## `node assert-no-typescript.mjs`


## `vp pack`

```
VITE+ - The Unified Toolchain for the Web

ℹ entry: src/index.ts
ℹ tsconfig: tsconfig.json
ℹ Build start
warn: The `tsgo` generator on TypeScript 7.0 is deprecated and will be removed in the next minor. Upgrade to TypeScript 7.1 or later.
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
