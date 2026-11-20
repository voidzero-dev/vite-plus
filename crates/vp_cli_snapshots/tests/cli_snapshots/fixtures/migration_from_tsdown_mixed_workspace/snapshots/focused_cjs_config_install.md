# focused_cjs_config_install

A filtered install provides Vite+ for the member's CommonJS config and keeps its tsdown release.

## `vpt json-edit package.json devDependencies '{"vite":"8.3.8"}'`


## `vpt write-file pnpm-workspace.yaml 'packages: ['\''packages/*'\'']
sharedWorkspaceLockfile: false
'`


## `vpt mkdir -p packages/cjs-member/src`


## `vpt write-file packages/cjs-member/package.json '{"name":"cjs-member","private":true,"type":"commonjs","scripts":{"release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.14"}}'`


## `vpt write-file packages/cjs-member/vite.config.cjs 'const { defineConfig } = require('\''vite'\''); module.exports = defineConfig({});'`


## `vpt write-file packages/cjs-member/release.config.mts 'export default { entry: '\''src/index.ts'\'', outDir: '\''release-out'\'', dts: false };'`


## `vpt write-file packages/cjs-member/src/index.ts 'export const sample = true;'`


## `vpt write-file packages/cjs-member/index.html '<main>Member config</main>'`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `pnpm --filter cjs-member install --ignore-scripts`


## `vpt stat-file node_modules/vite-plus --assert missing`

```
node_modules/vite-plus: missing
```

## `cd packages/cjs-member && node -e 'console.log(typeof require('\''./vite.config.cjs'\''))'`

```
object
```

## `cd packages/cjs-member && vp build`


## `vpt print-file packages/cjs-member/dist/index.html`

```
<main>Member config</main>
```

## `cd packages/cjs-member && vp run release`


## `vpt stat-file packages/cjs-member/release-out/index.mjs --assert file`

```
packages/cjs-member/release-out/index.mjs: file
```
