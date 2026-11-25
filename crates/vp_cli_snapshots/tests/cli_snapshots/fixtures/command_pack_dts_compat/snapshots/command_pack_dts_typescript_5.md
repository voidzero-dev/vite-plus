# command_pack_dts_typescript_5

The packed CLI generates inferred declarations with TypeScript 5.9.3 and automatic generator selection.

## `vpt json-edit package.json devDependencies.typescript 5.9.3`


## `vp install --ignore-scripts`


## `vp pack`

```
VITE+ - The Unified Toolchain for the Web

ℹ entry: src/index.ts
ℹ tsconfig: tsconfig.json
ℹ Build start
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
