# focused_vpx_shell_loop

A member install supplies vpx for both shell loop iterations.

## `vpt json-edit package.json devDependencies '{"vite":"8.3.8"}'`


## `vpt write-file pnpm-workspace.yaml 'packages: ['\''packages/*'\'']
sharedWorkspaceLockfile: false
'`


## `vpt mkdir -p packages/generator/src packages/generator/scripts`


## `vpt write-file packages/generator/package.json '{"name":"generator","private":true,"type":"module","scripts":{"generate":"for target in client worker; do vpx ./scripts/generate.ts \"$target\"; done","release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.14"}}'`


## `vpt write-file packages/generator/scripts/generate.ts 'const target: string = process.argv[2] ?? '\''default'\'';
console.log('\''generator-'\'' + target);
'`


## `vpt write-file packages/generator/src/index.ts 'export const sample = true;'`


## `vpt write-file packages/generator/release.config.mts 'export default { entry: '\''src/index.ts'\'', outDir: '\''release-out'\'', dts: false };'`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file packages/generator/package.json`

```
{"name":"generator","private":true,"type":"module","scripts":{"generate":"for target in client worker; do vpx ./scripts/generate.ts \"$target\"; done","release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.14","vite-plus":"catalog:"}}
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

## `cd packages/generator && pnpm run generate`

```
$ for target in client worker; do vpx ./scripts/generate.ts "$target"; done
generator-client
generator-worker
```

## `cd packages/generator && pnpm run release`


## `vpt stat-file packages/generator/release-out/index.mjs --assert file`

```
packages/generator/release-out/index.mjs: file
```
