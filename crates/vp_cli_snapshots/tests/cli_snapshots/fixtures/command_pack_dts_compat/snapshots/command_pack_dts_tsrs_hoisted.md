# command_pack_dts_tsrs_hoisted

With pnpm's default hoisting and no typescript, no transitive typescript reaches the declaration plugin, so the bundled tsrs generates the declarations.

## `vpt json-edit package.json devDependencies '{"vite-plus":"latest"}'`


## `vpt write-file pnpm-workspace.yaml 'minimumReleaseAge: 0'`


## `vp install --ignore-scripts`


## `vp pack`

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
