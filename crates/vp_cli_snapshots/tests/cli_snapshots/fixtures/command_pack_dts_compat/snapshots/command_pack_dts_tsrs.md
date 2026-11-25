# command_pack_dts_tsrs

Without typescript, the packed CLI generates inferred declarations with the bundled tsrs. The experimental tsrs notice does not fail --fail-on-warn. pnpm hoist:false keeps undeclared compilers out of reach.

## `vpt json-edit package.json devDependencies '{"vite-plus":"latest"}'`


## `vp install --ignore-scripts`


## `vp pack --fail-on-warn`

```
VITE+ - The Unified Toolchain for the Web

ℹ entry: src/index.ts
ℹ tsconfig: tsconfig.json
ℹ Generating declarations with bundled tsrs <version> (experimental) because `typescript` is not installed
ℹ Build start
ℹ Emit types with tsrs
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
