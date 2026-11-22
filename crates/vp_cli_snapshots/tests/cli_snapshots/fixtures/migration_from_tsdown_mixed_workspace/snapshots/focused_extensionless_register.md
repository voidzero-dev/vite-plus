# focused_extensionless_register

An extensionless script finds the member's migrated Node.js preload.

## `vpt json-edit package.json devDependencies '{"vite":"8.3.8"}'`


## `vpt write-file pnpm-workspace.yaml 'packages: ['\''packages/*'\'']
sharedWorkspaceLockfile: false
'`


## `vpt mkdir -p packages/generator/src`


## `vpt write-file packages/generator/package.json '{"name":"generator","private":true,"type":"module","scripts":{"release":"tsdown -c release.config.mts","generate":"./bin/generate"},"devDependencies":{"tsdown":"0.22.14","@oxc-node/core":"1.0.2"}}'`


## `vpt write-file packages/generator/src/index.ts 'export const release = true;'`


## `vpt write-file packages/generator/release.config.mts 'export default { entry: '\''src/index.ts'\'', outDir: '\''release-out'\'', dts: false };'`


## `vpt mkdir -p packages/generator/bin`


## `vpt write-file packages/generator/bin/generate '#'\!'/usr/bin/env -S node --import @oxc-node/core/register
console.log('\''generated'\'');
'`


## `vpt chmod +x packages/generator/bin/generate`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file packages/generator/package.json`

```
{"name":"generator","private":true,"type":"module","scripts":{"release":"tsdown -c release.config.mts","generate":"./bin/generate"},"devDependencies":{"tsdown":"0.22.14","vite-plus":"catalog:"}}
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

## `vpt print-file packages/generator/bin/generate`

```
#!/usr/bin/env -S node --import vite-plus/register
console.log('generated');
```

## `cd packages/generator && pnpm run generate`

```
$ ./bin/generate
generated
```

## `cd packages/generator && pnpm run release`


## `vpt stat-file packages/generator/release-out/index.mjs --assert file`

```
packages/generator/release-out/index.mjs: file
```
