# root_pack_shared_tsdown

The workspace root packs its resources while the member still publishes with the root's tsdown dependency.

## `vpt json-edit package.json dependencies '{"tsdown":"0.22.14"}'`


## `vpt json-edit package.json devDependencies '{"vite":"8.3.8"}'`


## `vpt json-edit package.json scripts '{"build":"tsdown"}'`


## `vpt json-edit packages/legacy-pack/package.json scripts '{"release":"tsdown -c release.config.mts"}'`


## `vpt write-file packages/legacy-pack/release.config.mts 'export default { entry: '\''src/library.ts'\'', outDir: '\''release-out'\'', dts: false };'`


## `vpt write-file tsdown.config.ts 'export default { entry: '\''src/root-library.ts'\'', outDir: '\''root-out'\'', publicDir: '\''public'\'' };'`


## `vpt write-file vite.config.ts 'import rootConfig from '\''./tsdown.config.ts'\''; export default { pack: rootConfig };'`


## `vpt mkdir -p public`


## `vpt write-file public/root.txt 'root pack asset
'`


## `vp install --ignore-scripts`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file tsdown.config.ts`

```
export default { entry: "src/root-library.ts", outDir: "root-out", publicDir: "public" };
```

## `pnpm run build`


## `vpt stat-file root-out/root-library.mjs --assert file`

```
root-out/root-library.mjs: file
```

## `vpt print-file root-out/public/root.txt`

```
root pack asset
```

## `cd packages/legacy-pack && pnpm run release`


## `vpt stat-file packages/legacy-pack/release-out/library.mjs --assert file`

```
packages/legacy-pack/release-out/library.mjs: file
```
