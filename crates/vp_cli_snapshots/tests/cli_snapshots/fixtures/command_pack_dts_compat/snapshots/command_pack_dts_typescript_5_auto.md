# command_pack_dts_typescript_5_auto

Automatic declaration generation uses TypeScript 5.9.3 with tsconfig discovery disabled even when native-preview is installed.

## `vpt json-edit package.json devDependencies.typescript 5.9.3`


## `vpt json-edit package.json devDependencies.@typescript/native-preview 7.0.0-dev.20260707.2`


## `vp install --ignore-scripts`


## `vp pack`

```
VITE+ - The Unified Toolchain for the Web

ℹ entry: src/index.ts
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
