# focused_vp_vpr_install

A filtered install provides the member's existing vp and vpr commands and preserves its tsdown release.

## `vpt json-edit package.json devDependencies '{"vite":"8.3.8"}'`


## `vpt write-file pnpm-workspace.yaml 'packages: ['\''packages/*'\'']
sharedWorkspaceLockfile: false
'`


## `vpt mkdir -p packages/local-cli/src`


## `vpt write-file packages/local-cli/package.json '{"name":"local-cli","private":true,"type":"module","scripts":{"format":"vp fmt","check":"vpr fixture-check","fixture-check":"node check.js","release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.14"}}'`


## `vpt write-file packages/local-cli/check.js console.log('member-check');`


## `vpt write-file packages/local-cli/src/index.ts 'export const sample=true;'`


## `vpt write-file packages/local-cli/release.config.mts 'export default { entry: '\''src/index.ts'\'', outDir: '\''release-out'\'', dts: false };'`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file packages/local-cli/package.json`

```
{"name":"local-cli","private":true,"type":"module","scripts":{"format":"vp fmt","check":"vpr fixture-check","fixture-check":"node check.js","release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.14","vite-plus":"catalog:"}}
```

## `pnpm --filter local-cli install --ignore-scripts`


## `vpt stat-file node_modules/vite-plus --assert missing`

```
node_modules/vite-plus: missing
```

## `cd packages/local-cli && pnpm run format`


## `vpt print-file packages/local-cli/src/index.ts`

```
export const sample = true;
```

## `cd packages/local-cli && pnpm run check`


## `cd packages/local-cli && pnpm run release`


## `vpt stat-file packages/local-cli/release-out/index.mjs --assert file`

```
packages/local-cli/release-out/index.mjs: file
```
