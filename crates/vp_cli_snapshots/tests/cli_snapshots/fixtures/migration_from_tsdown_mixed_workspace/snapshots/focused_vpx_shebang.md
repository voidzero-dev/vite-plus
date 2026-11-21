# focused_vpx_shebang

A filtered member install supplies vpx for the migrated shebang.

## `vpt json-edit package.json devDependencies '{"vite":"8.3.8"}'`


## `vpt write-file pnpm-workspace.yaml 'packages: ['\''packages/*'\'']
sharedWorkspaceLockfile: false
'`


## `vpt mkdir -p packages/generator/src`


## `vpt write-file packages/generator/package.json '{"name":"generator","private":true,"type":"module","scripts":{"generate":"./generate.ts","release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.14","tsx":"4.20.3"}}'`


## `vpt write-file packages/generator/generate.ts '#'\!'/usr/bin/env tsx
const value: string = '\''release config'\''; console.log(JSON.stringify({ value }));'`


## `vpt chmod +x packages/generator/generate.ts`


## `vpt write-file packages/generator/src/index.ts 'export const sample = true;'`


## `vpt write-file packages/generator/release.config.mts 'export default { entry: '\''src/index.ts'\'', outDir: '\''release-out'\'', dts: false };'`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file packages/generator/package.json`

```
{"name":"generator","private":true,"type":"module","scripts":{"generate":"./generate.ts","release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.14","vite-plus":"catalog:"}}
```

## `vpt print-file packages/generator/generate.ts`

```
#!/usr/bin/env vpx
const value: string = 'release config'; console.log(JSON.stringify({ value }));
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
$ ./generate.ts
{"value":"release config"}
```

## `cd packages/generator && pnpm run release`


## `vpt stat-file packages/generator/release-out/index.mjs --assert file`

```
packages/generator/release-out/index.mjs: file
```
