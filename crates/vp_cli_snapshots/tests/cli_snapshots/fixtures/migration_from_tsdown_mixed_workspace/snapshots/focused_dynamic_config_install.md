# focused_dynamic_config_install

The native loader resolves the configured dynamic import from a filtered member install.

## `vpt json-edit package.json devDependencies '{"vite":"8.3.8"}'`


## `vpt write-file pnpm-workspace.yaml 'packages: ['\''packages/*'\'']
sharedWorkspaceLockfile: false
'`


## `vpt mkdir -p packages/dynamic-member/src`


## `vpt write-file packages/dynamic-member/package.json '{"name":"dynamic-member","private":true,"type":"module","scripts":{"release":"tsdown -c release.config.mts"},"devDependencies":{"tsdown":"0.22.14"}}'`


## `vpt write-file packages/dynamic-member/vite.config.ts 'export default async () => { const { defineConfig } = await import('\''vite'\''); return defineConfig({}); };'`


## `vpt write-file packages/dynamic-member/release.config.mts 'export default { entry: '\''src/index.ts'\'', outDir: '\''release-out'\'', dts: false };'`


## `vpt write-file packages/dynamic-member/src/index.ts 'export const sample = true;'`


## `vpt write-file packages/dynamic-member/index.html '<main>Dynamic config</main>'`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `pnpm --filter dynamic-member install --ignore-scripts`


## `vpt stat-file node_modules/vite-plus --assert missing`

```
node_modules/vite-plus: missing
```

## `cd packages/dynamic-member && vp build --configLoader native`


## `vpt print-file packages/dynamic-member/dist/index.html`

```
<main>Dynamic config</main>
```

## `cd packages/dynamic-member && vp run release`


## `vpt stat-file packages/dynamic-member/release-out/index.mjs --assert file`

```
packages/dynamic-member/release-out/index.mjs: file
```
