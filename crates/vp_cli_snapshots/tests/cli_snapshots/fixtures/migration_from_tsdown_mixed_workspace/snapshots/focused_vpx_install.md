# focused_vpx_install

A filtered member install provides the migrated vpx command and keeps the tsdown release.

## `vpt json-edit package.json devDependencies '{"vite":"8.3.8"}'`


## `vpt write-file pnpm-workspace.yaml 'packages: ['\''packages/*'\'']
sharedWorkspaceLockfile: false
'`


## `vpt mkdir -p packages/generator/src`


## `vpt write-file packages/generator/package.json '{"name":"generator","private":true,"type":"module","scripts":{"generate":"tsx generate.ts","release":"tsdown -c release.config.mts"},"devDependencies":{"tsx":"4.21.0","tsdown":"0.22.14"}}'`


## `vpt write-file packages/generator/generate.ts 'const name: string = '\''focused-vpx-marker'\''; console.log(name);'`


## `vpt write-file packages/generator/release.config.mts 'export default { entry: '\''src/index.ts'\'', outDir: '\''release-out'\'', dts: false };'`


## `vpt write-file packages/generator/src/index.ts 'export const sample = true;'`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file packages/generator/package.json`

```
{"name":"generator","private":true,"type":"module","scripts":{"generate":"vpx generate.ts","release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.14","vite-plus":"catalog:"}}
```

## `pnpm --filter generator install --ignore-scripts`


## `vpt stat-file node_modules/vite-plus --assert missing`

```
node_modules/vite-plus: missing
```

## `cd packages/generator && pnpm run generate`

```
$ vpx generate.ts
focused-vpx-marker
```

## `cd packages/generator && pnpm run release`


## `vpt stat-file packages/generator/release-out/index.mjs --assert file`

```
packages/generator/release-out/index.mjs: file
```
