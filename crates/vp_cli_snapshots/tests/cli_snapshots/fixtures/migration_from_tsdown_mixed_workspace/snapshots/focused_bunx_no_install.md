# focused_bunx_no_install

Bun resolves the member's vp binary with downloads disabled.

## `vpt json-edit package.json devDependencies '{"vite":"8.3.8"}'`


## `vpt write-file pnpm-workspace.yaml 'packages: ['\''packages/*'\'']
sharedWorkspaceLockfile: false
'`


## `vpt mkdir -p packages/generator/src packages/generator/scripts`


## `vpt write-file packages/generator/package.json '{"name":"generator","private":true,"type":"module","scripts":{"release":"tsdown -c release.config.mts","generate":"bunx --no-install vp --version"},"devDependencies":{"tsdown":"0.22.14"}}'`


## `vpt write-file packages/generator/src/index.ts 'export const release = true;'`


## `vpt write-file packages/generator/release.config.mts 'export default { entry: '\''src/index.ts'\'', outDir: '\''release-out'\'', dts: false };'`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file packages/generator/package.json`

```
{"name":"generator","private":true,"type":"module","scripts":{"release":"tsdown -c release.config.mts","generate":"bunx --no-install vp --version"},"devDependencies":{"tsdown":"0.22.14","vite-plus":"catalog:"}}
```

## `pnpm --filter generator install --ignore-scripts`


## `vpt stat-file node_modules/vite-plus --assert missing`

```
node_modules/vite-plus: missing
```

## `vpt stat-file packages/generator/node_modules/vite-plus/package.json --assert file`

```
packages/generator/node_modules/vite-plus/package.json: file
```

## `bun --version`


## `cd packages/generator && pnpm run generate`


## `cd packages/generator && pnpm run release`


## `vpt stat-file packages/generator/release-out/index.mjs --assert file`

```
packages/generator/release-out/index.mjs: file
```
