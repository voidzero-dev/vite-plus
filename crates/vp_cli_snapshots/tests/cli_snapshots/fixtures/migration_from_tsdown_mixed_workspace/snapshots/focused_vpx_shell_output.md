# focused_vpx_shell_output

A filtered member install lets vpx write JSON through redirection and a pipeline.

## `vpt json-edit package.json devDependencies '{"vite":"8.3.8"}'`


## `vpt write-file pnpm-workspace.yaml 'packages: ['\''packages/*'\'']
sharedWorkspaceLockfile: false
shellEmulator: true
'`


## `vpt mkdir -p packages/generator/src`


## `vpt write-file packages/generator/package.json '{"name":"generator","private":true,"type":"module","scripts":{"redirect":"vpx ./generate.ts > generated.json","pipeline":"vpx ./generate.ts | node write-pipe.cjs","release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.14"}}'`


## `vpt write-file packages/generator/generate.ts 'const value: string = '\''release config'\''; console.log(JSON.stringify({ value }));'`


## `vpt write-file packages/generator/write-pipe.cjs 'const fs = require('\''node:fs'\''); fs.writeFileSync('\''generated-pipe.json'\'', fs.readFileSync(0));'`


## `vpt write-file packages/generator/src/index.ts 'export const sample = true;'`


## `vpt write-file packages/generator/release.config.mts 'export default { entry: '\''src/index.ts'\'', outDir: '\''release-out'\'', dts: false };'`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file packages/generator/package.json`

```
{"name":"generator","private":true,"type":"module","scripts":{"redirect":"vpx ./generate.ts > generated.json","pipeline":"vpx ./generate.ts | node write-pipe.cjs","release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.14","vite-plus":"catalog:"}}
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

## `cd packages/generator && pnpm run redirect`


## `cd packages/generator && pnpm run pipeline`


## `vpt print-file packages/generator/generated.json`

```
{"value":"release config"}
```

## `vpt print-file packages/generator/generated-pipe.json`

```
{"value":"release config"}
```

## `cd packages/generator && pnpm run release`


## `vpt stat-file packages/generator/release-out/index.mjs --assert file`

```
packages/generator/release-out/index.mjs: file
```
