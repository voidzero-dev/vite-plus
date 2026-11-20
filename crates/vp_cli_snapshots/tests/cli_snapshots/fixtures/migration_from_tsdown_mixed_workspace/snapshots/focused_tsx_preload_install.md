# focused_tsx_preload_install

The member installs the new Node.js preloader and can still run its tsdown release.

## `vpt json-edit package.json devDependencies '{"vite":"8.3.8"}'`


## `vpt write-file pnpm-workspace.yaml 'packages: ['\''packages/*'\'']
sharedWorkspaceLockfile: false
'`


## `vpt mkdir -p packages/register-member/src`


## `vpt write-file packages/register-member/package.json '{"name":"register-member","private":true,"type":"module","scripts":{"check":"node --import tsx ./check.ts","release":"tsdown -c release.config.mts"},"devDependencies":{"tsx":"4.20.6","tsdown":"0.22.14"}}'`


## `vpt write-file packages/register-member/check.ts 'const value: number = 42; console.log('\''check-value-'\'' + value);'`


## `vpt write-file packages/register-member/release.config.mts 'export default { entry: '\''src/index.ts'\'', outDir: '\''release-out'\'', dts: false };'`


## `vpt write-file packages/register-member/src/index.ts 'export const sample = true;'`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file packages/register-member/package.json`

```
{"name":"register-member","private":true,"type":"module","scripts":{"check":"node --import vite-plus/register ./check.ts","release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.14","vite-plus":"catalog:"}}
```

## `pnpm --filter register-member install --ignore-scripts`


## `vpt stat-file node_modules/vite-plus --assert missing`

```
node_modules/vite-plus: missing
```

## `cd packages/register-member && pnpm run check`

```
$ node --import vite-plus/register ./check.ts
check-value-42
```

## `cd packages/register-member && vp run release`


## `vpt stat-file packages/register-member/release-out/index.mjs --assert file`

```
packages/register-member/release-out/index.mjs: file
```
