# command_pack_dts_typescript_7

The packed CLI generates inferred declarations with stable TypeScript 7.0.2 and automatic generator selection.

## `vp install --ignore-scripts`


## `vp pack`

```
VITE+ - The Unified Toolchain for the Web

ℹ entry: src/index.ts
ℹ tsconfig: tsconfig.json
ℹ Build start
warn: TypeScript 7.0 does not yet have a stable API and is experimental. Some options will be unavailable.
ℹ Emit types with typescript@7.0.2
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
