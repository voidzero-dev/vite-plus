# focused_vpx_shell_conditional

A member install provides vpx for the selected shell branch.

## `vpt json-edit package.json devDependencies '{"vite":"8.3.8"}'`


## `vpt write-file pnpm-workspace.yaml 'packages: ['\''packages/*'\'']
sharedWorkspaceLockfile: false
'`


## `vpt mkdir -p packages/generator/src`


## `vpt write-file packages/generator/package.json '{"name":"generator","private":true,"type":"module","scripts":{"generate":"if [ -n \"$CI\" ]; then vpx ./generate.ts; fi","release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.14"}}'`


## `vpt write-file packages/generator/generate.ts console.log('generated');`


## `vpt write-file packages/generator/src/index.ts 'export const release = true;'`


## `vpt write-file packages/generator/release.config.mts 'export default { entry: '\''src/index.ts'\'', outDir: '\''release-out'\'', dts: false };'`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file packages/generator/package.json`

```
{"name":"generator","private":true,"type":"module","scripts":{"generate":"if [ -n \"$CI\" ]; then vpx ./generate.ts; fi","release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.14","vite-plus":"catalog:"}}
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

## `cd packages/generator && CI=true pnpm run generate`

```
$ if [ -n "$CI" ]; then vpx ./generate.ts; fi
generated
```

## `cd packages/generator && pnpm run release`


## `vpt stat-file packages/generator/release-out/index.mjs --assert file`

```
packages/generator/release-out/index.mjs: file
```
